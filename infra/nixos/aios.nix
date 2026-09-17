{ config, lib, pkgs, ... }:
{
  imports = [ ./cjk-console.nix ];

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;
  boot.loader.timeout = 3;
  boot.kernelModules = [ "kvm-amd" ];
  boot.extraModprobeConfig = ''
    options kvm_amd nested=1
  '';

  networking.hostName = "aios";
  networking.networkmanager.enable = true;
  networking.firewall.allowedTCPPorts = [ 22 2222 3000 7460 ];

  time.timeZone = "Asia/Shanghai";
  i18n.defaultLocale = "en_US.UTF-8";

  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PasswordAuthentication = true;
      PrintMotd = false;
      PrintLastLog = false;
    };
  };
  users.motd = "";
  environment.etc.motd.text = "";

  # Project-agreed host login; hash only.
  users.users.root.hashedPassword = "$6$VXUzKaFlgh2.HdHS$IQaYkYYn4oC0YFVHYXVrehUsryETuffiMUYUzpE9AApHe0rx1.DEIYFDNVzP4bfFYaHfkQH8ucZt9OBxiJcqw1";
  users.mutableUsers = true;

  hardware.enableRedistributableFirmware = true;
  virtualisation.libvirtd.enable = false;

  nix.settings = {
    experimental-features = [ "nix-command" "flakes" ];
    substituters = [
      "https://mirrors.ustc.edu.cn/nix-channels/store"
      "https://cache.nixos.org"
    ];
    trusted-public-keys = [
      "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
    ];
  };

  environment.systemPackages = with pkgs; [
    vim git tmux htop pciutils usbutils wget curl python3
    qemu_kvm socat minisign age
  ];

  system.stateVersion = "26.05";
}
