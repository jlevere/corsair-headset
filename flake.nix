{
  description = "Corsair Headset — macOS menu bar control app";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, flake-utils, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };

        craneLib = crane.mkLib pkgs;

        # Common args for all crate builds
        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          buildInputs = [ pkgs.hidapi ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.apple-sdk_15
          ];
          nativeBuildInputs = [ pkgs.pkg-config ];
        };

        # Build deps once, share across targets
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the tray binary
        corsair-tray = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "-p corsair-tray";
        });

        # Build the CLI binary
        corsair-cli = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "-p corsair-cli";
        });

        # Assemble the macOS .app bundle
        corsair-app = pkgs.stdenv.mkDerivation {
          pname = "corsair-headset";
          version = "0.8.0";

          src = ./bundle;

          buildInputs = [ corsair-tray ];

          installPhase = ''
            mkdir -p "$out/Applications/Corsair Headset.app/Contents/MacOS"
            mkdir -p "$out/Applications/Corsair Headset.app/Contents/Resources"
            cp ${./bundle/Info.plist} "$out/Applications/Corsair Headset.app/Contents/Info.plist"
            cp ${./bundle/AppIcon.icns} "$out/Applications/Corsair Headset.app/Contents/Resources/AppIcon.icns"
            cp ${corsair-tray}/bin/corsair-tray "$out/Applications/Corsair Headset.app/Contents/MacOS/corsair-tray"
            echo 'APPL????' > "$out/Applications/Corsair Headset.app/Contents/PkgInfo"
          '';
        };

        # RE tools (optional dev shell for protocol analysis)
        python = pkgs.python313.withPackages (ps: [
          ps.r2pipe
          ps.rzpipe
          ps.capstone
          ps.unicorn
          ps.keystone-engine
          ps.protobuf
          ps.construct
        ]);

        rizinWithPlugins = pkgs.rizin.withPlugins (ps: [
          ps.rz-ghidra
          ps.jsdec
          ps.sigdb
        ]);

      in {
        packages = {
          default = corsair-app;
          tray = corsair-tray;
          cli = corsair-cli;
          app = corsair-app;
        };

        checks = {
          inherit corsair-tray corsair-cli;
          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "-- -D warnings";
          });
          tests = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = [
            # RE tools
            rizinWithPlugins
            pkgs.radare2
            python
            pkgs.file
            pkgs.hexyl
            pkgs.jq
            pkgs.binwalk

            # HID
            pkgs.hidapi
            pkgs.pkg-config

          ];
        };
      });
}
