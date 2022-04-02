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
          packages.disruption = mkPackage "disruption";
          apps.disruption = utils.lib.mkApp { drv = packages.disruption; };
          devShell = pkgs.mkShell {
            nativeBuildInputs = [ pkgs.rustc pkgs.cargo pkgs.rust-analyzer ];
          };
        });
    in packages // {
      nixosModule = { config, lib, pkgs, ... }:
        with lib;
        let
          cfg = config.services.disruption;
          pkg = self.packages.${pkgs.system}.disruption;
          tomlConfigPath = (pkgs.formats.toml { }).generate "config.toml" ({
            interval_milliseconds = cfg.intervalMs;
            state_file_path = "/var/lib/disruption/state.json";
            subscriptions = builtins.map ({ branches, discordWebhookUrl }: {
              inherit branches;
              discord_webhook_url = discordWebhookUrl;
            }) cfg.subscriptions;
          });
        in {
          options.services.disruption = {
            enable = mkEnableOption "disruption";

            intervalMs = mkOption {
              type = types.ints.positive;
              default = 1000 * 60 * 5;
              example = "300000";
              description =
                "Milliseconds to sleep between scrapes. Please be courteous and set this to a high value.";
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

          config.systemd = mkIf cfg.enable {
            services.disruption = rec {
              environment = { RUST_LOG = "warn,havoc=debug,disruption=debug"; };
              serviceConfig = {
                User = "disruption";
                Group = "disruption";
                DynamicUser = true;
                StateDirectory = "disruption";
              };
              after = [ "network-online.target" ];
              wantedBy = [ "network-online.target" ];
              script = "${pkg}/bin/disruption ${tomlConfigPath}";
            };
          };
        };
    };
}
