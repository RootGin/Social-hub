{
  description = "Social Manager Dev Environment";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }: {
    devShells.x86_64-linux.default = let
      pkgs = import nixpkgs { system = "x86_64-linux"; };
    in pkgs.mkShell {
      buildInputs = with pkgs; [
        cargo rustc pkg-config
        webkitgtk_4_1 gtk3 cairo glib openssl
        nodejs  # Cần cho phần frontend
      ];
      # Giúp Tauri tìm thấy thư viện WebKit
      PKG_CONFIG_PATH = "${pkgs.webkitgtk_4_1.dev}/lib/pkgconfig";
    };
  };
}