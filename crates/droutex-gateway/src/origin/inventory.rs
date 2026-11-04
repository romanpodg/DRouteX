//! Fresh unprivileged host-address checks, adapted from the D53 DNS inventory.
//! A successful probe is not a connection permit; caller must recheck the
//! original authorization and actually connect only the numeric target checked.
use super::registry::Registry;
use std::{
    collections::BTreeSet,
    error::Error,
    fmt,
    future::Future,
    io::Read,
    net::IpAddr,
    process::{Child, Command, Stdio},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::sync::{Semaphore, oneshot};

const MAX_OUTPUT_BYTES: u64 = 65_536;
const MAX_PROBES: usize = 8;
static PROBE_SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventoryError {
    Configuration,
    Unavailable,
    SelfAddress,
    Timeout,
    Busy,
    State,
}
impl fmt::Display for InventoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "origin inventory {self:?}")
    }
}
impl Error for InventoryError {}
type Result<T> = std::result::Result<T, InventoryError>;

/// Trusted host-supplied aliases must include public/NAT and configured listener
/// addresses invisible to the OS. The live OS inventory is obtained per probe.
pub struct OriginInventory {
    configured: Arc<BTreeSet<IpAddr>>,
    slots: Arc<Semaphore>,
    registry: Arc<Registry>,
}
impl OriginInventory {
    pub fn new(aliases: &[IpAddr], listeners: impl IntoIterator<Item = IpAddr>) -> Result<Self> {
        Self::with_registry(aliases, listeners, Arc::new(Registry::new()))
    }
    pub(super) fn with_registry(
        aliases: &[IpAddr],
        listeners: impl IntoIterator<Item = IpAddr>,
        registry: Arc<Registry>,
    ) -> Result<Self> {
        if aliases.len() > super::MAX_SELF_ADDRESSES {
            return Err(InventoryError::Configuration);
        }
        let mut configured = BTreeSet::new();
        for address in aliases.iter().copied() {
            insert(&mut configured, address)?;
        }
        let mut remaining = super::MAX_SELF_ADDRESSES - aliases.len();
        for address in listeners {
            remaining = remaining
                .checked_sub(1)
                .ok_or(InventoryError::Configuration)?;
            insert(&mut configured, address)?;
        }
        Ok(Self {
            configured: Arc::new(configured),
            slots: Arc::clone(PROBE_SLOTS.get_or_init(|| Arc::new(Semaphore::new(MAX_PROBES)))),
            registry,
        })
    }
    /// Stop new probes and join all workers owned by this inventory.
    pub async fn shutdown(&self) -> Result<()> {
        self.registry.shutdown().await
    }
    /// Starts a bounded blocking probe. Poll its result and call `finish` to
    /// wait for completion. `shutdown` joins workers; dropping a probe only
    /// signals cancellation and does not establish joined cleanup.
    pub fn start(&self, target: IpAddr, deadline: tokio::time::Instant) -> Result<InventoryProbe> {
        let slot = Arc::clone(&self.slots)
            .try_acquire_owned()
            .map_err(|_| InventoryError::Busy)?;
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&cancel);
        let configured = Arc::clone(&self.configured);
        let receiver = self.registry.start(Arc::clone(&cancel), move || {
            let _slot = slot; // Owned until the actual blocking worker has exited.
            let bytes = read_native(deadline.into_std(), &signal)?;
            let mut addresses = (*configured).clone();
            decode(&bytes, &mut addresses)?;
            if signal.load(Ordering::Acquire) || Instant::now() >= deadline.into_std() {
                return Err(InventoryError::Timeout);
            }
            if addresses.contains(&super::normalize_mapped(target)) {
                return Err(InventoryError::SelfAddress);
            }
            Ok(())
        })?;
        Ok(InventoryProbe {
            receiver: Some(receiver),
            cancel,
            _registry: Arc::clone(&self.registry),
        })
    }
}

/// Signals cancellation on drop; the inventory registry owns and joins the
/// worker. This object does not authorize a checked/pinned origin connection.
pub struct InventoryProbe {
    receiver: Option<oneshot::Receiver<Result<()>>>,
    cancel: Arc<AtomicBool>,
    _registry: Arc<Registry>,
}
impl InventoryProbe {
    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Result<()>> {
        let Some(receiver) = self.receiver.as_mut() else {
            return Poll::Ready(Err(InventoryError::State));
        };
        match std::pin::Pin::new(receiver).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(result) => {
                self.receiver.take();
                Poll::Ready(result.map_err(|_| InventoryError::State)?)
            }
        }
    }
    /// Signals cancellation and waits for worker completion. The registry keeps
    /// ownership and can still join the worker if this future is dropped.
    pub async fn finish(mut self) -> Result<()> {
        self.cancel.store(true, Ordering::Release);
        if let Some(receiver) = self.receiver.as_mut() {
            let _ = receiver.await.map_err(|_| InventoryError::State)?;
            self.receiver.take();
        }
        Ok(())
    }
}
impl Drop for InventoryProbe {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}

fn insert(set: &mut BTreeSet<IpAddr>, ip: IpAddr) -> Result<()> {
    let ip = super::normalize_mapped(ip);
    if set.len() == super::MAX_SELF_ADDRESSES && !set.contains(&ip) {
        return Err(InventoryError::Configuration);
    }
    set.insert(ip);
    Ok(())
}
struct ChildOwner(Child);
impl Drop for ChildOwner {
    fn drop(&mut self) {
        if !self.0.try_wait().is_ok_and(|status| status.is_some()) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
fn read_native(deadline: Instant, cancel: &AtomicBool) -> Result<Vec<u8>> {
    if cancel.load(Ordering::Acquire) || Instant::now() >= deadline {
        return Err(InventoryError::Timeout);
    }
    let mut command = native_command()?;
    let mut child = ChildOwner(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| InventoryError::Unavailable)?,
    );
    let output = child.0.stdout.take().ok_or(InventoryError::State)?;
    std::thread::scope(|scope| {
        let reader = std::thread::Builder::new()
            .spawn_scoped(scope, move || {
                let mut bytes = Vec::new();
                output
                    .take(MAX_OUTPUT_BYTES + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| InventoryError::Unavailable)?;
                if bytes.len() as u64 > MAX_OUTPUT_BYTES {
                    return Err(InventoryError::Unavailable);
                }
                Ok(bytes)
            })
            .map_err(|_| InventoryError::Unavailable)?;
        let status = loop {
            if cancel.load(Ordering::Acquire) || Instant::now() >= deadline {
                let _ = child.0.kill();
                let _ = child.0.wait();
                break Err(InventoryError::Timeout);
            }
            match child.0.try_wait() {
                Ok(Some(status)) if status.success() => break Ok(()),
                Ok(Some(_)) => break Err(InventoryError::Unavailable),
                Err(_) => {
                    let _ = child.0.kill();
                    let _ = child.0.wait();
                    break Err(InventoryError::Unavailable);
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(2)),
            }
        };
        let bytes = reader.join().map_err(|_| InventoryError::State)?;
        status?;
        bytes
    })
}
#[cfg(target_os = "linux")]
fn native_command() -> Result<Command> {
    let path = ["/usr/sbin/ip", "/usr/bin/ip", "/sbin/ip", "/bin/ip"]
        .into_iter()
        .find(|path| std::path::Path::new(path).is_file())
        .ok_or(InventoryError::Unavailable)?;
    let mut command = Command::new(path);
    command.args(["-j", "address", "show"]);
    Ok(command)
}
#[cfg(target_os = "windows")]
fn native_command() -> Result<Command> {
    use std::os::windows::process::CommandExt;
    let root = std::env::var_os("SystemRoot").ok_or(InventoryError::Unavailable)?;
    let path =
        std::path::PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    if !path.is_absolute() {
        return Err(InventoryError::Unavailable);
    }
    let mut command = Command::new(path);
    command.creation_flags(0x0800_0000);
    command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; try { [System.Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces() | ForEach-Object { foreach ($entry in $_.GetIPProperties().UnicastAddresses) { $entry.Address.ToString() } } } catch { exit 1 }"]);
    Ok(command)
}
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn native_command() -> Result<Command> {
    Err(InventoryError::Unavailable)
}

#[cfg(any(target_os = "linux", test))]
fn linux_addresses(bytes: &[u8], addresses: &mut BTreeSet<IpAddr>) -> Result<()> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| InventoryError::Unavailable)?;
    let interfaces = value.as_array().ok_or(InventoryError::Unavailable)?;
    let mut count = 0;
    for interface in interfaces {
        let entries = interface
            .get("addr_info")
            .and_then(|v| v.as_array())
            .ok_or(InventoryError::Unavailable)?;
        for entry in entries {
            let family = entry
                .get("family")
                .and_then(|v| v.as_str())
                .ok_or(InventoryError::Unavailable)?;
            let address: IpAddr = entry
                .get("local")
                .and_then(|v| v.as_str())
                .ok_or(InventoryError::Unavailable)?
                .parse()
                .map_err(|_| InventoryError::Unavailable)?;
            if !matches!(
                (family, address),
                ("inet", IpAddr::V4(_)) | ("inet6", IpAddr::V6(_))
            ) {
                return Err(InventoryError::Unavailable);
            }
            insert(addresses, address)?;
            count += 1;
        }
    }
    if count == 0 {
        return Err(InventoryError::Unavailable);
    }
    Ok(())
}
#[cfg(any(target_os = "windows", test))]
fn windows_addresses(bytes: &[u8], addresses: &mut BTreeSet<IpAddr>) -> Result<()> {
    let text = std::str::from_utf8(bytes).map_err(|_| InventoryError::Unavailable)?;
    let mut count = 0;
    for line in text.lines() {
        let line = line.trim();
        let address = if let Some((address, scope)) = line.split_once('%') {
            if scope.is_empty() || !scope.bytes().all(|b| b.is_ascii_digit()) {
                return Err(InventoryError::Unavailable);
            }
            address
        } else {
            line
        };
        insert(
            addresses,
            address.parse().map_err(|_| InventoryError::Unavailable)?,
        )?;
        count += 1;
    }
    if count == 0 {
        return Err(InventoryError::Unavailable);
    }
    Ok(())
}
fn decode(bytes: &[u8], addresses: &mut BTreeSet<IpAddr>) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        linux_addresses(bytes, addresses)
    }
    #[cfg(target_os = "windows")]
    {
        windows_addresses(bytes, addresses)
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = (bytes, addresses);
        Err(InventoryError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::poll_fn;

    #[test]
    fn platform_decoders_refuse_missing_malformed_or_inconsistent_addresses() {
        let linux = br#"[{"addr_info":[{"family":"inet","local":"127.0.0.1"},{"family":"inet6","local":"::1"}]}]"#;
        let mut addresses = BTreeSet::new();
        linux_addresses(linux, &mut addresses).unwrap();
        assert_eq!(addresses.len(), 2);
        for invalid in [
            b"[]".as_slice(),
            b"{}",
            br#"[{"addr_info":[{"family":"inet6","local":"1.1.1.1"}]}]"#,
            br#"[{"addr_info":[{"family":"inet","local":"1.1.1.1"},{}]}]"#,
        ] {
            assert!(linux_addresses(invalid, &mut BTreeSet::new()).is_err());
        }
        let mut windows = BTreeSet::new();
        windows_addresses(
            b"127.0.0.1\r\nfe80::1%20\r\n::ffff:1.1.1.1\r\n",
            &mut windows,
        )
        .unwrap();
        assert!(windows.contains(&"1.1.1.1".parse().unwrap()));
        for invalid in [
            b"".as_slice(),
            b"not-an-address\n",
            b"fe80::1%a\n",
            b"1.1.1.1\ninvalid\n",
        ] {
            assert!(windows_addresses(invalid, &mut BTreeSet::new()).is_err());
        }
    }

    #[test]
    fn no_runtime_and_excess_config_fail_closed() {
        let aliases = vec!["1.1.1.1".parse().unwrap(); super::super::MAX_SELF_ADDRESSES + 1];
        assert!(matches!(
            OriginInventory::new(&aliases, []),
            Err(InventoryError::Configuration)
        ));
        assert!(matches!(
            OriginInventory::new(
                &[],
                std::iter::repeat_n(
                    "1.1.1.1".parse().unwrap(),
                    super::super::MAX_SELF_ADDRESSES + 1
                )
            ),
            Err(InventoryError::Configuration)
        ));
        let inventory = OriginInventory::new(&[], []).unwrap();
        assert!(matches!(
            inventory.start("1.1.1.1".parse().unwrap(), tokio::time::Instant::now()),
            Err(InventoryError::Unavailable)
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn live_inventory_rejects_node_and_aliases_with_owned_cleanup_and_global_quota() {
        let alias = "1.1.1.1".parse().unwrap();
        let mut first = OriginInventory::new(&[alias], []).unwrap();
        let mut second = OriginInventory::new(&[], []).unwrap();
        assert!(
            Arc::ptr_eq(&first.slots, &second.slots),
            "all inventories share the process limit"
        );
        // Test quota exhaustion in a shared isolated semaphore so parallel
        // resolver tests cannot consume permits or observe the artificial load.
        let fixture_slots = Arc::new(Semaphore::new(MAX_PROBES));
        first.slots = Arc::clone(&fixture_slots);
        second.slots = fixture_slots;
        let deadline = || tokio::time::Instant::now() + Duration::from_secs(10);
        for target in ["127.0.0.1".parse().unwrap(), alias] {
            let mut probe = first.start(target, deadline()).unwrap();
            let result =
                tokio::time::timeout(Duration::from_secs(12), poll_fn(|cx| probe.poll(cx)))
                    .await
                    .unwrap();
            assert_eq!(result, Err(InventoryError::SelfAddress));
            probe.finish().await.unwrap();
        }
        let mut expired = first
            .start("8.8.8.8".parse().unwrap(), tokio::time::Instant::now())
            .unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(3), poll_fn(|cx| expired.poll(cx)))
                .await
                .unwrap(),
            Err(InventoryError::Timeout)
        );
        expired.finish().await.unwrap();
        let permits = Arc::clone(&first.slots)
            .try_acquire_many_owned(MAX_PROBES as u32)
            .unwrap();
        assert!(matches!(
            second.start("8.8.8.8".parse().unwrap(), deadline()),
            Err(InventoryError::Busy)
        ));
        drop(permits);
        let probe = second
            .start("8.8.8.8".parse().unwrap(), deadline())
            .unwrap();
        probe.finish().await.unwrap();
        assert_eq!(second.slots.available_permits(), MAX_PROBES);
        let abandoned = second
            .start("8.8.4.4".parse().unwrap(), deadline())
            .unwrap();
        drop(abandoned);
        tokio::time::timeout(Duration::from_secs(3), async {
            while second.slots.available_permits() != MAX_PROBES {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("cancelled worker must release its owned slot");
        first.shutdown().await.unwrap();
        second.shutdown().await.unwrap();
        assert!(matches!(
            first.start("8.8.8.8".parse().unwrap(), deadline()),
            Err(InventoryError::Unavailable)
        ));
    }
}
