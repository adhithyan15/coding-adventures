/// HMAC message authentication (RFC 2104 / FIPS 198-1).
///
/// The Dart lane of the Rust `hmac` crate: a generic higher-order
/// [hmac] over any in-lane hash, fixed-memory SHA-256/SHA-384 paths
/// with segmented-message variants, MD5/SHA-1/SHA-512 named variants,
/// lowercase hex conveniences, and constant-time [verifyHmac].
library;

export 'src/errors.dart';
export 'src/hmac.dart';
