{ self, lib, ... }:
{
  flake.nixosModules.bomberman-arena =
    {
      config,
      pkgs,
      lib,
      ...
    }:
    let
      cfg = config.services.bomberman-arena;
      format = pkgs.formats.toml { };

      settings = lib.recursiveUpdate {
        server = {
          bind = "${cfg.listenAddress}:${toString cfg.udpPort}";
          web_bind = "${cfg.webAddress}:${toString cfg.webPort}";
        } // lib.optionalAttrs (cfg.visualizerRoot != null) {
          static_dir = cfg.visualizerRoot;
        };
      } cfg.settings;

      configFile = format.generate "bomberman-server.toml" settings;
    in
    {
      options.services.bomberman-arena = {
        enable = lib.mkEnableOption "the Bomberman arena server";

        package = lib.mkOption {
          type = lib.types.package;
          default = self.packages.${pkgs.stdenv.hostPlatform.system}.bomber-server;
          defaultText = lib.literalMD "the flake's `bomber-server`";
          description = "Which build of the server to run.";
        };

        listenAddress = lib.mkOption {
          type = lib.types.str;
          default = "0.0.0.0";
          description = ''
            Address the bot-facing UDP socket binds to. Bots are usually on
            other machines, so this defaults to every interface.
          '';
        };

        udpPort = lib.mkOption {
          type = lib.types.port;
          default = 47800;
          description = "UDP port bots send their two bytes to.";
        };

        webAddress = lib.mkOption {
          type = lib.types.str;
          default = "127.0.0.1";
          description = ''
            Address the web interface binds to.

            Defaults to loopback deliberately. The moderation WebSocket has **no
            authentication**: anyone who can reach it can start, pause and end
            matches and kick players. Put it behind a reverse proxy with auth
            before exposing it, rather than changing this to 0.0.0.0.
          '';
        };

        webPort = lib.mkOption {
          type = lib.types.port;
          default = 8080;
          description = "TCP port for the visualizer and moderation UI.";
        };

        visualizerRoot = lib.mkOption {
          type = lib.types.nullOr lib.types.path;
          default = null;
          example = lib.literalExpression "./visualizer-dist";
          description = ''
            Directory holding a built visualizer, served at `/`. When null the
            server serves a placeholder page naming the endpoints.
          '';
        };

        openFirewall = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = ''
            Open the UDP port for bots. The web port is never opened here --
            see `webAddress`.
          '';
        };

        settings = lib.mkOption {
          inherit (format) type;
          default = { };
          example = lib.literalExpression ''
            {
              lobby.min_players = 4;
              rules.round_time_secs = 300;
              map = { width = 21; height = 17; symmetry = "quad"; };
            }
          '';
          description = ''
            Contents of `server.toml`. The `[server]` bind addresses are filled
            in from the options above; everything else is yours. See
            `''${pkgs.bomber-server}/share/bomber-server/server.toml` for the
            annotated defaults.
          '';
        };
      };

      config = lib.mkIf cfg.enable {
        systemd.services.bomberman-arena = {
          description = "Bomberman arena server";
          wantedBy = [ "multi-user.target" ];
          after = [ "network.target" ];

          serviceConfig = {
            ExecStart = "${lib.getExe cfg.package} --config ${configFile}";
            Restart = "on-failure";
            RestartSec = 2;

            DynamicUser = true;
            StateDirectory = "bomberman-arena";
            WorkingDirectory = "/var/lib/bomberman-arena";

            # The server reads one config file, binds two sockets and touches
            # nothing else, so it can be locked down hard.
            NoNewPrivileges = true;
            PrivateTmp = true;
            PrivateDevices = true;
            ProtectSystem = "strict";
            ProtectHome = true;
            ProtectKernelTunables = true;
            ProtectKernelModules = true;
            ProtectControlGroups = true;
            RestrictAddressFamilies = [
              "AF_INET"
              "AF_INET6"
            ];
            RestrictNamespaces = true;
            LockPersonality = true;
            MemoryDenyWriteExecute = true;
            SystemCallArchitectures = "native";
            SystemCallFilter = [ "@system-service" ];
          };
        };

        networking.firewall = lib.mkIf cfg.openFirewall {
          allowedUDPPorts = [ cfg.udpPort ];
        };
      };
    };

  # `nixosModules.default` so `imports = [ bomberman.nixosModules.default ];`
  # works without anyone having to look up the name.
  flake.nixosModules.default = self.nixosModules.bomberman-arena;
}
