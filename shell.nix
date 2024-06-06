{ pkgs ? import <nixos> { }, unstable ? import <nixos-unstable> { } }:
# let
#   libdbusmenu-gtk4 = with pkgs; stdenv.mkDerivation (finalAttrs: {
#     pname = "libdbusmenu-gtk4";
#     version = "16.04.0";

#     src = ./libdbusmenu;

#     nativeBuildInputs = [
#       vala
#       pkg-config
#       intltool
#       gobject-introspection
#       gnome.gnome-common

#       # use for initial building
#       gtk-doc
#       gettext
#       glib
#       intltool
#       pkg-config
#       libtool
#       gtk2
#       gtk3
#       gtk4
#       atk
#       json-glib
#       gobject-introspection
#       libxslt
#     ];

#     buildInputs = [
#       glib
#       dbus-glib
#       json-glib
#       gtk4
#     ];

#     postPatch = ''
#       for f in {configure,ltmain.sh,m4/libtool.m4}; do
#         substituteInPlace $f \
#           --replace /usr/bin/file ${file}/bin/file
#       done
#     '';

#     # https://projects.archlinux.org/svntogit/community.git/tree/trunk/PKGBUILD?h=packages/libdbusmenu
#     preConfigure = ''
#       export HAVE_VALGRIND_TRUE="#"
#       export HAVE_VALGRIND_FALSE=""
#     '';

#     configureFlags = [
#       "CFLAGS=-Wno-error"
#       "--sysconfdir=/etc"
#       "--localstatedir=/var"
#       # TODO use `lib.withFeatureAs`
#       "--with-gtk=4"
#       "--disable-scrollkeeper"
#       "--disable-dumper"
#     ];

#     doCheck = false; # generates shebangs in check phase, too lazy to fix

#     installFlags = [
#       "sysconfdir=${placeholder "out"}/etc"
#       "localstatedir=\${TMPDIR}"
#       "typelibdir=${placeholder "out"}/lib/girepository-1.0"
#     ];

#     passthru.tests.pkg-config = testers.testMetaPkgConfig finalAttrs.finalPackage;

#     meta = with lib; {
#       description = "Library for passing menu structures across DBus";
#       homepage = "https://launchpad.net/dbusmenu";
#       license = with licenses; [ gpl3 lgpl21 lgpl3 ];
#       pkgConfigModules = [
#         "dbusmenu-glib-0.4"
#         "dbusmenu-jsonloader-0.4"
#         "dbusmenu-gtk4-0.4"
#       ];
#       platforms = platforms.linux;
#       maintainers = [ maintainers.msteen ];
#     };
#   });
# in
# let
#   libdbusmenu = with pkgs; stdenv.mkDerivation (finalAttrs: {
#     pname = "libdbusmenu-glib";
#     version = "16.04.0";

#     src = ./libdbusmenu;

#     nativeBuildInputs = [
#       vala
#       pkg-config
#       intltool
#       gobject-introspection
#       gnome.gnome-common
#       libxslt
#     ];

#     buildInputs = [
#       glib
#       dbus-glib
#       json-glib
#     ];

#     postPatch = ''
#       for f in {configure,ltmain.sh,m4/libtool.m4}; do
#         substituteInPlace $f \
#           --replace /usr/bin/file ${file}/bin/file
#       done
#     '';

#     # https://projects.archlinux.org/svntogit/community.git/tree/trunk/PKGBUILD?h=packages/libdbusmenu
#     preConfigure = ''
#       export HAVE_VALGRIND_TRUE="#"
#       export HAVE_VALGRIND_FALSE=""
#     '';

#     configureFlags = [
#       "CFLAGS=-Wno-error"
#       "--sysconfdir=/etc"
#       "--localstatedir=/var"
#       # TODO use `lib.withFeatureAs`
#       "--disable-gtk"
#       "--disable-scrollkeeper"
#       "--disable-dumper"
#     ];

#     doCheck = false; # generates shebangs in check phase, too lazy to fix

#     installFlags = [
#       "sysconfdir=${placeholder "out"}/etc"
#       "localstatedir=\${TMPDIR}"
#       "typelibdir=${placeholder "out"}/lib/girepository-1.0"
#     ];

#     passthru.tests.pkg-config = testers.testMetaPkgConfig finalAttrs.finalPackage;

#     meta = with lib; {
#       description = "Library for passing menu structures across DBus";
#       homepage = "https://launchpad.net/dbusmenu";
#       license = with licenses; [ gpl3 lgpl21 lgpl3 ];
#       pkgConfigModules = [
#         "dbusmenu-glib-0.4"
#         "dbusmenu-jsonloader-0.4"
#       ];
#       platforms = platforms.linux;
#       maintainers = [ maintainers.msteen ];
#     };
#   });
# in
pkgs.mkShell {
  nativeBuildInputs = with pkgs.buildPackages; [
    pkg-config
    unstable.gtk4
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

    # For building libdbusmenu

  ];
  shellHook = ''
    export LIBCLANG_PATH="${pkgs.llvmPackages_17.libclang.lib}/lib"
    export PKG_CONFIG_PATH="$PKG_CONFIG_PATH:${pkgs.wireplumber.dev}/lib/pkgconfig"
  '';
}
