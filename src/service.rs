use anyhow::{Context, Result};
use std::{fs, process::Command};

/// The systemd unit, embedded at compile time
pub const SERVICE_UNIT: &str = include_str!("../templates/nix-opengl-driver-sync.service.in");

/// Print the unit file to stdout
pub fn print_service() {
    println!("{}", SERVICE_UNIT);
}

/// Install & enable the oneshot sync service
pub fn install_service() -> Result<()> {
    let path = "/etc/systemd/system/nix-opengl-driver-sync.service";
    fs::write(path, SERVICE_UNIT).with_context(|| format!("writing service unit to {}", path))?;

    // reload systemd to pick up the new unit
    Command::new("systemctl")
        .args(["daemon-reload"])
        .status()
        .context("running systemctl daemon-reload")?;

    // enable (so it runs on every boot)
    Command::new("systemctl")
        .args(["enable", "nix-opengl-driver-sync.service"])
        .status()
        .context("enabling nix-opengl-driver-sync.service")?;

    println!("Installed and enabled nix-opengl-driver-sync.service");
    Ok(())
}

/// Disable & remove the sync service
pub fn uninstall_service() -> Result<()> {
    // stop it (in case it's running)
    let _ = Command::new("systemctl")
        .args(["stop", "nix-opengl-driver-sync.service"])
        .status();

    // disable it
    Command::new("systemctl")
        .args(["disable", "nix-opengl-driver-sync.service"])
        .status()
        .context("disabling nix-opengl-driver-sync.service")?;

    // remove the unit file
    let path = "/etc/systemd/system/nix-opengl-driver-sync.service";
    fs::remove_file(path).with_context(|| format!("removing service unit {}", path))?;

    // reload systemd so it forgets about the unit
    Command::new("systemctl")
        .args(["daemon-reload"])
        .status()
        .context("running systemctl daemon-reload")?;

    println!("Uninstalled nix-opengl-driver-sync.service");
    Ok(())
}
