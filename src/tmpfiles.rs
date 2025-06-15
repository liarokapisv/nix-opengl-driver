use crate::state::GCROOT_SYMLINK;
use anyhow::{bail, Context as _, Result};
use std::{fs, io::ErrorKind, process::Command};

pub const RUN_SYMLINK: &str = "/run/opengl-driver";
pub const TMPFILES_CONF: &str = "/etc/tmpfiles.d/nix-opengl-driver.conf";

/// Print the tmpfiles.d rule
pub fn print_rule() {
    println!("L {} - - - - {}", RUN_SYMLINK, GCROOT_SYMLINK);
}

/// Install `/etc/tmpfiles.d/nix-opengl-driver.conf`
pub fn install_rule() -> Result<()> {
    let rule = format!("L {} - - - - {}\n", RUN_SYMLINK, GCROOT_SYMLINK);
    fs::write(TMPFILES_CONF, rule)
        .with_context(|| format!("writing tmpfiles rule to {}", TMPFILES_CONF))?;
    println!("Installed tmpfiles.d rule.");
    Command::new("systemd-tmpfiles")
        .args(["--create", "/etc/tmpfiles.d/nix-opengl-driver.conf"])
        .status()
        .context("applying tmpfiles rule")?;
    println!("/run/opengl-driver is now populated.");
    Ok(())
}

/// Uninstall the tmpfiles rule **and** remove `/run/opengl-driver`
pub fn uninstall_rule() -> Result<()> {
    // 1) remove the tmpfiles configuration
    match fs::remove_file(TMPFILES_CONF) {
        Ok(()) => {}
        Err(e) => match e.kind() {
            ErrorKind::NotFound => {} // already gone
            ErrorKind::PermissionDenied => {
                bail!("permission denied removing {}: {}", TMPFILES_CONF, e)
            }
            _ => return Err(e).context("removing tmpfiles config"),
        },
    }
    Ok(())
}
