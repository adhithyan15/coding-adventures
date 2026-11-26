/// HTTP/1 request and response head parsing.
///
/// HTTP/1 is text-framed. A caller first needs the start line and headers,
/// then clear instructions for how to consume the body bytes that follow.
/// [parseRequestHead] and [parseResponseHead] draw exactly that boundary:
/// they return the parsed head, the [ParsedRequestHead.bodyOffset] where
/// the body starts, and the [BodyKind] saying how to consume it. All
/// failures throw [Http1ParseError].
library;

export 'src/http1.dart';
