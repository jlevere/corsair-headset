{
  description = "Corsair iCUE RE workbench — headset protocol analysis & Rust implementation";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config = {
            allowUnfree = true;
            allowBroken = true;
          };
        };

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
        devShells.default = pkgs.mkShell {
          name = "icue-re";

          packages = [
            # Binary analysis
            rizinWithPlugins
            pkgs.radare2

            # Python scripting
            python

            # Utilities
            pkgs.file
            pkgs.hexyl
            pkgs.jq
            pkgs.binwalk

            # Rust toolchain
            pkgs.rustc
            pkgs.cargo
            pkgs.rust-analyzer
            pkgs.clippy
            pkgs.rustfmt
            pkgs.wasm-pack
            pkgs.wasm-bindgen-cli

            # Native HID access
            pkgs.hidapi
            pkgs.pkg-config

            # WASM linker
            pkgs.lld

            # Frontend
            pkgs.bun
          ];

          shellHook = ''
            echo "icue-re workbench"
            echo ""
            echo "RE:    rizin <binary>  |  r2 <binary>"
            echo "Rust:  cargo build  |  cargo test  |  wasm-pack build crates/corsair-web"
            echo ""
          '';
        };
      });
}
