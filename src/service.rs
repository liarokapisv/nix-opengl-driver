use anyhow::{bail, Context, Result};
use handlebars::Handlebars;
use serde::Serialize;
use std::{env, fs, path::Path, process::Command};

const GCROOT_TOOL: &str = "/nix/var/nix/gcroots/nix-opengl-driver/tool";
const SERVICE_NAME: &str = "nix-opengl-driver.service";
const SERVICE_PATH: &str = "/etc/systemd/system/nix-opengl-driver.service";

fn tool_path() -> Result<String> {
    let exe = env::current_exe().context("getting current exe path")?;
    let exe = exe.canonicalize().context("canonicalizing exe path")?;
    Ok(exe.to_str().expect("Path is not valid utf8").to_string())
}

fn tool_derivation_path(path: &str) -> String {
    path.splitn(5, '/').take(4).collect::<Vec<_>>().join("/")
}

fn pin_if_nix_executable(tool_path: &str, quiet: bool) -> Result<()> {
    if tool_path.starts_with("/nix/store/") {
        let gcroot_dir = Path::new(GCROOT_TOOL)
            .parent()
            .expect("GCROOT_TOOL must have a parent");
        fs::create_dir_all(gcroot_dir).context("creating GC-root directory")?;

        let status = Command::new("nix-store")
            .args([
                "--add-root",
                GCROOT_TOOL,
                "--indirect",
                &tool_derivation_path(tool_path),
            ])
            .status()
            .context("pinning tool in GC-root")?;
        if !status.success() {
            bail!(
                "nix-store --add-root failed (exit {})",
                status.code().unwrap_or(-1)
            );
        }
    } else if !quiet {
        eprintln!(
            "Binary is not in /nix/store, skipping GC-root; \
             service will invoke `{}` directly",
            tool_path
        );
    }

    Ok(())
}

fn render_service() -> Result<(String, String)> {
    let tool_path = tool_path().context("evaluating tool derivation for service installation")?;

    let tpl = include_str!("../templates/nix-opengl-driver.service.in");

    let mut hb = Handlebars::new();
    hb.register_template_string("service", tpl)?;

    #[derive(Serialize)]
    struct Args<'a> {
        tool_path: &'a str,
    }

    let rendered = hb
        .render(
            "service",
            &Args {
                tool_path: &tool_path,
            },
        )
        .context("rendering service unit")?;

    Ok((tool_path, rendered))
}

pub fn print_service() -> Result<()> {
    let (_, service_unit) = render_service().context("printing service")?;
    println!("{}", service_unit);

    Ok(())
}

pub fn install_service(quiet: bool) -> Result<()> {
    let (tool_path, service_unit) =
        render_service().context("rendering service unit for installation")?;

    pin_if_nix_executable(&tool_path, quiet).context("if nix executable, pin as a gc-root")?;

    Command::new("nix-store")
        .args([
            "--add-root",
            GCROOT_TOOL,
            "--indirect",
            "--realise",
            &tool_path,
        ])
        .status()
        .context("pinning tool in GC-root")?;

    fs::write(SERVICE_PATH, service_unit)
        .with_context(|| format!("writing service unit to {}", SERVICE_PATH))?;

    Command::new("systemctl")
        .args(["daemon-reload"])
        .status()
        .context("running systemctl daemon-reload")?;

    Command::new("systemctl")
        .args(["enable", SERVICE_NAME])
        .status()
        .context("enabling sync service")?;

    println!("Installed service pointing at {}", tool_path);
    Ok(())
}

pub fn uninstall_service() -> Result<()> {
    let _ = Command::new("systemctl")
        .args(["stop", SERVICE_NAME])
        .status();

    // disable it
    Command::new("systemctl")
        .args(["disable", SERVICE_NAME])
        .status()
        .context(format!("disabling {}", SERVICE_NAME))?;

    // remove the unit file
    fs::remove_file(SERVICE_PATH)
        .with_context(|| format!("removing service unit {}", SERVICE_PATH))?;

    // reload systemd so it forgets about the unit
    Command::new("systemctl")
        .args(["daemon-reload"])
        .status()
        .context("running systemctl daemon-reload")?;

    let _ = Command::new("nix-store")
        .args(["--delete-root", GCROOT_TOOL])
        .status();

    println!("Uninstalled {}", SERVICE_NAME);
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn tool_derivation_path_works() {
        let tool_path = "/nix/store/fdz1cyfchp65rc9rdidyffx1n21i473v-nix-opengl-driver-0.1.0/bin/nix-opengl-driver";
        assert_eq!(
            tool_derivation_path(tool_path),
            "/nix/store/fdz1cyfchp65rc9rdidyffx1n21i473v-nix-opengl-driver-0.1.0"
        );
    }
}
