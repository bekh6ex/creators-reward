{
  description = "Creator's reward development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config.allowUnfree = true;
        };

        fontsConf = pkgs.makeFontsConf {
          fontDirectories = [ pkgs.dejavu_fonts ];
        };

        playwrightDeps = with pkgs; [
          glib
          nspr
          nss
          at-spi2-core
          cups.lib
          dbus.lib
          expat
          libx11
          libxcomposite
          libxdamage
          libxext
          libxfixes
          libxrandr
          libxcb
          libxshmfence
          mesa
          libgbm
          libGL
          libdrm
          vulkan-loader
          gtk3
          pango.out
          cairo
          gdk-pixbuf
          libxkbcommon
          fontconfig.lib
          freetype
          systemd
          alsa-lib
          wayland
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          name = "creators-reward";
          hardeningDisable = [ "fortify" ];

          nativeBuildInputs = with pkgs.buildPackages; [
            starship
            prek
            gitleaks
            reuse
            ast-grep
            just
            typescript-language-server
            oxlint
            nodejs_24
            pnpm
            zlib
            procps
            caddy

            rustup
            rustc
            cargo
            cargo-shear
            cargo-deny
            cargo-vet
            cargo-llvm-cov
            cargo-watch
            cargo-mutants
            rust-analyzer

            util-linux
            iproute2
            openssl
            jq
          ] ++ playwrightDeps;

          shellHook = ''
            eval "$(starship init bash)"
            export LD_LIBRARY_PATH="${pkgs.zlib}/lib:${pkgs.zlib.static}/lib:$LD_LIBRARY_PATH"
            export FONTCONFIG_FILE=${fontsConf}


          '';
        };
      }
    );
}
