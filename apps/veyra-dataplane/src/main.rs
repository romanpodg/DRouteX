//! `veyra-dataplane`: serves DNS and, with a `gateway:` section, the gateway
//! for a policy file (spec §5.2, §9, §10).

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;
use veyra_dns::{Engine, Listeners};
use veyra_gateway::{Gateway, Service};
use veyra_policy::{Bundle, Policy};

#[derive(Parser)]
#[command(
    name = "veyra-dataplane",
    version,
    about = "Veyra data plane: DNS and gateway"
)]
struct Args {
    /// The policy file; it must have `dns:` and `admission:` sections, and
    /// `gateway:` to serve the gateway too.
    #[arg(long)]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> ExitCode {
    // Logs go to stderr (RUST_LOG, default `info`); stdout carries only the
    // "listening on" lines that tests and service managers read.
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
    match run(Args::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Result<(), String> {
    let path = args.config.display();
    let text = std::fs::read_to_string(&args.config).map_err(|e| format!("{path}: {e}"))?;
    let bundle = Bundle::from_yaml(&text).map_err(|e| format!("{path}: {e}"))?;
    let policy = match Policy::compile(&bundle) {
        Ok((policy, warnings)) => {
            warnings.iter().for_each(|w| tracing::warn!("{w}"));
            policy
        }
        Err(issues) => {
            issues.iter().for_each(|i| eprintln!("{i}"));
            return Err(format!("{path}: has errors, run `veyra validate`"));
        }
    };
    let listen = policy
        .dns()
        .map(|dns| dns.listen.clone())
        .unwrap_or_default();
    // The gateway reads the same policy: DNS hands out its address (spec §10.2).
    let gateway = match policy.gateway_config().cloned() {
        Some(config) => {
            let gateway = Gateway::new(policy.clone()).map_err(|e| format!("{path}: {e}"))?;
            Some((config, gateway))
        }
        None => None,
    };
    let engine = Arc::new(Engine::new(policy).map_err(|e| format!("{path}: {e}"))?);
    let listeners = Listeners::bind(&listen)
        .await
        .map_err(|e| format!("cannot listen on {listen:?}: {e}"))?;
    for addr in listeners.local_addrs().map_err(|e| e.to_string())? {
        println!("listening on {addr} (udp, tcp)");
    }
    let mut gateway_listeners = None;
    if let Some((config, gateway)) = gateway {
        let bound = veyra_gateway::Listeners::bind(&config)
            .await
            .map_err(|e| format!("cannot listen on {:?}: {e}", [config.tls, config.http]))?;
        for (addr, service) in bound.local_addrs().map_err(|e| e.to_string())? {
            let service = match service {
                Service::Tls => "tls",
                Service::Http => "http",
            };
            println!("listening on {addr} ({service})");
        }
        gateway_listeners = Some(bound.serve(Arc::new(gateway)));
    }
    let gateway = async {
        match gateway_listeners {
            Some(serve) => serve.await,
            None => std::future::pending().await,
        }
    };
    tokio::select! {
        () = listeners.serve(engine) => Ok(()),
        () = gateway => Ok(()),
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("shutting down");
            Ok(())
        }
    }
}
