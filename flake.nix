{
  description = "Discord client instrumentation toolkit";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk.url = "github:nix-community/naersk";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, naersk, utils }:
    let
      packages = utils.lib.eachDefaultSystem (system:
        let
          pkgs = nixpkgs.legacyPackages."${system}";
          naersk-lib = naersk.lib."${system}";
          naerskBuildPackage = args:
            naersk-lib.buildPackage (args // {
              nativeBuildInputs = [ pkgs.pkg-config pkgs.openssl ]
                ++ nixpkgs.lib.optional pkgs.stdenv.isDarwin [
                  # needed by curl-sys on darwin
                  pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
                ];
            });
          mkPackage = n:
            naerskBuildPackage {
              pname = n;
              root = ./.;
              targets = [ n ];
            };
        in rec {
          packages.havoc = mkPackage "havoc";
          packages.watchdog = mkPackage "watchdog";
          apps.watchdog = utils.lib.mkApp { drv = packages.watchdog; };
          devShell = pkgs.mkShell {
            nativeBuildInputs = [ pkgs.rustc pkgs.cargo pkgs.rust-analyzer ];
          };
        });
    in packages // {
      nixosModule = { config, lib, pkgs, ... }:
        with lib;
        let
          cfg = config.services.watchdog;
          pkg = self.packages.${pkgs.system}.watchdog;
          localDatabase = cfg.postgresUrl == "local";
          tomlConfigPath = (pkgs.formats.toml { }).generate "config.toml" ({
            interval_milliseconds = cfg.intervalMs;
            http_api_server_bind_address = cfg.bind;
            postgres.url = if localDatabase then
              "postgres://${cfg.user}@localhost/${cfg.localDatabaseName}"
            else
              cfg.databaseUrl;
            subscriptions = builtins.map ({ branches, discordWebhookUrl }: {
              inherit branches;
              discord_webhook_url = discordWebhookUrl;
            }) cfg.subscriptions;
          });
        in {
          options.services.watchdog = {
            enable = mkEnableOption "watchdog";

            intervalMs = mkOption {
              type = types.ints.positive;
              default = 1000 * 60 * 5;
              example = "300000";
              description = mdDoc
                "Milliseconds to sleep between scrapes. Please be courteous and set this to a high value.";
            };

            user = mkOption {
              type = types.str;
              default = "watchdog";
              description = mdDoc ''
                The user for watchdog to run under.

                This is also used as the name of the Postgres user, if the
                database is being created locally.
              '';
            };

            group = mkOption {
              type = types.str;
              default = "watchdog";
              description = mdDoc "The group for watchdog to run under.";
            };

            bind = mkOption {
              type = types.str;
              default = "127.0.0.1:6700";
              description = mdDoc ''
                The address to bind the HTTP API server to.
              '';
            };

            postgresUrl = mkOption {
              type = types.str;
              default = "local";
              description = mdDoc ''
                The URL of the Postgres database to connect to. Pass `local`
                to create the database locally.
              '';
            };

            localDatabaseName = mkOption {
              type = types.str;
              default = "watchdog";
              description = mdDoc ''
                The name of the Postgres database to create, when creating
                locally.
              '';
            };

            subscriptions = with types;
              mkOption {
                description = "Subscriptions to scrape targets.";
                type = listOf (submodule {
                  options = {
                    branches = mkOption {
                      type = listOf (enum [ "stable" "canary" "ptb" ]);
                      description =
                        "The branches that this subscription is interested in.";
                    };
                    discordWebhookUrl = mkOption {
                      type = str;
                      description = "The Discord webhook URL to post to.";
                    };
                  };
                });
              };
          };

          config = mkIf cfg.enable {
            systemd = {
              services.watchdog = rec {
                environment = { RUST_LOG = "warn,havoc=debug,watchdog=debug"; };
                serviceConfig = {
                  User = cfg.user;
                  Group = cfg.group;
                  StateDirectory = "watchdog";
                };
                after = [ "network-online.target" ]
                  ++ (optional localDatabase "postgresql.service");
                wantedBy = [ "network-online.target" ];
                script = "${pkg}/bin/watchdog ${tomlConfigPath}";
              };
            };

            users = {
              users = mkIf (cfg.user == "watchdog") {
                watchdog = {
                  group = cfg.group;
                  isSystemUser = true;
                };
              };

              groups = mkIf (cfg.group == "watchdog") { watchdog = { }; };
            };

            services.postgresql = mkIf (cfg.enable && localDatabase) {
              enable = mkDefault true;
              authentication = ''
                local ${cfg.localDatabaseName} ${cfg.user} trust
              '';
              ensureDatabases = [ cfg.localDatabaseName ];
              ensureUsers = [{
                name = cfg.user;
                ensurePermissions."DATABASE \"${cfg.localDatabaseName}\"" =
                  "ALL PRIVILEGES";
              }];
            };
          };
        };
    };
}
