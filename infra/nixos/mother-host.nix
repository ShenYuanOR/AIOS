{ config, lib, pkgs, ... }:
let
  cfg = config.aios.mother;
  motherd = pkgs.callPackage ../../motherd/package.nix { };
  motherctlBin = "${motherd}/bin/motherctl";
  # Stable login-shell path; also listed in /etc/shells via systemPackages.
  aiosLoginShell = "/run/current-system/sw/bin/motherctl";
  pluginSrc = pkgs.runCommand "aios-plugins" { } ''
    mkdir -p $out
    cp -a ${../../plugins}/. $out/
  '';
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
    guestRunner = lib.mkOption {
      type = lib.types.nullOr lib.types.package;
      default = null;
      description = "microvm declaredRunner for /spawn (shared nix store).";
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

    # Living-room identity. root keeps an engineering shell so rebuild/SFTP still work.
    users.users.aios = {
      isNormalUser = true;
      group = "users";
      extraGroups = [ "kvm" "mother" ];
      initialPassword = "root";
      shell = aiosLoginShell;
      ignoreShellProgramCheck = true;
    };
    environment.shells = lib.mkAfter [ aiosLoginShell ];
    # mutableUsers may keep an old bash in passwd; force the login shell.
    system.activationScripts.aiosLoginShell = {
      deps = [ "users" "etc" ];
      text = ''
        if getent passwd aios >/dev/null; then
          want=${aiosLoginShell}
          cur=$(getent passwd aios | cut -d: -f7)
          if [ "$cur" != "$want" ]; then
            ${pkgs.shadow}/bin/usermod -s "$want" aios || true
          fi
        fi
      '';
    };

    systemd.tmpfiles.rules = [
      "d ${cfg.dataDir} 0750 root mother -"
      "d ${cfg.dataDir}/keys 0700 root mother -"
      "d ${cfg.dataDir}/leases 0750 root mother -"
      "d ${cfg.dataDir}/vault 0700 root mother -"
      "d ${cfg.dataDir}/vault-in 0770 root mother -"
      "f ${cfg.dataDir}/model-alias.json 0660 root mother - {\"chat-mother\":\"grok\",\"chat-code\":\"gpt\",\"cheap\":\"deepseek\"}"
      "d ${cfg.dataDir}/audit 0750 root mother -"
      "d ${cfg.dataDir}/plugins 0755 root mother -"
      "d ${cfg.dataDir}/sessions 0770 root mother -"
      "d ${cfg.dataDir}/sessions/aios 0770 root mother -"
      "d ${cfg.dataDir}/sessions/aios/active 0770 root mother -"
      "d ${cfg.dataDir}/rooms 0750 root mother -"
      "d ${cfg.dataDir}/letta 0750 root mother -"
      "d /run/motherd 0770 root mother -"
    ];

    environment.systemPackages = [ motherd pkgs.minisign pkgs.age pkgs.qemu_kvm pkgs.socat pkgs.iproute2 ];

    environment.variables.AIOS_PLUGIN_SRC = "${pluginSrc}";

    systemd.services.motherd = {
      description = "AIOS motherd dead core";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];
      path = with pkgs; [ minisign coreutils qemu_kvm systemd iproute2 ];
      serviceConfig = {
        ExecStart = "${motherd}/bin/motherd --data ${cfg.dataDir}";
        User = "root";
        Group = "mother";
        UMask = "0007";
        Restart = "always";
        RestartSec = 2;
        Environment = [
          "AIOS_PLUGIN_SRC=${pluginSrc}"
        ] ++ lib.optional (cfg.guestRunner != null)
          "AIOS_GUEST_RUNNER=${cfg.guestRunner}/bin";
      };
    };

    networking.firewall.allowedTCPPorts = [ 2222 ];
    services.openssh.settings.PrintMotd = lib.mkDefault false;
    services.openssh.settings.PrintLastLog = lib.mkDefault false;
    users.motd = lib.mkDefault "";
    # sshd runs this as `$SHELL -c ForceCommand`. aios SHELL is motherctl,
    # so motherctl -c of this binary must enter the REPL (ctl.rs).
    services.openssh.extraConfig = ''
      Match User aios
        ForceCommand ${motherctlBin}
        PermitTTY yes
    '';
  };
}
