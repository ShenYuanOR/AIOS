{
  description = "AIOS: NixOS + motherd + signed plugins + per-project microVM";

  nixConfig = {
    extra-substituters = [
      "https://mirrors.ustc.edu.cn/nix-channels/store"
      "https://cache.nixos.org"
    ];
    extra-trusted-public-keys = [
      "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
    ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    microvm = {
      url = "github:astro/microvm.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, microvm }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      nixosModules.mother = import ./infra/nixos/mother-host.nix;
      nixosModules.aiosHost = import ./infra/nixos/aios.nix;

      nixosConfigurations.aios = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          ./infra/nixos/hardware-aios.nix
          ./infra/nixos/aios.nix
          self.nixosModules.mother
          {
            aios.mother.guestRunner =
              self.nixosConfigurations.guest.config.microvm.declaredRunner;
          }
        ];
      };

      nixosConfigurations.guest = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          microvm.nixosModules.microvm
          ./infra/guest/default.nix
        ];
      };

      nixosConfigurations.iso = nixpkgs.lib.nixosSystem {
        inherit system;
        modules = [
          "${nixpkgs}/nixos/modules/installer/cd-dvd/installation-cd-minimal.nix"
          ./infra/iso/default.nix
          self.nixosModules.mother
        ];
      };

      packages.${system} = {
        motherd = pkgs.callPackage ./motherd/package.nix { };
        default = self.packages.${system}.motherd;
        iso = self.nixosConfigurations.iso.config.system.build.isoImage;
        guest-runner = self.nixosConfigurations.guest.config.microvm.declaredRunner;
        sign-iso = pkgs.writeShellScriptBin "aios-sign-iso" ''
          set -euo pipefail
          iso=''${1:?usage: aios-sign-iso <iso-file> [sig-file]}
          pub=/var/lib/motherd/keys/plugins.pub
          sec=/var/lib/motherd/keys/plugins.sec
          test -f "$sec" || { echo "missing $sec (bootstrap motherd first)"; exit 1; }
          sig=''${2:-}
          if [ -z "$sig" ]; then
            dir=$(dirname "$iso")
            if [ -w "$dir" ]; then
              sig="$iso.minisig"
            else
              sig="./$(basename "$iso").minisig"
            fi
          fi
          ${pkgs.minisign}/bin/minisign -S -s "$sec" -m "$iso" -x "$sig" -t aios-iso
          echo "signed $sig  pubkey $pub"
        '';
      };

      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [ rustc cargo git tmux minisign ];
      };
    };
}
