{ pkgs, ... }:
{
  rust-toolchain = pkgs.rust-bin.stable.latest.default;
  
  rust-components = [
    "rustfmt"
    "clippy"
    "rust-src"
    "rust-analysis"
  ];
  
  cargo-nextest = pkgs.cargo-nextest;
  
  cargo-audit = pkgs.cargo-audit;
  cargo-deny = pkgs.cargo-deny;
  cargo-tarpaulin = pkgs.cargo-tarpaulin;
}