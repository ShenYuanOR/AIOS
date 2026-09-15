{
  description = "AIOS: NixOS + motherd + signed plugins + per-project microVM";

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
      };

      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [ rustc cargo git tmux ];
      };
    };
}
