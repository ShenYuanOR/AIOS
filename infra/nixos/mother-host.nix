{ config, lib, pkgs, ... }:
let
  cfg = config.aios.mother;
  motherd = pkgs.callPackage ../../motherd/package.nix { };
in {
  options.aios.mother = {
    enable = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Run motherd on this host (first mother = aios).";
    };
    dataDir = lib.mkOption {
      type = lib.types.path;
      default = "/var/lib/motherd";
    };
  };

  imports = [ ../../plugins/bundle.nix ];

  config = lib.mkIf cfg.enable {
    users.groups.mother = { };
    users.users.mother = {
      isSystemUser = true;
      group = "mother";
      extraGroups = [ "kvm" ];
    };

    systemd.tmpfiles.rules = [
      "d ${cfg.dataDir} 0750 mother mother -"
      "d ${cfg.dataDir}/keys 0700 mother mother -"
      "d ${cfg.dataDir}/leases 0750 mother mother -"
      "d ${cfg.dataDir}/vault 0700 mother mother -"
      "d ${cfg.dataDir}/audit 0750 mother mother -"
      "d ${cfg.dataDir}/plugins 0755 mother mother -"
      "d ${cfg.dataDir}/rooms 0750 mother mother -"
      "d /run/motherd 0755 mother mother -"
    ];

    environment.systemPackages = [ motherd pkgs.minisign pkgs.age pkgs.qemu_kvm ];

    systemd.services.motherd = {
      description = "AIOS motherd dead core";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];
      serviceConfig = {
        ExecStart = "${motherd}/bin/motherd --data ${cfg.dataDir}";
        User = "root";
        Restart = "always";
        RestartSec = 2;
      };
    };
  };
}
