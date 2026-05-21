#!/usr/bin/env bash
set -euo pipefail

# Builds WebKitGTK from source with WebRTC + MediaStream enabled and
# packages it as a .deb for distribution.
#
# Why: Debian/Ubuntu's stock libwebkit2gtk disables WebRTC (patent fears),
# which breaks Discord voice. Everyone in the Tauri/Discord ecosystem has
# to compile their own -- now you can share the result.
#
# Requirements: ~8 GB disk, ~45 min build time.
#
# Runtime notes for WebRTC video:
#   - Set GDK_BACKEND=x11  (Wayland has GBM buffer issues with webrtcbin)
#   - Set WEBKIT_DISABLE_DMABUF_RENDERER=1
#   - Set GST_PLUGIN_PATH to include gst-plugins-good and gst-plugins-bad
#   - tauricord's inject.js already hijacks permissions.query to grant
#     mic/camera, so no wry patches needed.

WEBKIT_VERSION="2.46.6"
NUM_JOBS="${NUM_JOBS:-$(nproc)}"
PREFIX="${PREFIX:-/usr}"
PKG_NAME="libwebkit2gtk-4.1-webrtc"
PKG_VERSION="${WEBKIT_VERSION}-1"

echo "==> Installing build & runtime dependencies..."
sudo apt-get install -y \
    cmake \
    ninja-build \
    gperf \
    bison \
    flex \
    ruby-dev \
    unifdef \
    checkinstall \
    libgcrypt20-dev \
    libsecret-1-dev \
    libssl-dev \
    libglib2.0-dev \
    libicu-dev \
    libfontconfig-dev \
    libfreetype-dev \
    libcairo2-dev \
    libpango1.0-dev \
    libharfbuzz-dev \
    libepoxy-dev \
    libtasn1-6-dev \
    libxml2-dev \
    libxslt1-dev \
    libsqlite3-dev \
    libjpeg-dev \
    libpng-dev \
    libwebp-dev \
    libopenjp2-7-dev \
    libtiff-dev \
    libgdk-pixbuf-2.0-dev \
    liblcms2-dev \
    libseccomp-dev \
    libsystemd-dev \
    libatk-bridge2.0-dev \
    libx11-dev \
    libxft-dev \
    libxt-dev \
    libxcomposite-dev \
    libxdamage-dev \
    libxrandr-dev \
    libxrender-dev \
    libxi-dev \
    libxkbcommon-dev \
    libwayland-dev \
    wayland-protocols \
    libegl-dev \
    libgl-dev \
    libgles-dev \
    libegl1-mesa-dev \
    libgles2-mesa-dev \
    libgtk-3-dev \
    libgtk-4-dev \
    libwpebackend-fdo-1.0-dev \
    libgudev-1.0-dev \
    libsoup-3.0-dev \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    libgstreamer-plugins-bad1.0-dev \
    libnice-dev \
    libwebrtc-audio-processing-dev \
    gobject-introspection \
    libgirepository1.0-dev \
    gi-docgen \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    libunwind-dev \
    gettext

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
    -DENABLE_DOCUMENTATION=OFF \
    -DUSE_LIBBACKTRACE=OFF \
    -DENABLE_BUBBLEWRAP_SANDBOX=OFF

echo "==> Building WebKitGTK (this will take a while)..."
ninja -j"$NUM_JOBS"

echo "==> Packaging as .deb..."
sudo checkinstall -D --install=no --fstrans=no \
    --pkgname="$PKG_NAME" \
    --pkgversion="$PKG_VERSION" \
    --pkgrelease="1" \
    --pkgarch="amd64" \
    --pkgsource="https://webkitgtk.org/releases/webkitgtk-$WEBKIT_VERSION.tar.xz" \
    --pkglicense="LGPL-2.1+" \
    --maintainer="tauricord" \
    --provides="libwebkit2gtk-4.1-0,libwebkit2gtk-4.1-dev,libjavascriptcoregtk-4.1-0,libjavascriptcoregtk-4.1-dev,gir1.2-webkit2-4.1,gir1.2-javascriptcoregtk-4.1" \
    --conflicts="libwebkit2gtk-4.1-0,libwebkit2gtk-4.1-dev,libjavascriptcoregtk-4.1-0,libjavascriptcoregtk-4.1-dev" \
    --replaces="libwebkit2gtk-4.1-0,libwebkit2gtk-4.1-dev,libjavascriptcoregtk-4.1-0,libjavascriptcoregtk-4.1-dev" \
    --requires="libc6,libglib2.0-0,libgtk-4-1,libsoup-3.0-0,libgstreamer1.0-0,libnice10,libwebrtc-audio-processing1,libepoxy0,libharfbuzz0b,libcairo2,libpng16-16,libjpeg62-turbo,libwebp7,libxml2,libxslt1.1,libsqlite3-0,libpango-1.0-0,libgdk-pixbuf-2.0-0,liblcms2-2,libseccomp2,libsystemd0,libtasn1-6,libgcrypt20,libsecret-1-0,libssl3,libfontconfig1,libfreetype6,libatk-bridge2.0-0,libepoxy0,libwpebackend-fdo-1.0-0,libgudev-1.0-0,gstreamer1.0-plugins-good,gstreamer1.0-plugins-bad" \
    --pakdir=/tmp \
    ninja install

sudo ldconfig

echo "==> .deb created: $(ls -1h /tmp/*.deb /root/*.deb 2>/dev/null | head -1)"
echo ""
echo "==> Verifying WebRTC support..."
pkg-config --cflags --libs webkit2gtk-4.1

echo ""
echo "==> Done! WebKitGTK $WEBKIT_VERSION with WebRTC packaged."
echo "    Install with: sudo dpkg -i ${PKG_NAME}_${PKG_VERSION}_amd64.deb"
echo ""
echo "    Runtime env for Discord voice:"
echo "      export GDK_BACKEND=x11"
echo "      export WEBKIT_DISABLE_DMABUF_RENDERER=1"
echo "      export GST_PLUGIN_PATH=/usr/lib/x86_64-linux-gnu/gstreamer-1.0"
