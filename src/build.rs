use crate::detect::Driver;
use anyhow::{bail, Context, Result};
use handlebars::Handlebars;
use crate::hash_store::HashStore;
use regex::Regex;
use serde::Serialize;
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
};
use tempfile::TempDir;

/// Render the Nix expression from our Handlebars templates.
pub fn render_nix_expr(driver: &Driver, hash: Option<&str>) -> Result<String> {
    let tpl_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    let mesa_tpl = fs::read_to_string(tpl_dir.join("nix-opengl-driver.mesa.nix.in"))?;
    let nvidia_tpl = fs::read_to_string(tpl_dir.join("nix-opengl-driver.nvidia.nix.in"))?;

    let mut hb = Handlebars::new();
    hb.register_template_string("mesa", mesa_tpl)?;
    hb.register_template_string("nvidia", nvidia_tpl)?;

    let sha256 = hash.unwrap_or("");
    let version = if let Driver::Nvidia(v) = driver {
        v.as_str()
    } else {
        ""
    };

    Ok(match driver {
        Driver::Nvidia(_) => {
            #[derive(Serialize)]
            struct Substitutions<'a> {
                version: &'a str,
                sha256: &'a str,
            }

            let subs = Substitutions { version, sha256 };
            hb.render("nvidia", &subs)?
        }
        Driver::Mesa => hb.render("mesa", &())?,
    })
}

/// Write `default.nix` into `dir` using our renderer.
fn write_nix_expr(dir: &Path, driver: &Driver, hash: Option<&str>) -> Result<()> {
    let expr = render_nix_expr(driver, hash)?;
    fs::write(dir.join("default.nix"), expr)?;
    Ok(())
}

fn run_nix(dir: &Path, quiet: bool) -> Result<(ExitStatus, String)> {
    let mut child = Command::new("nix")
        .args(["build", "-f", dir.to_str().unwrap(), "-o", "result"])
        .current_dir(dir)
        .stdout(if !quiet {
            Stdio::inherit()
        } else {
            Stdio::null()
        })
        .stderr(Stdio::piped())
        .spawn()
        .context("spawning `nix build`")?;

    let mut stderr_buf = String::new();
    if let Some(stderr) = child.stderr.take() {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        while reader.read_line(&mut line)? > 0 {
            eprint!("{line}");
            stderr_buf.push_str(&line);
            line.clear();
        }
    }

    let status = child.wait().context("waiting on nix build")?;
    Ok((status, stderr_buf))
}

fn extract_hash(s: &str) -> Option<String> {
    let re = Regex::new(r"got:\s*(sha256-[A-Za-z0-9+/=]+)").unwrap();
    let cap = re.captures(s)?;
    Some(cap[1].to_string())
}

pub fn resolve_hash(driver: &Driver, quiet: bool) -> Result<String> {
    if let Driver::Nvidia(ver) = driver {
        let mut store = HashStore::load()?;
        // 1) If we already know this version → return it
        if let Some(old) = store.get(ver) {
            return Ok(old.clone());
        }
        // 2) Else do the two-phase Nix run as before…
        let tmp = TempDir::new().context("creating tempdir")?;
        let dir = tmp.path();
        write_nix_expr(dir, driver, None)?;
        let (status, stderr) = run_nix(dir, quiet)?;
        if status.success() {
            return Ok(String::new());
        }
        let hash = extract_hash(&stderr).context("could not find sha256 in Nix output")?;

        // 3) Persist it before returning
        store.insert(ver.clone(), hash.clone())?;
        return Ok(hash);
    }
    // Not NVIDIA → no hash
    Ok(String::new())
}

pub fn build_farm(driver: &Driver, quiet: bool) -> Result<PathBuf> {
    let tmp = TempDir::new().context("creating tempdir")?;
    let dir = tmp.path();

    // 1) figure out the real hash (or "")
    let sha = resolve_hash(driver, quiet).context("resolving hash before building")?;

    // 2) write expression with real hash
    write_nix_expr(dir, driver, Some(&sha))?;

    // 3) build with live progress
    let (status, stderr) = run_nix(dir, quiet)?;
    if !status.success() {
        bail!("`nix build` failed:\n{stderr}");
    }

    // 4) return the canonicalized result link
    fs::canonicalize(dir.join("result")).context("resolving result")
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn hash_is_properly_extracted() {
        const OUTPUT: &'static str = r#"
building '/nix/store/09x2dbx2mb0chnr02rg6fg48fphm8s44-intel-ocl-5.0-63503.drv'...
error: hash mismatch in fixed-output derivation '/nix/store/0sv2pvsyilwvi488kkjw6mbx6h8sv4yv-NVIDIA-Linux-x86_64-570.133.07.run.drv':
         specified: sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=
            got:    sha256-LUPmTFgb5e9VTemIixqpADfvbUX1QoTT2dztwI3E3CY=
error: 1 dependencies of derivation '/nix/store/d15fr4ik98n677hri556aqwkds0cz6sb-nvidia-x11-570.133.07.drv' failed to build
        "#;

        assert_eq!(
            extract_hash(&OUTPUT).as_deref(),
            Some("sha256-LUPmTFgb5e9VTemIixqpADfvbUX1QoTT2dztwI3E3CY=")
        );
    }
}
