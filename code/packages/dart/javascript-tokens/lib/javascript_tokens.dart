/// Backend-agnostic JavaScript pipeline types: every ECMAScript
/// edition with a grammar ([EsVersion]), byte-offset spans ([Span]),
/// and the broad cross-version token classification ([TokenKind]).
library;

export 'src/es_version.dart';
export 'src/span.dart';
export 'src/token_kind.dart';
