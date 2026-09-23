{
  perSystem =
    { pkgs, ... }:
    {
      devShells.default = pkgs.mkShell {
        packages = with pkgs; [
          # Rust: server, simulation, protocol, bot SDK.
          # nixpkgs-unstable rustc is recent enough that no rust-overlay input is needed.
          rustc
          cargo
          clippy
          rustfmt
          rust-analyzer

          # Visualizer.
          nodejs_22
          pnpm

          # Python bot SDK (stdlib only, but authors want a REPL).
          python3

          # Task running and poking the UDP port by hand.
          just
          watchexec
          socat
        ];

        env = {
          RUST_BACKTRACE = "1";
          # rust-analyzer needs the source of the stdlib to resolve std:: symbols.
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
        };

        shellHook = ''
          echo "bomberman dev shell -- run 'just' to list tasks"
        '';
      };
    };
}
