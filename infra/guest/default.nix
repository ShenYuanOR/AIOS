{ config, lib, pkgs, ... }:
{
  networking.hostName = "room";
  networking.extraHosts = ''
    10.0.2.2 aios
    10.0.2.2 gitea.aios
  '';
  services.openssh.enable = true;
  services.openssh.settings.PermitRootLogin = "yes";
  networking.firewall.allowedTCPPorts = [ 22 ];

  users.users.root.password = "root";
  users.mutableUsers = true;

  programs.git.enable = true;
  programs.git.config = {
    init = { defaultBranch = "main"; };
    user = { name = "room"; email = "room@aios.local"; };
  };

  environment.systemPackages = with pkgs; [
    git vim tmux curl wget
    gcc gnumake pkg-config
    python3 rustc cargo go nodejs
  ];

  environment.variables.AIOS_GITEA = "http://aios:3000";

  microvm = {
    hypervisor = "qemu";
    vcpu = 4;
    # qemu microvm + virtiofs hangs at exactly 2048M (ACPI tables)
    mem = 2176;
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
    forwardPorts = [{
      from = "host";
      proto = "tcp";
      host.address = "0.0.0.0";
      host.port = 2222;
      guest.port = 22;
    }];
  };

  system.stateVersion = "26.05";
}
