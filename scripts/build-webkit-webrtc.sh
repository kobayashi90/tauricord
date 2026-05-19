#!/usr/bin/env bash
set -euo pipefail

# Builds WebKitGTK from source with WebRTC + MediaStream enabled.
# This is needed because Ubuntu's stock libwebkit2gtk-4.1-dev does not
# enable WebRTC (patent/legal concerns), which breaks Discord voice.
#
# Requirements: ~8 GB disk, ~45 min build time.
# Only run this if the WebKit PPA is unavailable.

WEBKIT_VERSION="2.46.6"
NUM_JOBS="${NUM_JOBS:-$(nproc)}"
PREFIX="${PREFIX:-/usr}"

echo "==> Installing build dependencies..."
sudo apt-get install -y \
    cmake \
    ninja-build \
    libicu-dev \
    libtasn1-6-dev \
    libx11-dev \
    libxft-dev \
    libxt-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libgst-dev \
    libnice-dev \
    libwebrtc-audio-processing-dev \
    libsoup-3.0-dev \
    libsqlite3-dev \
    libxml2-dev \
    libxslt1-dev \
    libjpeg-dev \
    libpng-dev \
    libwebp-dev \
    libopenjp2-7-dev \
    libtiff-dev \
    libgdk-pixbuf-2.0-dev \
    libcairo2-dev \
    libpango1.0-dev \
    libharfbuzz-dev \
    libhyphen-dev \
    libwoff-dev \
    libmanette-0.2-dev \
    libseccomp-dev \
    libsystemd-dev \
    liblcms2-dev \
    libatk-bridge2.0-dev \
    libepoxy-dev \
    libegl1-mesa-dev \
    libgles2-mesa-dev

echo "==> Downloading WebKitGTK $WEBKIT_VERSION..."
cd /tmp
wget -q "https://webkitgtk.org/releases/webkitgtk-$WEBKIT_VERSION.tar.xz"
tar xf "webkitgtk-$WEBKIT_VERSION.tar.xz"
cd "webkitgtk-$WEBKIT_VERSION"

echo "==> Configuring with WebRTC + MediaStream enabled..."
mkdir -p build && cd build
cmake .. \
    -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="$PREFIX" \
    -DPORT=GTK \
    -DENABLE_WEB_RTC=ON \
    -DENABLE_MEDIA_STREAM=ON \
    -DENABLE_GAMEPAD=OFF \
    -DENABLE_SPELLCHECK=OFF \
    -DENABLE_JOURNALD_LOG=OFF \
    -DUSE_AVIF=OFF \
    -DUSE_JPEGXL=OFF \
    -DUSE_WOFF2=OFF \
    -DUSE_SYSTEM_MALLOC=ON \
    -DUSE_LIBHYPHEN=OFF \
    -DENABLE_BUBBLEWRAP_SANDBOX=OFF

echo "==> Building WebKitGTK (this will take a while)..."
ninja -j"$NUM_JOBS"

echo "==> Installing..."
sudo ninja install
sudo ldconfig

echo "==> Verifying WebRTC support..."
pkg-config --cflags --libs webkit2gtk-4.1

echo "==> Done! WebKitGTK $WEBKIT_VERSION with WebRTC installed."
