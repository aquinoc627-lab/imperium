{ pkgs, ... }:
{
  nodejs = pkgs.nodejs_20;
  
  pnpm = pkgs.pnpm;
  
  node-packages = with pkgs; [
    typescript
    vite
    vitest
    eslint
    prettier
  ];
}