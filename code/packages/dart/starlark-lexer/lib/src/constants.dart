/// Constants and configuration boundaries for the Starlark lexical analyzer.
///
/// Complies with Standing Rule #20: magic numbers and defaults are centralized here.
library;

/// Default version identifier for Starlark grammar resolution.
const String defaultStarlarkVersion = 'starlark';

/// Supported version identifiers accepted by [resolveStarlarkVersion].
const List<String> supportedStarlarkVersions = ['starlark', '1'];

/// Maximum source length permitted in a single tokenization request (1 MB).
///
/// CWE-400 mitigation against unbounded memory consumption and regex bombs.
const int maxSourceLength = 1048576;

/// Minimum UTF-16 code unit value for high/low surrogates (0xD800).
const int minSurrogateCodeUnit = 0xD800;

/// Maximum UTF-16 code unit value for high/low surrogates (0xDFFF).
const int maxSurrogateCodeUnit = 0xDFFF;

/// Minimum UTF-16 code unit value for high surrogates (0xD800).
const int minHighSurrogate = 0xD800;

/// Maximum UTF-16 code unit value for high surrogates (0xDBFF).
const int maxHighSurrogate = 0xDBFF;

/// Minimum UTF-16 code unit value for low surrogates (0xDC00).
const int minLowSurrogate = 0xDC00;

/// Maximum UTF-16 code unit value for low surrogates (0xDFFF).
const int maxLowSurrogate = 0xDFFF;
