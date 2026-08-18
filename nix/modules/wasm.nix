{ pkgs, ... }:
{
  wasmtime = pkgs.wasmtime;
  wasi-sdk = pkgs.wasi-sdk;
  
  wasm-tools = with pkgs; [
    wasmtime
    wasi-sdk
    wit-bindgen
  ];
}