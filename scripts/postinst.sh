#!/bin/sh
set -e

# Tauricord post-install script
# Prints WebRTC setup guidance based on distro.

if [ -r /etc/os-release ]; then
  . /etc/os-release
fi

# Check if runtime packages are already installed
HAS_GST=0
HAS_NICE=0
if command -v dpkg >/dev/null 2>&1; then
  dpkg -s gstreamer1.0-plugins-bad >/dev/null 2>&1 && HAS_GST=1
  dpkg -s libnice10 >/dev/null 2>&1 && HAS_NICE=1
fi

if [ "$HAS_GST" = 1 ] && [ "$HAS_NICE" = 1 ]; then
  echo "  [✓] WebRTC runtime packages already installed."
  exit 0
fi

echo ""
echo "  ┌──────────────────────────────────────────────────────────┐"
echo "  │ Discord voice needs WebRTC-enabled WebKitGTK             │"
echo "  │                                                          │"
case "$ID" in
  ubuntu)
    case "$VERSION_ID" in
      22.04|22.10)
        echo "  │ Ubuntu $VERSION_ID needs the WebRTC PPA:                    │"
        echo "  │ (Ubuntu PPA — does NOT work on Debian)                      │"
        echo "  │                                                          │"
        echo "  │   sudo add-apt-repository ppa:escalion/ppa-webkit2gtk-    │"
        echo "  │   experimental                                            │"
        echo "  │   sudo apt update && sudo apt upgrade                    │"
        echo "  │                                                          │"
        echo "  │ Then install runtime deps:                                │"
        echo "  │   sudo apt install gstreamer1.0-plugins-bad libnice10     │"
        echo "  │                libwebrtc-audio-processing1                │"
        ;;
      *)
        echo "  │ Ubuntu $VERSION_ID may need the WebRTC PPA:                  │"
        echo "  │ (Ubuntu PPA — does NOT work on Debian)                      │"
        echo "  │                                                          │"
        echo "  │   sudo add-apt-repository ppa:escalion/ppa-webkit2gtk-    │"
        echo "  │   experimental                                            │"
        echo "  │   sudo apt update && sudo apt upgrade                    │"
        echo "  │                                                          │"
        echo "  │ Or install runtime deps directly:                         │"
        echo "  │   sudo apt install gstreamer1.0-plugins-bad libnice10     │"
        echo "  │                libwebrtc-audio-processing1                │"
        ;;
    esac
    ;;
  debian)
    echo "  │ Debian $VERSION_ID stock WebKitGTK (2.50.x) lacks WebRTC.   │"
    echo "  │ You need a newer WebKitGTK. Options:                         │"
    echo "  │                                                          │"
    echo "  │ 1) Debian experimental (version 2.53+ has WebRTC):           │"
    echo "  │    echo 'deb http://deb.debian.org/debian experimental main' │"
    echo "  │    >> /etc/apt/sources.list                                  │"
    echo "  │    apt update && apt install -t experimental \\             │"
    echo "  │      libwebkit2gtk-4.1-0                                     │"
    echo "  │                                                          │"
    echo "  │ 2) Flatpak build (bundles its own WebRTC-enabled WebKit):    │"
    echo "  │    cargo tauri build --bundles flatpak                       │"
    echo "  │                                                          │"
    echo "  │ Then install runtime deps:                                    │"
    echo "  │   sudo apt install gstreamer1.0-plugins-bad libnice10        │"
    echo "  │                libwebrtc-audio-processing1                   │"
    ;;
  *)
    echo "  │ See https://github.com/kobayashi90/tauricord#voice          │"
    echo "  │ for distro-specific WebRTC setup instructions.              │"
    ;;
esac
echo "  └──────────────────────────────────────────────────────────┘"
echo ""
