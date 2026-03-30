{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, fenix, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        toolchain = fenix.packages.${system}.stable.withComponents [
          "cargo"
          "clippy"
          "rust-src"
          "rust-analyzer"
          "rustc"
          "rustfmt"
        ];

        craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;
          version="0.0.0";
          pname="zesh_workspace";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Helper to build individual workspace crates
        buildCrate = pname: 
        let
            crateInfo = craneLib.crateNameFromCargoToml { cargoToml = ./${pname}/Cargo.toml; };
        in
        craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          inherit (crateInfo) version;
          pname = crateInfo.pname or pname;
          cargoExtraArgs = "--package ${pname}";
        });
      in
      {
        checks = {
          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          fmt = craneLib.cargoFmt commonArgs;

          tests = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        packages = {
          zesh = buildCrate "zesh";
          zellij_rs = buildCrate "zellij_rs";
          zesh_git = buildCrate "zesh_git";
          zox_rs = buildCrate "zox_rs";
          default = self.packages.${system}.zesh;
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};

          packages = with pkgs; [
            cargo-watch
          ];
        };
      }
    );
}
