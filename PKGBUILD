# Maintainer: Tauricord <tauricord@example.com>
pkgname=tauricord
pkgver=0.5.0
pkgrel=1
pkgdesc="A lightweight desktop wrapper for the Discord Web App, built using Tauri"
arch=('x86_64')
url="https://github.com/kobayashi90/tauricord"
license=('Unlicense')
depends=('webkit2gtk-4.1' 'libappindicator-gtk3')
makedepends=('rust' 'cargo' 'npm')
source=("https://github.com/kobayashi90/tauricord/archive/refs/tags/v${pkgver}.tar.gz")
sha256sums=('SKIP')

build() {
  cd "$pkgname-$pkgver"
  cargo build --release
}

package() {
  cd "$pkgname-$pkgver"
  install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
  install -Dm644 "icons/icon.png" "$pkgdir/usr/share/icons/hicolor/256x256/apps/io.tauricord.dev.png"
  install -Dm644 "assets/io.tauricord.dev.desktop" "$pkgdir/usr/share/applications/io.tauricord.dev.desktop"
}
