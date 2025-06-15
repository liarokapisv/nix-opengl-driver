use anyhow::{Context, Result};
use handlebars::Handlebars;
use serde::Serialize;
use std::{fs, path::Path, process::Command};

const GCROOT_TOOL: &str = "/nix/var/nix/gcroots/nix-opengl-driver/tool";
const FLAKE_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const SERVICE_NAME: &str = "nix-opengl-driver.service";
const SERVICE_PATH: &str = "/etc/systemd/system/nix-opengl-driver.service";

fn tool_path() -> Result<String> {
    let flake_ref = format!("{FLAKE_ROOT}#nix-opengl-driver");
    println!("flake_ref: {FLAKE_ROOT}");
    let out = Command::new("nix")
        .args(&["build", "--print-out-paths", "--no-link", &flake_ref])
        .output()
        .context("running `nix build` for tool")?;
    if !out.status.success() {
        anyhow::bail!("`nix build {}` failed", flake_ref);
    }
    Ok(String::from_utf8(out.stdout)?.trim().to_string())
}

fn render_service() -> Result<(String, String)> {
    let tool_path = tool_path().context("evaluating tool derivation for service installation")?;

    let tpl = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("templates/{}.in", SERVICE_NAME)),
    )
    .context("reading service unit template")?;

    let mut hb = Handlebars::new();
    hb.register_template_string("service", tpl)?;

    #[derive(Serialize)]
    struct Args<'a> {
        tool_path: &'a str,
    }

    let data = Args {
        tool_path: &tool_path,
    };

    let rendered = hb
        .render("service", &data)
        .context("rendering service unit")?;

    Ok((tool_path, rendered))
}

pub fn print_service() -> Result<()> {
    Ok(println!(
        "{}",
        render_service().context("printing service")?.1
    ))
}

pub fn install_service() -> Result<()> {
    let (tool_path, service_unit) =
        render_service().context("rendering service unit for installation")?;

    Command::new("nix-store")
        .args(&[
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
        .args(&["daemon-reload"])
        .status()
        .context("running systemctl daemon-reload")?;

    Command::new("systemctl")
        .args(&["enable", SERVICE_NAME])
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
        .args(&["--delete-root", GCROOT_TOOL])
        .status();

    println!("Uninstalled {}", SERVICE_NAME);
    Ok(())
}
