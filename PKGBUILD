# Maintainer: Andreas Bjoru <andreas.bjoru@gmail.com>
pkgname=scribe
pkgver=0.1.4
pkgrel=1
pkgdesc="Fast, lean voice dictation using Whisper"
arch=('x86_64')
url="https://github.com/abjoru/scribe"
license=('MIT')
depends=('dotool' 'alsa-lib' 'oniguruma')
makedepends=('rust' 'cargo')
optdepends=(
    'cuda: GPU acceleration'
    'polybar: system tray support'
)
install=scribe.install

build() {
    # Force gcc linker instead of rust-lld for proper system library linking
    export RUSTFLAGS="-C linker=gcc"

    # Use system oniguruma library
    export RUSTONIG_SYSTEM_LIBONIG=1

    cargo build --release --locked
}

package() {
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
