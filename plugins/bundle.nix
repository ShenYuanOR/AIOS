{ config, lib, pkgs, ... }:
let
  plug = ../plugins;
  py = pkgs.python3.interpreter;
  giteaPw = "root";
in {
  virtualisation.podman.enable = true;
  virtualisation.oci-containers.backend = "podman";

  # Official Letta (classic sleep-time server). LAN must not see this port.
  virtualisation.oci-containers.containers.aios-letta = {
    image = "docker.m.daocloud.io/letta/letta:0.16.8";
    extraOptions = [ "--pull=newer" "--env-file=/var/lib/motherd/letta.env" ];
    ports = [ "127.0.0.1:8283:8283" ];
    volumes = [
      "/var/lib/motherd/letta/pgdata:/var/lib/postgresql/data"
    ];
    autoStart = true;
  };

  systemd.tmpfiles.rules = [
    "d /var/lib/motherd/letta 0750 root root -"
    "d /var/lib/motherd/letta/pgdata 0750 root root -"
    "f /var/lib/motherd/letta.env 0600 root root -"
  ];

  services.gitea = {
    enable = true;
    lfs.enable = true;
    settings = {
      server = {
        HTTP_ADDR = "0.0.0.0";
        HTTP_PORT = 3000;
        DOMAIN = "192.168.110.99";
        ROOT_URL = "http://192.168.110.99:3000/";
        SSH_DOMAIN = "192.168.110.99";
      };
      service.DISABLE_REGISTRATION = true;
    };
  };

  systemd.services.aios-gitea-bootstrap = {
    description = "AIOS product Gitea admin + hello repo";
    wantedBy = [ "multi-user.target" ];
    after = [ "gitea.service" ];
    requires = [ "gitea.service" ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
    };
    path = [ pkgs.curl pkgs.coreutils pkgs.gnused pkgs.gawk pkgs.util-linux ];
    script = ''
      set -euo pipefail
      for i in $(seq 1 90); do
        if curl -sf http://127.0.0.1:3000/ >/dev/null; then
          break
        fi
        sleep 2
      done
      cfg=/var/lib/gitea/custom/conf/app.ini
      runuser -u gitea -- env GITEA_WORK_DIR=/var/lib/gitea GITEA_CUSTOM=/var/lib/gitea/custom \
        ${config.services.gitea.package}/bin/gitea admin user create \
        --config "$cfg" \
        --admin --username ShenYuan --password ${giteaPw} \
        --email shenyuan@aios.local --must-change-password=false \
        || true
      curl -sf -u ShenYuan:${giteaPw} \
        -H 'Content-Type: application/json' \
        -d '{"name":"hello","private":false,"auto_init":true,"default_branch":"main"}' \
        http://127.0.0.1:3000/api/v1/user/repos \
        >/dev/null || true
    '';
  };

  systemd.services.aios-letta-env = {
    description = "Write Letta env from motherd vault";
    wantedBy = [ "multi-user.target" ];
    after = [ "motherd.service" ];
    before = [ "podman-aios-letta.service" ];
    serviceConfig.Type = "oneshot";
    serviceConfig.RemainAfterExit = true;
    path = [ pkgs.python3 pkgs.coreutils ];
    script = ''
      ${py} ${plug}/letta/boot_env.py || true
    '';
  };

  systemd.services.aios-mux = {
    description = "AIOS mux/SKILL plugin";
    wantedBy = [ "multi-user.target" ];
    after = [ "motherd.service" ];
    serviceConfig.ExecStart = "${py} ${plug}/mux-skill/mux.py";
    serviceConfig.Restart = "always";
  };
  systemd.services.aios-cockpit = {
    description = "AIOS cockpit web plugin";
    wantedBy = [ "multi-user.target" ];
    after = [ "motherd.service" ];
    serviceConfig.ExecStart = "${py} ${plug}/cockpit-web/server.py";
    serviceConfig.Restart = "always";
  };
  systemd.services.aios-metrics = {
    description = "AIOS metrics export plugin";
    wantedBy = [ "multi-user.target" ];
    after = [ "motherd.service" ];
    serviceConfig.ExecStart = "${py} ${plug}/metrics-export/export.py";
    serviceConfig.Restart = "always";
  };
  systemd.services.aios-letta-plugin = {
    description = "AIOS Letta stingy-ingest plugin (talks to official Letta)";
    wantedBy = [ "multi-user.target" ];
    after = [ "podman-aios-letta.service" "motherd.service" ];
    serviceConfig.ExecStart = "${py} ${plug}/letta/shim.py";
    serviceConfig.Restart = "always";
  };

  environment.etc."aios/plugins/model-router".source = plug + "/model-router";
  environment.etc."aios/plugins/mother-nl".source = plug + "/mother-nl";
  environment.etc."aios/plugins/archive".source = plug + "/archive";
  environment.etc."aios/plugins/letta".source = plug + "/letta";
}
