# Maintainer: Andreas Bjoru <andreas.bjoru@gmail.com>
pkgname=scribe
pkgver=0.1.0
pkgrel=1
pkgdesc="Fast, lean voice dictation using Whisper"
arch=('x86_64')
url="https://github.com/abjoru/scribe"
license=('MIT')
depends=('dotool' 'alsa-lib')
makedepends=('rust' 'cargo')
optdepends=(
    'cuda: GPU acceleration'
    'polybar: system tray support'
)
source=("${pkgname}-${pkgver}.tar.gz::${url}/archive/v${pkgver}.tar.gz")
sha256sums=('SKIP')
install=scribe.install

build() {
    cd "${srcdir}/${pkgname}-${pkgver}"
    cargo build --release --locked
}

package() {
    cd "${srcdir}/${pkgname}-${pkgver}"

    # Binary
    install -Dm755 "target/release/scribe" \
        "${pkgdir}/usr/bin/scribe"

    # Default config
    install -Dm644 "config/default.toml" \
        "${pkgdir}/usr/share/scribe/default.toml"

    # Systemd service
    install -Dm644 "scribe.service" \
        "${pkgdir}/usr/lib/systemd/user/scribe.service"

    # udev rules
    install -Dm644 "99-uinput.rules" \
        "${pkgdir}/usr/lib/udev/rules.d/99-scribe.rules"

    # Documentation
    install -Dm644 README.md "${pkgdir}/usr/share/doc/${pkgname}/README.md"

    # License
    install -Dm644 LICENSE "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}
