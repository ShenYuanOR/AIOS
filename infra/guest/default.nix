{ config, lib, pkgs, ... }:
{
  networking.hostName = "room";
  services.openssh.enable = true;
  services.openssh.settings.PermitRootLogin = "yes";

  users.users.root.password = "root";
  users.mutableUsers = true;

  environment.systemPackages = with pkgs; [
    git vim tmux curl wget
    gcc gnumake pkg-config
    python3 rustc cargo go nodejs
  ];

  microvm = {
    hypervisor = "qemu";
    vcpu = 4;
    mem = 2048;
    shares = [{
      source = "/nix/store";
      mountPoint = "/nix/.ro-store";
      tag = "ro-store";
      proto = "virtiofs";
    }];
    interfaces = [{
      type = "user";
      id = "room0";
      mac = "02:00:00:00:00:01";
    }];
  };

  system.stateVersion = "26.05";
}
