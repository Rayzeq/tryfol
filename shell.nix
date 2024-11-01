{ pkgs ? import <nixos> { } }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs.buildPackages; [
    pkg-config
    gtk4
    gtk4-layer-shell
    pipewire
    wireplumber
    llvmPackages_17.llvm
    llvmPackages_17.clang
    llvmPackages_17.libclang
    udev
    libdbusmenu
    openssl
  ];
  shellHook = ''
    export LIBCLANG_PATH="${pkgs.llvmPackages_17.libclang.lib}/lib"
    export PKG_CONFIG_PATH="$PKG_CONFIG_PATH:${pkgs.wireplumber.dev}/lib/pkgconfig"
    echo "
      set auto-load safe-path /
      set debug-file-directory ${pkgs.gtk4.debug}/lib/debug/
    " > .gdbinit
  '';
}
