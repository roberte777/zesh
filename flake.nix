{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
  inputs.utils.url = "github:numtide/flake-utils";
  inputs.naersk.url = "github:nmattia/naersk";
  inputs.naersk.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, utils, naersk }:
    utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };
        naersk-lib = pkgs.callPackage naersk { };
      in
      {
        packages = {
          zesh = naersk-lib.buildPackage {
            src = ./.;
            pname = "zesh";
          };
          zellij_rs = naersk-lib.buildPackage {
            src = ./.;
            pname = "zellij_rs";
          };
          zesh_git = naersk-lib.buildPackage {
            src = ./.;
            pname = "zesh_git";
          };
          zox_rs = naersk-lib.buildPackage {
            src = ./.;
            pname = "zox_rs";
          };
          default = self.packages.${system}.zesh;
        };

        devShells.default = with pkgs; mkShell {
          buildInputs = [
            cargo
            rustc
            rustfmt
            pre-commit
            clippy
          ];
        };
      });
}
