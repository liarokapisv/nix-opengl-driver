use clap::{ArgGroup, Parser, Subcommand};

/// Manage the Nix-based OpenGL driver symlink farm
#[derive(Parser)]
#[command(author, version, about)]
#[command(group(
    ArgGroup::new("force")
        .args(&["force_nvidia", "force_mesa"])
        .multiple(false)
))]
pub struct Cli {
    /// Only print the final result (store path) to stdout
    #[arg(long)]
    pub quiet: bool,

    /// Force using the Mesa software stack
    #[arg(long, group = "force")]
    pub force_mesa: bool,

    /// Force using NVIDIA with exactly this version
    #[arg(long, value_name = "VERSION", group = "force")]
    pub force_nvidia: Option<String>,

    /// Actually resolve real NVIDIA hashes instead of placeholders
    #[arg(long)]
    pub resolve_hashes: bool,

    #[command(subcommand)]
    pub cmd: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show detected vs active driver and last sync info
    Status,

    /// Show only the detected (auto- or forced) driver
    Driver,

    /// Print the Nix expression for the symlink farm
    Code,

    /// Build the symlink farm (prints store path; does not switch)
    Build,

    /// Build and switch the active symlink to the newly built farm
    Sync,

    /// Print the tmpfiles.d rule for `/run/opengl-driver`
    Tmpfiles,

    /// Install & apply the tmpfiles.d rule (creates `/run/opengl-driver`)
    TmpfilesInstall,

    /// Remove the tmpfiles.d rule.
    TmpfilesUninstall,

    /// Print the systemd oneshot service unit to stdout
    Service,

    /// Install & enable the systemd oneshot service
    ServiceInstall,

    /// Disable & remove the systemd oneshot service
    ServiceUninstall,

    /// Install both the tmpfiles rule (and apply it) and the on-boot sync service
    Install,

    /// Uninstall all state, GC-root, tmpfiles rule, and service
    Uninstall,

    /// Dump the raw JSON state file (or its backup)
    State,

    /// Dump the persisted NVIDIA version→hash map
    HashStore,
}
