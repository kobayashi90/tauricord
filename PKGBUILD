# Maintainer: Tauricord Development Team
# Contributor: Your Name <your.email@example.com>
pkgname=tauricord
pkgver=0.5.0
pkgrel=1
pkgdesc="A lightweight desktop wrapper for the Discord Web App, built using Tauri"
arch=('x86_64')
url="https://github.com/kobayashi90/tauricord"
license=('Unlicense')
depends=(
  'webkit2gtk-4.1'
  'libappindicator-gtk3'
  'libxcb'
)
makedepends=(
  'rust'
  'cargo'
  'base-devel'
  'git'
)
provides=("$pkgname")
conflicts=("$pkgname")
source=("git+https://github.com/kobayashi90/tauricord.git#tag=v${pkgver}")
sha256sums=('SKIP')

build() {
  cd "$pkgname"
  cargo build --release --locked
}

check() {
  cd "$pkgname"
  cargo test --release
}

package() {
  cd "$pkgname"
  
  # Binary
  install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
  
  # Icon
  install -Dm644 "icons/icon.png" \
    "$pkgdir/usr/share/icons/hicolor/256x256/apps/io.tauricord.dev.png"
  
  # Desktop file
  install -Dm644 "assets/io.tauricord.dev.desktop" \
    "$pkgdir/usr/share/applications/io.tauricord.dev.desktop"
  
  # License
  install -Dm644 "LICENSE" \
    "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
