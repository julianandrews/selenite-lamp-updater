{
  description = "selenite-lamp-updater — schedule and count driven updater for the selenite lamp";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs =
    { self, nixpkgs }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAllSystems (
        pkgs:
        let
          cargo = pkgs.lib.importTOML ./Cargo.toml;
          selenite-lamp-updater = pkgs.rustPlatform.buildRustPackage {
            pname = cargo.package.name;
            version = cargo.package.version;
            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;

              # selenite-lamp is pulled from GitHub in Cargo.lock. Recompute this
              # with `nix hash path` on a clean checkout whenever the lock moves.
              outputHashes."selenite-lamp-0.2.4" = "sha256-ZnqSXmkL/+QYmkayeZPUwZVrFKqrT8xYJD7isXLoVTU=";
            };

            # serialport (through selenite-lamp) links against libudev on Linux.
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.udev ];

            meta = {
              description = cargo.package.description;
              homepage = "https://github.com/julianandrews/selenite-lamp-updater";
              license = pkgs.lib.licenses.bsd3;
              mainProgram = "selenite-lamp-updater";
              platforms = pkgs.lib.platforms.linux;
            };
          };
        in
        {
          inherit selenite-lamp-updater;
          default = selenite-lamp-updater;
        }
      );

      checks = forAllSystems (pkgs: {
        default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          name = "selenite-lamp-updater";
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
            pkg-config
            udev
          ];
        };
      });
    };
}
