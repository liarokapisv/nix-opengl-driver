use anyhow::{Context, Result};
use dirs::data_local_dir;
use libc::geteuid;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, fs, path::PathBuf};

/// Global path used by root‐run services
const GLOBAL_STORE: &str = "/var/lib/nix-opengl-driver/hashmap.json";

#[derive(Deserialize, Serialize, Default)]
struct Mapping {
    map: HashMap<String, String>,
}

pub struct HashStore {
    data: Mapping,
    path: PathBuf,
}

impl HashStore {
    /// Pick the correct path: global if root, else per-user.
    fn store_path() -> PathBuf {
        // if running as root, use the global path
        if unsafe { geteuid() } == 0 {
            PathBuf::from(GLOBAL_STORE)
        } else {
            // per-user cache under $XDG_STATE_HOME/nix-opengl-driver
            let mut p = data_local_dir()
                .unwrap_or_else(|| {
                    // fallback to ~/.local/state
                    dirs::home_dir()
                        .expect("home_dir missing")
                        .join(".local")
                        .join("state")
                })
                .join("nix-opengl-driver");
            p.push("hashmap.json");
            p
        }
    }

    /// Load existing or start empty.
    pub fn load() -> Result<Self> {
        let path = Self::store_path();
        let data = if path.exists() {
            let s = fs::read_to_string(&path)
                .with_context(|| format!("reading hash store at {}", path.display()))?;
            serde_json::from_str(&s)
                .with_context(|| format!("parsing JSON in {}", path.display()))?
        } else {
            Mapping::default()
        };
        Ok(HashStore { data, path })
    }

    /// If we have a mapping for this version, return it.
    pub fn get(&self, version: &str) -> Option<&String> {
        self.data.map.get(version)
    }

    /// Insert and immediately persist to disk.
    /// On PermissionDenied, prints a warning and continues.
    pub fn insert(&mut self, version: String, hash: String) -> Result<()> {
        self.data.map.insert(version.clone(), hash.clone());
        let json_txt =
            serde_json::to_string_pretty(&self.data).context("serializing hash store")?;

        let path = &self.path;
        if let Some(dir) = path.parent() {
            if let Err(e) = fs::create_dir_all(dir) {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    eprintln!(
                        "⚠️  Warning: cannot create hash store directory `{}`: {}",
                        dir.display(),
                        e
                    );
                    return Ok(());
                }
                return Err(e).context("creating directory for hash store");
            }
        }

        if let Err(e) = fs::write(path, json_txt) {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                eprintln!(
                    "⚠️  Warning: cannot write hash store `{}`: {}",
                    path.display(),
                    e
                );
                return Ok(());
            }
            return Err(e).with_context(|| format!("writing hash store to {}", path.display()));
        }

        Ok(())
    }
}

/// Pretty-print the loaded store to stdout.
pub fn print_store() -> Result<()> {
    let hs = HashStore::load().context("loading hash store")?;
    let out = json!({ "map": hs.data.map });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
