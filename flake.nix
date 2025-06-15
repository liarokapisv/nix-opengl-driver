{
  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-flake.url = "github:juspay/rust-flake";
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } (
      { self, ... }:
      {
        imports = [
          inputs.rust-flake.flakeModules.default
          inputs.rust-flake.flakeModules.nixpkgs
        ];
        systems = [
          "x86_64-linux"
        ];

        perSystem =
          {
            self',
            config,
            lib,
            pkgs,
            ...
          }:
          {
            rust-project.src =
              let
                unfilteredRoot = ./.; # The original, unfiltered source
              in
              lib.fileset.toSource {
                root = unfilteredRoot;
                fileset = lib.fileset.unions [
                  (config.rust-project.crane-lib.fileset.commonCargoSources unfilteredRoot)
                  (lib.fileset.maybeMissing ./templates)
                ];
              };

            devShells.default = pkgs.mkShell {
              inputsFrom = [
                self'.devShells.rust
              ];
              packages = [
                pkgs.nix
              ];
            };

            packages.default = self'.packages.nix-opengl-driver;
          };
      }
    );
}
