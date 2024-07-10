{ pkgs ? import <nixos> { } }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs.buildPackages; [
    pkg-config
    gtk4
    (gtk4-layer-shell.overrideAttrs (oldAttrs: {
      nativeBuildInputs = oldAttrs.nativeBuildInputs ++ [ wayland-protocols ];
    }))
    pipewire
    wireplumber
    llvmPackages_17.llvm
    llvmPackages_17.clang
    llvmPackages_17.libclang
    udev
    libdbusmenu
  ];
  shellHook = ''
    export LIBCLANG_PATH="${pkgs.llvmPackages_17.libclang.lib}/lib"
    export PKG_CONFIG_PATH="$PKG_CONFIG_PATH:${pkgs.wireplumber.dev}/lib/pkgconfig"
  '';
}
