{ pkgs, ... }:
{
  # Physical TTY: kernel vgacon/fbcon cannot draw CJK.
  # nixpkgs Noto CJK is a variable-font TTC; kmscon's freetype path
  # loads the family name but rasterizes tofu. Sarasa Term SC is static.
  i18n.supportedLocales = [
    "C.UTF-8/UTF-8"
    "en_US.UTF-8/UTF-8"
    "zh_CN.UTF-8/UTF-8"
  ];

  fonts = {
    fontconfig.enable = true;
    fontconfig.defaultFonts = {
      serif = [
        "Noto Serif CJK SC"
        "Noto Serif"
      ];
      sansSerif = [
        "Noto Sans CJK SC"
        "Noto Sans"
      ];
      monospace = [
        "Sarasa Term SC"
        "Noto Sans Mono CJK SC"
        "Noto Sans Mono"
      ];
      emoji = [ "Noto Color Emoji" ];
    };
    packages = with pkgs; [
      sarasa-gothic
      noto-fonts
      noto-fonts-cjk-sans
      noto-fonts-cjk-serif
      noto-fonts-color-emoji
    ];
  };

  services.kmscon = {
    enable = true;
    hwRender = true;
    fonts = [
      {
        name = "Sarasa Term SC";
        package = pkgs.sarasa-gothic;
      }
    ];
    extraConfig = ''
      font-engine=pango
      font-size=16
    '';
  };
}
