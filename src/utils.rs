use std::{fs, os::unix::fs as unix_fs, path::Path};

use anyhow::Context as _;

pub fn pin_store_path(store_path: &str, gcroot: &str) -> anyhow::Result<()> {
    let gcroot = Path::new(gcroot);

    if let Some(dir) = gcroot.parent() {
        fs::create_dir_all(dir).context("creating GC-root directory")?;
    }
    let _ = fs::remove_file(gcroot);
    unix_fs::symlink(store_path, gcroot).context("creating GC-root symlink")?;

    Ok(())
}
