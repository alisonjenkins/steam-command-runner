{
  description = "Steam compatibility tool and command wrapper for Linux gaming";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, ... }@inputs:
    inputs.flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import inputs.nixpkgs) {
          inherit system;
        };

        naersk = pkgs.callPackage inputs.naersk { };

      in
      {
        # For `nix build` & `nix run`:
        packages.default = naersk.buildPackage {
          src = ./.;
          pname = "steam-command-runner";
          version = "0.2.0";

          meta = with pkgs.lib; {
            description = "Steam compatibility tool and command wrapper for Linux gaming";
            homepage = "https://github.com/alisonjenkins/steam-command-runner";
            license = licenses.mit;
            platforms = platforms.linux;
          };
        };

        # Legacy alias
        defaultPackage = self.packages.${system}.default;

        # For `nix develop`:
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            cargo
            rustc
            rust-analyzer
            clippy
            rustfmt
          ];

          # For running tests
          buildInputs = with pkgs; [
            cacert
          ];
        };

        # Legacy alias
        devShell = self.devShells.${system}.default;
      }
    );
}
