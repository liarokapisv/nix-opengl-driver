# nix-opengl-driver

A standalone CLI tool for managing the Nix OpenGL/Vulkan driver symlink farm on Linux.

## Overview

Nixpkgs packages that use OpenGL assume that an OpenGL symlink farm is available at `/run/opengl-driver`. This is baked into the runpath of some core OpenGL packages (for example `libGLX.so.0`'s runpath entry contains `/run/opengl-driver`). The contents of the symlink farm can be derived by going through the NixOS `hardware.graphics` module.

Solutions like [nixGL](https://github.com/nix-community/nixGL) try to delegate to the system OpenGL userspace stack through environment 
variable overriding. This is not properly respected by all applications, especially pre-wrapped ones, is intrusive, and doesn't always properly propagate across spawned processes. It can also lead to ABI incompatibilities due to symbol mismatches between the system OpenGL installation and the Nixpkgs provided one.

Another approach is to try to match the NixOS system. This means creating a valid symlink farm and exposing it at `/run/opengl-driver` so that Nixpkgs applications can properly access it without modifications. In that case, the symlink farm needs to be compatible with the system's installed kernel modules. For Mesa this is usually straightforward; for Nvidia one has to try to match the kernel driver's version (possible through quering  `/proc/drivers/nvidia/version )`. This creates a separate userspace OpenGL stack, but ensures we get the same compatibility as in NixOS.

This approach is currently used by [nix-system-graphics](https://github.com/soupglasses/nix-system-graphics), with the key limitations that 
it requires having first installed [system-manager](https://github.com/numtide/system-manager) and also needs manual configuration to populate the symlink farm and specify the Nvidia drivers if required.

`nix-opengl-driver` builds and maintains a symlink farm at `/run/opengl-driver` with:

- 64‑bit Mesa OpenGL/EGL drivers  
- Vulkan loader and ICD JSONs  
- OpenCL ICDs (Clover, PoCL)  
- VA-API/VDPAU support  
- Optional NVIDIA drivers via `mkDriver`

It integrates with systemd to auto-detect at boot the exact version of the NVIDIA drivers used.
The goal is to also integrate with other init-systems and transparently manage the `/run/opengl-driver` symlink farm for standalone Nix installations.

## Usage


```bash
Usage: nix-opengl-driver [OPTIONS] <COMMAND>

Commands:
  status              Show detected vs active driver and last sync info
  driver              Show only the detected (auto- or forced) driver
  code                Print the Nix expression for the symlink farm
  build               Build the symlink farm (prints store path; does not switch)
  sync                Build and switch the active symlink to the newly built farm
  tmpfiles            Print the tmpfiles.d rule for `/run/opengl-driver`
  tmpfiles-install    Install & apply the tmpfiles.d rule (creates `/run/opengl-driver`)
  tmpfiles-uninstall  Remove the tmpfiles.d rule
  service             Print the systemd oneshot-sync service unit to stdout
  service-install     Install & enable the systemd oneshot-sync service
  service-uninstall   Disable & remove the systemd oneshot-sync service
  install             Install both the tmpfiles rule (and apply it) and the on-boot sync service
  uninstall           Uninstall all state, GC-root, tmpfiles rule, and service
  state               Dump the raw JSON state file (or its backup)
  hash-store          Dump the persisted NVIDIA version→hash map
  help                Print this message or the help of the given subcommand(s)

Options:
      --quiet                   Only print the final result (store path) to stdout
      --force-mesa              Force using the Mesa software stack
      --force-nvidia <VERSION>  Force using NVIDIA with exactly this version
      --resolve-hashes          Actually resolve real NVIDIA hashes instead of placeholders
  -h, --help                    Print help
  -V, --version                 Print version
```
