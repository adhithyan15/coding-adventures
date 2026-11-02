### Added Kotlin micro-qr Gradle and build metadata

The existing Kotlin Micro QR encoder is now discoverable and executable through
repository-standard Gradle metadata and cross-platform BUILD fronts. Its native
suite runs against repository-local GF(256), barcode, and paint dependencies,
and an explicit empty capability profile records the encoder's pure in-memory
authority boundary.
