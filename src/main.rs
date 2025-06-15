mod build;
mod cli;
mod detect;
mod hash_store;
mod service;
mod state;
mod tmpfiles;

use anyhow::Context as _;
use anyhow::Result;
use clap::Parser;
use detect::Driver;
use log::info;
use std::path::Path;
use std::{fs, process::Command};

fn main() -> Result<()> {
    env_logger::init();
    let cli = cli::Cli::parse();

    match cli.cmd {
        cli::Commands::Status => {
            let d = pick_driver(&cli)?;
            println!("Detected driver: {:?}", d);
            if let Some(s) = state::State::load() {
                println!("Active driver: {}", s.detected);
                println!("Active path:   {}", s.active);
                println!("Last sync:     {}", s.last_sync);
            } else {
                println!("Active driver: <none> (run `nix-opengl-driver sync`)");
            }
        }
        cli::Commands::Driver => {
            println!("{:?}", pick_driver(&cli)?);
        }
        cli::Commands::Code => {
            let d = pick_driver(&cli)?;
            let nix_expr = if cli.resolve_hashes {
                match &d {
                    Driver::Nvidia(_) => {
                        // two‐phase resolve
                        let sha = build::resolve_hash(&d, cli.quiet)
                            .context("resolving NVIDIA hash for --resolve-hashes")?;
                        build::render_nix_expr(&d, Some(&sha))?
                    }
                    _ => {
                        // nothing to resolve
                        build::render_nix_expr(&d, None)?
                    }
                }
            } else {
                // placeholders only
                build::render_nix_expr(&d, None)?
            };
            println!("{}", nix_expr);
        }
        cli::Commands::Build => {
            let d = pick_driver(&cli)?;
            let p = build::build_farm(&d, cli.quiet)?;
            println!("{}", p.display());
        }
        cli::Commands::Sync => {
            let d = pick_driver(&cli)?;
            let p = build::build_farm(&d, cli.quiet)?;
            info!("Updating GC root");
            let status = Command::new("nix-store")
                .args([
                    "--add-root",
                    state::GCROOT_SYMLINK,
                    "--indirect",
                    "--realise",
                    &p.to_string_lossy(),
                ])
                .status()?;
            if !status.success() {
                eprintln!("nix-store failed");
            }
            state::State::save(&d, &p)?;
            println!("Synced: {}", p.display());
        }
        cli::Commands::State => {
            // Prefer the primary state file, otherwise the backup
            let path = if Path::new(state::STATE_FILE).exists() {
                state::STATE_FILE
            } else if Path::new(state::STATE_BAK).exists() {
                eprintln!("primary state missing, reading backup");
                state::STATE_BAK
            } else {
                eprintln!(
                    "no state file found at {} or {}",
                    state::STATE_FILE,
                    state::STATE_BAK
                );
                std::process::exit(1);
            };

            let data = fs::read_to_string(path).unwrap_or_else(|e| {
                eprintln!("failed to read {}: {}", path, e);
                std::process::exit(1);
            });
            println!("{}", data);
        }
        cli::Commands::Tmpfiles => {
            tmpfiles::print_rule();
        }
        cli::Commands::TmpfilesInstall => {
            tmpfiles::install_rule()?;
            println!("Installed tmpfiles.d rule and populated /run/opengl-driver");
        }
        cli::Commands::TmpfilesUninstall => {
            tmpfiles::uninstall_rule().context("uninstalling tmpfiles rule")?;
            println!("Uninstalled tmpfiles rule");
        }
        cli::Commands::Service => service::print_service().context("printing systemd service")?,
        cli::Commands::ServiceInstall => {
            service::install_service(cli.quiet).context("installing systemd service")?;
        }
        cli::Commands::ServiceUninstall => {
            service::uninstall_service().context("uninstalling systemd service")?;
        }
        cli::Commands::Install => {
            tmpfiles::install_rule().context("installing tmpfiles rule")?;
            service::install_service(cli.quiet).context("installing systemd service")?;
            println!("Installed tmpfiles.d rule, service file and populated /run/opengl-driver");
        }
        cli::Commands::Uninstall => {
            use std::io::ErrorKind;
            // files to remove (gcroot, state, backup rule)
            let paths = [state::GCROOT_SYMLINK, state::STATE_FILE, state::STATE_BAK];
            for &path in &paths {
                match fs::remove_file(path) {
                    Ok(()) => {}
                    Err(e) => match e.kind() {
                        ErrorKind::NotFound => continue,
                        ErrorKind::PermissionDenied => {
                            eprintln!("Failed to remove {}: {}", path, e);
                            std::process::exit(1);
                        }
                        _ => return Err(e.into()),
                    },
                }
            }
            service::uninstall_service().context("uninstalling systemd service")?;
            tmpfiles::uninstall_rule().context("uninstalling tmpfiles rule")?;
            println!("Uninstalled gc-root, state, tmpfiles rule and service");
        }
        cli::Commands::HashStore => {
            hash_store::print_store().context("printing hash store")?;
        }
    }

    Ok(())
}

fn pick_driver(cli: &cli::Cli) -> Result<Driver, anyhow::Error> {
    if let Some(ver) = &cli.force_nvidia {
        Ok(Driver::Nvidia(ver.clone()))
    } else if cli.force_mesa {
        Ok(Driver::Mesa)
    } else {
        detect::detect_driver()
    }
}
