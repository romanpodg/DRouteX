//! `veyra` command line, stage 1a (spec §23.2): validate and explain a policy file.

use clap::{Parser, Subcommand};
use serde::Serialize;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use veyra_policy::{Bundle, Decision, Issue, Mode, Name, Policy};

#[derive(Parser)]
#[command(name = "veyra", version, about = "Veyra policy tools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check a policy file and print every error and warning.
    Validate { config: PathBuf },
    /// Show which rule decides a domain for a profile, device or source address, as JSON.
    Explain {
        #[arg(long)]
        config: PathBuf,
        #[arg(long, required_unless_present_any = ["device", "source"])]
        profile: Option<String>,
        #[arg(long)]
        device: Option<String>,
        /// Client address; its profile comes from `admission` like a DNS query's would.
        #[arg(long, conflicts_with_all = ["profile", "device"])]
        source: Option<IpAddr>,
        /// Connection mode: smartdns_tcp, explicit_proxy or tunnel.
        #[arg(long, default_value = "smartdns_tcp")]
        mode: Mode,
        domain: String,
    },
}

/// The `veyra explain` JSON: the decision plus the inputs it was made for.
/// Fields are only ever added during stage 1a, never renamed.
#[derive(Serialize)]
struct Explain<'a> {
    #[serde(flatten)]
    decision: &'a Decision,
    mode: Mode,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<IpAddr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_binding: Option<String>,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Validate { config } => match load(&config)? {
            Ok((_, warnings)) => {
                warnings.iter().for_each(|w| println!("{w}"));
                println!("{}: valid, {} warning(s)", config.display(), warnings.len());
                Ok(())
            }
            Err(issues) => {
                issues.iter().for_each(|i| println!("{i}"));
                let errors = issues
                    .iter()
                    .filter(|i| i.severity == veyra_policy::Severity::Error)
                    .count();
                Err(format!("{}: {errors} error(s)", config.display()))
            }
        },
        Command::Explain {
            config,
            profile,
            device,
            source,
            mode,
            domain,
        } => {
            let policy = match load(&config)? {
                Ok((policy, warnings)) => {
                    warnings.iter().for_each(|w| eprintln!("{w}"));
                    policy
                }
                Err(issues) => {
                    issues.iter().for_each(|i| eprintln!("{i}"));
                    return Err(format!(
                        "{}: has errors, run `veyra validate`",
                        config.display()
                    ));
                }
            };
            let (subject, source_binding) = match source {
                Some(ip) => {
                    let admitted = policy
                        .admit(ip)
                        .ok_or_else(|| format!("source {ip} is not admitted (spec §9.6)"))?;
                    (admitted.subject, Some(admitted.binding))
                }
                None => {
                    let subject = policy
                        .subject(profile.as_deref(), device.as_deref())
                        .map_err(|e| e.to_string())?;
                    (subject, None)
                }
            };
            let name = Name::parse(&domain).map_err(|e| format!("{domain:?}: {e}"))?;
            let decision = policy.decide_in(&subject, &name, Some(mode));
            let explain = Explain {
                decision: &decision,
                mode,
                source,
                source_binding,
            };
            let json = serde_json::to_string_pretty(&explain).map_err(|e| e.to_string())?;
            println!("{json}");
            Ok(())
        }
    }
}

type Compiled = Result<(Policy, Vec<Issue>), Vec<Issue>>;

fn load(path: &Path) -> Result<Compiled, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let bundle = Bundle::from_yaml(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Policy::compile(&bundle))
}
