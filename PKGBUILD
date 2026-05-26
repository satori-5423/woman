# Maintainer: satori-5423 <suxue2746841150@gmail.com>
# Contributor: satori-5423 <suxue2746841150@gmail.com>

pkgname=woman
pkgver=0.1.0
pkgrel=1
pkgdesc='AI-Powered Man Page Translation Tool'
arch=('x86_64')
url='https://github.com/satori-5423/woman'
license=('MIT')
depends=(
    'man-db'
    'glibc'
    'gcc-libs'
)
makedepends=('cargo')
source=("$pkgname-$pkgver.tar.gz::https://github.com/satori-5423/woman/archive/v$pkgver.tar.gz")
b2sums=('SKIP')

build() {
    cd "$srcdir/$pkgname-$pkgver"
    # Strip -flto=auto from CFLAGS to avoid GCC LTO ↔ LLD linker incompatibility.
    # ring's C/assembly code compiled with GCC LTO produces objects that lld
    # (LLVM linker) cannot read, causing undefined reference errors.
    export CFLAGS="${CFLAGS/-flto=auto/}"
    cargo build --release --locked
}

check() {
    cd "$srcdir/$pkgname-$pkgver"
    cargo test --release --locked
}

package() {
    cd "$srcdir/$pkgname-$pkgver"

    # Binary
    install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"

    # Shell completions
    install -Dm644 completions/woman.fish "$pkgdir/usr/share/fish/vendor_completions.d/woman.fish"

    # License
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
