{ lib, ... }:
{
  perSystem =
    { pkgs, self', ... }:
    {
      packages.bomber-server = pkgs.rustPlatform.buildRustPackage {
        pname = "bomber-server";
        version = "0.1.0";

        # Only what the build actually needs. Keeping docs and asset drops out
        # of the source means editing a guide does not invalidate the build.
        src = lib.fileset.toSource {
          root = ../.;
          fileset = lib.fileset.unions [
            ../Cargo.toml
            ../Cargo.lock
            ../crates
            ../config
          ];
        };

        cargoLock.lockFile = ../Cargo.lock;
        cargoBuildFlags = [
          "-p"
          "bomber-server"
        ];
        cargoTestFlags = [ "--workspace" ];

        # Ship the annotated default config next to the binary. The server runs
        # without one, but a host wants something to copy and edit.
        postInstall = ''
          install -Dm644 config/server.toml \
            "$out/share/bomber-server/server.toml"
        '';

        meta = {
          description = "Authoritative 60 Hz Bomberman arena server for bot competitions";
          mainProgram = "bomber-server";
          platforms = lib.platforms.unix;
          license = lib.licenses.mit;
        };
      };

      packages.default = self'.packages.bomber-server;
    };
}
