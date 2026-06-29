{
  description = "Social Manager Dev Environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    {
      devShells.x86_64-linux.default =
        let
          pkgs = import nixpkgs { system = "x86_64-linux"; };
        in
        pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            pkg-config
            webkitgtk_4_1
            gtk3
            cairo
            glib
            openssl
            libsoup_3
            glib-networking
            nodejs
          ];
          PKG_CONFIG_PATH = "${pkgs.webkitgtk_4_1.dev}/lib/pkgconfig";
          shellHook = ''
            # ponytail: WebKit/GIO needs glib-networking's libgiognutls.so for TLS.
            export GIO_EXTRA_MODULES="${pkgs.glib-networking}/lib/gio/modules''${GIO_EXTRA_MODULES:+:$GIO_EXTRA_MODULES}";
          '';
        };
    };
}
