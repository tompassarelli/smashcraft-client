# The libraries the Linux client links and loads (the tray library is loaded at
# run time). scripts/install.ts builds this with an out-link so they stay alive.
{ pkgs ? import <nixpkgs> { } }:
pkgs.buildEnv {
  name = "smashcraft-client-runtime";
  paths = with pkgs; [
    webkitgtk_4_1
    gtk3
    libsoup_3
    glib
    glib-networking
    cairo
    pango
    gdk-pixbuf
    atk
    librsvg
    openssl
    dbus
    libayatana-appindicator
  ];
  extraOutputsToInstall = [ "lib" ];
  ignoreCollisions = true;
}
