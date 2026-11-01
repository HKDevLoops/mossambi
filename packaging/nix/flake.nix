{
  description = "Mossambi: Universal File Conversion Platform & Indian Exam Studio";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        mossambi = pkgs.callPackage ./default.nix { };
      in
      {
        packages.default = mossambi;
        packages.mossambi = mossambi;

        apps.default = flake-utils.lib.mkApp {
          drv = mossambi;
          name = "mossambi";
        };

        apps.gui = flake-utils.lib.mkApp {
          drv = mossambi;
          name = "mossambi-gui";
        };

        apps.tui = flake-utils.lib.mkApp {
          drv = mossambi;
          name = "mossambi-tui";
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            pkg-config
            alsa-lib
            libxkbcommon
            wayland
            mesa
            libGL
            ffmpeg
          ];
        };
      }
    );
}
