{
  description = "forjj - A native jj forge";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, crane }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Use Rust from rust-toolchain.toml
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        # Crane for building Rust projects
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Common arguments for crane builds
        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          pname = "forjj";
          version = "0.1.0";

          buildInputs = with pkgs; [
            openssl
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.apple-sdk_15
            pkgs.libiconv
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
        };

        # Build dependencies separately for caching
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the main package
        forjj = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
        });

      in {
        # `nix build`
        packages = {
          default = forjj;
          forjj = forjj;
        };

        # `nix run`
        apps.default = flake-utils.lib.mkApp {
          drv = forjj;
        };

        # `nix flake check`
        checks = {
          inherit forjj;

          # Run clippy
          forjj-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          # Run tests
          forjj-test = craneLib.cargoTest (commonArgs // {
            inherit cargoArtifacts;
          });

          # Check formatting
          forjj-fmt = craneLib.cargoFmt {
            src = craneLib.cleanCargoSource ./.;
          };
        };

        # `nix develop`
        devShells.default = craneLib.devShell {
          # Inherit inputs from checks
          checks = self.checks.${system};

          # Extra packages for development
          packages = with pkgs; [
            # Rust tooling (provided by crane devShell via rustToolchain)
            cargo-watch
            cargo-edit
            cargo-outdated
            cargo-audit
            cargo-nextest

            # Development tools
            just
            watchexec

            # Git tooling
            git
            jujutsu  # jj itself for testing

            # Protobuf (for future jj native format work)
            protobuf

            # Database tools (for future use)
            sqlite
          ];

          # Shell hook for nice developer experience
          shellHook = ''
            echo "🔨 forjj development environment"
            echo ""
            echo "Available commands:"
            echo "  cargo build    - Build the project"
            echo "  cargo test     - Run tests"
            echo "  cargo watch    - Watch for changes and rebuild"
            echo "  cargo nextest  - Run tests with nextest"
            echo "  jj             - Jujutsu VCS (for testing)"
            echo ""
            echo "Nix commands:"
            echo "  nix build      - Build release binary"
            echo "  nix flake check - Run all checks (clippy, tests, fmt)"
            echo ""
          '';

          # Environment variables
          RUST_BACKTRACE = "1";
          RUST_LOG = "forjj=debug";
        };
      }
    );
}
