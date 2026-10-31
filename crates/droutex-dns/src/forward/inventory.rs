//! Fresh unprivileged OS inventory. No resolver, host-network mutation or user command.
use crate::{DnsError, ErrorCode, Result};
use std::future::Future;
use std::{
    collections::BTreeSet,
    io::Read,
    net::IpAddr,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::task::JoinHandle;

const MAX_ADDRESSES: usize = 256;
const MAX_OUTPUT_BYTES: u64 = 65536;
pub(super) struct Inventory {
    configured: Arc<BTreeSet<IpAddr>>,
    #[cfg(test)]
    fixture: Option<Arc<std::sync::Mutex<Result<BTreeSet<IpAddr>>>>>,
}
fn insert(set: &mut BTreeSet<IpAddr>, address: IpAddr) -> Result<()> {
    let address = match address {
        IpAddr::V6(ip) => ip.to_ipv4_mapped().map_or(address, IpAddr::V4),
        _ => address,
    };
    if set.len() == MAX_ADDRESSES && !set.contains(&address) {
        return Err(DnsError(ErrorCode::Configuration));
    }
    set.insert(address);
    Ok(())
}
impl Inventory {
    pub fn new(aliases: &[IpAddr], listeners: impl Iterator<Item = IpAddr>) -> Result<Self> {
        if aliases.len() > MAX_ADDRESSES {
            return Err(DnsError(ErrorCode::Configuration));
        }
        let mut configured = BTreeSet::new();
        for address in aliases.iter().copied().chain(listeners) {
            insert(&mut configured, address)?;
        }
        Ok(Self {
            configured: Arc::new(configured),
            #[cfg(test)]
            fixture: None,
        })
    }
    pub fn start(&self, target: IpAddr, deadline: tokio::time::Instant) -> Probe {
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&cancel);
        let configured = Arc::clone(&self.configured);
        #[cfg(test)]
        if let Some(fixture) = &self.fixture {
            let fixture = Arc::clone(fixture);
            let handle = tokio::spawn(async move {
                let snapshot = fixture.lock().map_err(|_| DnsError(ErrorCode::State))?;
                let source = snapshot.as_ref().map_err(|error| *error)?;
                let mut addresses = (*configured).clone();
                for address in source {
                    insert(&mut addresses, *address)?;
                }
                if addresses.contains(&target) {
                    return Err(DnsError(ErrorCode::Scope));
                }
                Ok(())
            });
            return Probe { handle, cancel };
        }
        let handle = tokio::task::spawn_blocking(move || {
            let bytes = read_native(deadline.into_std(), &signal)?;
            let mut addresses = (*configured).clone();
            decode(&bytes, &mut addresses)?;
            if addresses.contains(&target) {
                return Err(DnsError(ErrorCode::Scope));
            }
            Ok(())
        });
        Probe { handle, cancel }
    }
    #[cfg(test)]
    pub fn fixture(source: Arc<std::sync::Mutex<Result<BTreeSet<IpAddr>>>>) -> Self {
        Self {
            configured: Arc::new(BTreeSet::new()),
            fixture: Some(source),
        }
    }
}

/// Stored by the root-owned attempt budget, outside cancellable library futures.
pub(super) struct Probe {
    handle: JoinHandle<Result<()>>,
    cancel: Arc<AtomicBool>,
}
impl Probe {
    pub fn poll(&mut self, context: &mut Context<'_>) -> Poll<Result<()>> {
        std::pin::Pin::new(&mut self.handle)
            .poll(context)
            .map(|result| result.map_err(|_| DnsError(ErrorCode::State))?)
    }
    pub async fn finish(mut self) -> Result<()> {
        self.cancel.store(true, Ordering::Release);
        // The blocking worker owns and kills/waits its child. Aborting its async
        // waiter would not join that worker. Keep the same handle until completion.
        let _ = (&mut self.handle)
            .await
            .map_err(|_| DnsError(ErrorCode::State))?;
        Ok(())
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
    }
}

struct ChildOwner(Child);
impl Drop for ChildOwner {
    fn drop(&mut self) {
        // This guard runs only in the bounded blocking worker, never on Tokio.
        if !self.0.try_wait().is_ok_and(|status| status.is_some()) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
fn unavailable(_: impl Sized) -> DnsError {
    DnsError(ErrorCode::Unavailable)
}
fn read_native(deadline: Instant, cancel: &AtomicBool) -> Result<Vec<u8>> {
    if cancel.load(Ordering::Acquire) || Instant::now() >= deadline {
        return Err(DnsError(ErrorCode::Timeout));
    }
    let mut command = native_command()?;
    let mut child = ChildOwner(
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(unavailable)?,
    );
    let output = child.0.stdout.take().ok_or(DnsError(ErrorCode::State))?;
    std::thread::scope(|scope| {
        let reader = scope.spawn(move || {
            let mut bytes = Vec::new();
            output
                .take(MAX_OUTPUT_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(unavailable)?;
            if bytes.len() as u64 > MAX_OUTPUT_BYTES {
                return Err(DnsError(ErrorCode::Unavailable));
            }
            Ok(bytes)
        });
        let status = loop {
            if cancel.load(Ordering::Acquire) || Instant::now() >= deadline {
                let _ = child.0.kill();
                let _ = child.0.wait();
                break Err(DnsError(ErrorCode::Timeout));
            }
            match child.0.try_wait().map_err(unavailable) {
                Ok(Some(status)) => {
                    break if status.success() {
                        Ok(())
                    } else {
                        Err(DnsError(ErrorCode::Unavailable))
                    };
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(2)),
                Err(error) => {
                    let _ = child.0.kill();
                    let _ = child.0.wait();
                    break Err(error);
                }
            }
        };
        let bytes = reader.join().map_err(unavailable)?;
        status?;
        bytes
    })
}

#[cfg(target_os = "windows")]
fn native_command() -> Result<Command> {
    use std::os::windows::process::CommandExt;
    let root = std::env::var_os("SystemRoot").ok_or(DnsError(ErrorCode::Unavailable))?;
    let path =
        std::path::PathBuf::from(root).join("System32/WindowsPowerShell/v1.0/powershell.exe");
    if !path.is_absolute() {
        return Err(DnsError(ErrorCode::Unavailable));
    }
    let mut command = Command::new(path);
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    command.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
        "$ErrorActionPreference='Stop'; try { [System.Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces() | ForEach-Object { foreach ($entry in $_.GetIPProperties().UnicastAddresses) { $entry.Address.ToString() } } } catch { exit 1 }"]);
    Ok(command)
}
#[cfg(target_os = "linux")]
fn native_command() -> Result<Command> {
    let path = ["/usr/sbin/ip", "/usr/bin/ip", "/sbin/ip", "/bin/ip"]
        .into_iter()
        .find(|path| std::path::Path::new(path).is_file())
        .ok_or(DnsError(ErrorCode::Unavailable))?;
    let mut command = Command::new(path);
    command.args(["-j", "address", "show"]);
    Ok(command)
}
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn native_command() -> Result<Command> {
    Err(DnsError(ErrorCode::Unavailable))
}

#[cfg(any(target_os = "windows", test))]
fn windows_addresses(bytes: &[u8], addresses: &mut BTreeSet<IpAddr>) -> Result<()> {
    let text = std::str::from_utf8(bytes).map_err(unavailable)?;
    let mut count = 0;
    for line in text.lines() {
        let line = line.trim();
        let address = if let Some((address, scope)) = line.split_once('%') {
            if scope.is_empty() || !scope.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(DnsError(ErrorCode::Unavailable));
            }
            address
        } else {
            line
        };
        insert(addresses, address.parse().map_err(unavailable)?)?;
        count += 1;
    }
    if count == 0 {
        return Err(DnsError(ErrorCode::Unavailable));
    }
    Ok(())
}
#[cfg(any(target_os = "linux", test))]
fn linux_addresses(bytes: &[u8], addresses: &mut BTreeSet<IpAddr>) -> Result<()> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(unavailable)?;
    let interfaces = value.as_array().ok_or(DnsError(ErrorCode::Unavailable))?;
    let mut count = 0;
    for interface in interfaces {
        let entries = interface
            .get("addr_info")
            .and_then(|value| value.as_array())
            .ok_or(DnsError(ErrorCode::Unavailable))?;
        for entry in entries {
            let family = entry
                .get("family")
                .and_then(|value| value.as_str())
                .ok_or(DnsError(ErrorCode::Unavailable))?;
            let address: IpAddr = entry
                .get("local")
                .and_then(|value| value.as_str())
                .ok_or(DnsError(ErrorCode::Unavailable))?
                .parse()
                .map_err(unavailable)?;
            if !matches!(
                (family, address),
                ("inet", IpAddr::V4(_)) | ("inet6", IpAddr::V6(_))
            ) {
                return Err(DnsError(ErrorCode::Unavailable));
            }
            insert(addresses, address)?;
            count += 1;
        }
    }
    if count == 0 {
        return Err(DnsError(ErrorCode::Unavailable));
    }
    Ok(())
}
fn decode(bytes: &[u8], addresses: &mut BTreeSet<IpAddr>) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        windows_addresses(bytes, addresses)
    }
    #[cfg(target_os = "linux")]
    {
        linux_addresses(bytes, addresses)
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        let _ = (bytes, addresses);
        Err(DnsError(ErrorCode::Unavailable))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn platform_inventory_parsers_refuse_partial_unknown_empty_and_excess_data() {
        let mut addresses = BTreeSet::new();
        windows_addresses(
            b"127.0.0.1\r\nfe80::1%20\r\n::ffff:1.1.1.1\r\n",
            &mut addresses,
        )
        .unwrap();
        assert_eq!(addresses.len(), 3);
        assert!(addresses.contains(&"1.1.1.1".parse().unwrap()));
        for bytes in [
            b"".as_slice(),
            b"not-an-address\n",
            b"fe80::1%a\n",
            b"1.1.1.1\ninvalid\n",
        ] {
            assert!(windows_addresses(bytes, &mut BTreeSet::new()).is_err());
        }
        let valid=br#"[{"ifname":"lo","addr_info":[{"family":"inet","local":"127.0.0.1"},{"family":"inet6","local":"::1"}]}]"#;
        let mut linux = BTreeSet::new();
        linux_addresses(valid, &mut linux).unwrap();
        assert_eq!(linux.len(), 2);
        for bytes in [
            b"[]".as_slice(),
            b"{}",
            br#"[{"addr_info":[]}]"#,
            br#"[{"addr_info":[{"family":"inet6","local":"1.1.1.1"}]}]"#,
            br#"[{"addr_info":[{"family":"inet","local":"localhost"}]}]"#,
            br#"[{"addr_info":[{"family":"inet","local":"1.1.1.1"},{}]}]"#,
        ] {
            assert!(linux_addresses(bytes, &mut BTreeSet::new()).is_err());
        }
        let mut at_limit = BTreeSet::new();
        for n in 0..256 {
            insert(
                &mut at_limit,
                IpAddr::V4(std::net::Ipv4Addr::from(0x01000000 + n)),
            )
            .unwrap();
        }
        assert!(insert(&mut at_limit, "2.2.2.2".parse().unwrap()).is_err());
        assert_eq!(at_limit.len(), 256);
        assert!(
            Inventory::new(&vec!["1.1.1.1".parse().unwrap(); 257], std::iter::empty()).is_err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn actual_unprivileged_inventory_refuses_loopback_and_joins_cancelled_probe() {
        let inventory = Inventory::new(&[], std::iter::empty()).unwrap();
        // This functional probe includes a cold OS utility/runtime startup on
        // loaded CI hosts. It proves inventory/loop refusal, not500ms availability.
        // Production still supplies its original member deadline unchanged.
        let mut probe = inventory.start(
            "127.0.0.1".parse().unwrap(),
            tokio::time::Instant::now() + Duration::from_secs(10),
        );
        let outcome = tokio::time::timeout(
            Duration::from_secs(12),
            std::future::poll_fn(|context| probe.poll(context)),
        )
        .await
        .unwrap();
        assert_eq!(outcome.unwrap_err().code(), ErrorCode::Scope);
        // An already elapsed production-style deadline still fails closed;
        // the functional startup budget above must not become a forwarding retry.
        let mut expired = inventory.start("1.1.1.1".parse().unwrap(), tokio::time::Instant::now());
        let outcome = tokio::time::timeout(
            Duration::from_secs(3),
            std::future::poll_fn(|context| expired.poll(context)),
        )
        .await
        .unwrap();
        assert_eq!(outcome.unwrap_err().code(), ErrorCode::Timeout);
        let probe = inventory.start(
            "1.1.1.1".parse().unwrap(),
            tokio::time::Instant::now() + Duration::from_millis(500),
        );
        tokio::time::timeout(Duration::from_secs(3), probe.finish())
            .await
            .unwrap()
            .unwrap();
    }
}
