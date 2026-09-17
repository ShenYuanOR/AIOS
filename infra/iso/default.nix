{ config, lib, pkgs, ... }:
{
  imports = [ ../nixos/cjk-console.nix ];

  isoImage.isoBaseName = lib.mkForce "aios";
  isoImage.volumeID = "AIOS";
  isoImage.makeEfiBootable = true;
  isoImage.makeUsbBootable = true;

  networking.hostName = "aios";
  services.openssh.enable = true;
  services.openssh.settings.PermitRootLogin = "yes";

  environment.systemPackages = with pkgs; [ vim git tmux minisign age qemu_kvm ];

  # Product image: motherd self-starts; first-boot bootstrap is a kernel syscall.
  aios.mother.enable = true;

  isoImage.squashfsCompression = "zstd -Xcompression-level 6";
}
