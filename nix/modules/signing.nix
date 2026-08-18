{ pkgs, ... }:
{
  cosign = pkgs.cosign;
  rekor-cli = pkgs.rekor-cli;
  fulcio = pkgs.fulcio;
  
  slsa-tools = with pkgs; [
    slsa-verifier
  ];
}