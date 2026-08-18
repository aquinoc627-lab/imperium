{
  description = "IMPERIUM - The Self-Synthesizing Intent Runtime";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    poetry2nix.url = "github:nix-community/poetry2nix";
  };
  outputs = { self, nixpkgs, flake-utils, rust-overlay, poetry2nix, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rust = pkgs.rust-bin.stable.latest.default;
        python = pkgs.python312;
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rust cargo-nextest
            python python312Packages.uv
            nodejs_20 pnpm
            wasmtime wasi-sdk
            cosign rekor-cli fulcio
            sqlite postgresql
            libp2p
            opa
            just
            direnv
            git
            curl jq yq
          ];
          RUST_BACKTRACE = 1;
          PYTHONPATH = "";
        };
        packages.default = pkgs.writeText "imperium-flake" "Run: nix develop";
      });
}