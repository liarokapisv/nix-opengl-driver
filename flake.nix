{
  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-flake.url = "github:juspay/rust-flake";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.rust-flake.flakeModules.default
        inputs.rust-flake.flakeModules.nixpkgs
      ];
      systems = [
        "x86_64-linux"
      ];

      perSystem =
        { self', pkgs, ... }:
        {
          devShells.default = pkgs.mkShell {
            inputsFrom = [
              self'.devShells.rust
            ];
            packages = [
              pkgs.nix
            ];
          };
        };
    };
}
