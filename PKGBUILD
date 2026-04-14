# Maintainer: Your Name <your.email@example.com>
pkgname=vellum-git
pkgver=0.1.0
pkgrel=1
pkgdesc="A dynamic web-based wallpaper runner for Linux"
arch=('x86_64')
url="https://github.com/usoy/vellum"
license=('MIT')
depends=('gtk3' 'webkit2gtk' 'gtk-layer-shell')
makedepends=('rust' 'cargo')
provides=('vellum')
conflicts=('vellum')
source=('git+https://github.com/usoy/vellum.git')
sha256sums=('SKIP')

build() {
  cd vellum
  make build
}

package() {
  cd vellum
  make install DESTDIR="$pkgdir"
}
