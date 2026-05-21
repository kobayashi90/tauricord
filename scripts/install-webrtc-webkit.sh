#!/usr/bin/env bash
set -euo pipefail

# Downloads and installs the prebuilt WebKitGTK + WebRTC .deb from
# the latest tauricord GitHub release, so you don't have to compile it.
#
# Usage:
#   bash scripts/install-webrtc-webkit.sh
#
# This replaces the system's stock libwebkit2gtk-4.1 with a build that
# has -DENABLE_WEB_RTC=ON -DENABLE_MEDIA_STREAM=ON.  The stock packages
# are saved as .dpkg-old so you can revert with:
#   sudo apt-get install --reinstall libwebkit2gtk-4.1-0 libwebkit2gtk-4.1-dev

REPO="kobayashi90/tauricord"
DEB_NAME="libwebkit2gtk-4.1-webrtc_2.46.6-1_amd64.deb"

echo "==> Downloading prebuilt WebKitGTK (WebRTC) from GitHub..."
wget -q "https://github.com/$REPO/releases/latest/download/$DEB_NAME" -O "/tmp/$DEB_NAME"

echo "==> Installing (replaces system webkit2gtk)..."
sudo dpkg -i "/tmp/$DEB_NAME"
sudo ldconfig

echo ""
echo "==> Verifying WebRTC support..."
pkg-config --exists gstreamer-webrtc-1.0 && echo "    gstreamer-webrtc-1.0: OK"
pkg-config --exists nice && echo "    nice: OK"
echo ""

echo "==> Done! Now launch tauricord with:"
echo "    export GDK_BACKEND=x11"
echo "    export WEBKIT_DISABLE_DMABUF_RENDERER=1"
echo "    export GST_PLUGIN_PATH=/usr/lib/x86_64-linux-gnu/gstreamer-1.0"
echo "    tauricord"
