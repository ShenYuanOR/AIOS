{ config, lib, pkgs, ... }:
let
  plug = ../plugins;
  py = pkgs.python3.interpreter;
in {
  services.gitea = {
    enable = true;
    lfs.enable = true;
    settings = {
      server = {
        HTTP_ADDR = "0.0.0.0";
        HTTP_PORT = 3000;
        DOMAIN = "aios";
        ROOT_URL = "http://aios:3000/";
      };
      service.DISABLE_REGISTRATION = false;
    };
  };

  systemd.services.aios-mux = {
    description = "AIOS mux/SKILL plugin";
    wantedBy = [ "multi-user.target" ];
    after = [ "motherd.service" ];
    serviceConfig.ExecStart = "${py} ${plug}/mux-skill/mux.py";
  };
  systemd.services.aios-cockpit = {
    description = "AIOS cockpit web plugin";
    wantedBy = [ "multi-user.target" ];
    after = [ "motherd.service" ];
    serviceConfig.ExecStart = "${py} ${plug}/cockpit-web/server.py";
  };
  systemd.services.aios-metrics = {
    description = "AIOS metrics export plugin";
    wantedBy = [ "multi-user.target" ];
    after = [ "motherd.service" ];
    serviceConfig.ExecStart = "${py} ${plug}/metrics-export/export.py";
  };
  systemd.services.aios-letta = {
    description = "AIOS Letta memory plugin";
    wantedBy = [ "multi-user.target" ];
    after = [ "motherd.service" ];
    serviceConfig.ExecStart = "${py} ${plug}/letta/shim.py";
  };

  environment.etc."aios/plugins/model-router".source = plug + "/model-router";
  environment.etc."aios/plugins/mother-nl".source = plug + "/mother-nl";
  environment.etc."aios/plugins/archive".source = plug + "/archive";
}
