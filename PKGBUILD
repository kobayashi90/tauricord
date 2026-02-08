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
)
provides=("$pkgname")
conflicts=("$pkgname")
source=("$pkgname-$pkgver.tar.gz::https://github.com/kobayashi90/tauricord/archive/refs/tags/v${pkgver}.tar.gz")
sha256sums=('SKIP')

build() {
  cd "$pkgname-$pkgver"
  cargo build --release --locked
}

check() {
  cd "$pkgname-$pkgver"
  cargo test --release
}

package() {
  cd "$pkgname-$pkgver"
  
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
