{ pkgs, ... }:
{
  python = pkgs.python312;
  
  uv = pkgs.python312Packages.uv;
  
  python-packages = with pkgs.python312Packages; [
    pytest
    pytest-asyncio
    pytest-benchmark
    pytest-cov
    ruff
    mypy
    pre-commit
    hypothesis
    faker
  ];
  
  python-ml-packages = with pkgs.python312Packages; [
    torch
    transformers
    sentence-transformers
    scikit-learn
    numpy
    pandas
    pyro-ppl
  ];
}