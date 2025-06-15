use crate::detect::Driver;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

pub const GCROOT_SYMLINK: &str = "/nix/var/nix/gcroots/nix-opengl-driver/current";
pub const STATE_FILE: &str = "/var/lib/nix-opengl-driver/state.json";
pub const STATE_BAK: &str = "/var/lib/nix-opengl-driver/state.json.bak";

#[derive(Serialize, Deserialize)]
pub struct State {
    pub detected: String,
    pub active: String,
    pub last_sync: String,
}

impl State {
    pub fn load() -> Option<Self> {
        let txt = fs::read_to_string(STATE_FILE)
            .or_else(|_| fs::read_to_string(STATE_BAK))
            .ok()?;
        serde_json::from_str(&txt).ok()
    }

    pub fn save(d: &Driver, active: &Path) -> std::io::Result<()> {
        let s = State {
            detected: match d {
                Driver::Nvidia(v) => format!("nvidia {}", v),
                Driver::Mesa => "mesa".into(),
            },
            active: active.display().to_string(),
            last_sync: Utc::now().to_rfc3339(),
        };
        let json = serde_json::to_string_pretty(&s)?;
        let tmp = format!("{}.tmp", STATE_FILE);
        fs::create_dir_all(Path::new(STATE_FILE).parent().unwrap())?;
        fs::write(&tmp, &json)?;
        fs::rename(STATE_FILE, STATE_BAK).ok();
        fs::rename(tmp, STATE_FILE)?;
        Ok(())
    }
}
