use anyhow::{anyhow, Context, Result};
use regex::Regex;
use std::{fs, path::Path};

/// Which driver stack is active
#[derive(Debug)]
pub enum Driver {
    Nvidia(String),
    Mesa,
}

/// Detect NVIDIA vs Mesa
pub fn detect_driver() -> Result<Driver> {
    let npath = Path::new("/proc/driver/nvidia/version");
    if npath.exists() {
        let txt = fs::read_to_string(npath).context("reading NVIDIA version")?;
        let re = Regex::new(r"Kernel Module\s+([\d\.]+)").unwrap();
        if let Some(cap) = re.captures(&txt) {
            return Ok(Driver::Nvidia(cap[1].to_string()));
        } else {
            return Err(anyhow!("failed to parse NVIDIA version"));
        }
    }
    Ok(Driver::Mesa)
}
