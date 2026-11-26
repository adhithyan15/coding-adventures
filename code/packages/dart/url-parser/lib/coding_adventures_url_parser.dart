/// URL parsing, resolution, and canonicalization.
///
/// The Dart lane of `url-parser` (spec NET00): [Url.parse] splits absolute
/// URLs into components, [Url.resolve] implements RFC 1808 relative
/// resolution, [Url.canonicalize] normalizes identity, and
/// [percentEncode]/[percentDecode] convert percent-coding. Failures throw
/// [UrlError].
library coding_adventures_url_parser;

export 'src/url_parser.dart';
