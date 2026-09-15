{ pkgs }:
pkgs.writeShellScriptBin "aios-ci-gate" ''
  set -euo pipefail
  nix flake check
  nix build .#motherd
''
