/// Package-level numeric bounds, versioning, and structural limits for the
/// radix tree implementation.
///
/// Standing Rule #20: All magic numbers, structural bounds, and limits are
/// defined as named constants here rather than scattered inline.
library;

/// The semantic version string of the radix tree package.
const String radixTreePackageVersion = '0.1.0';

/// Maximum allowed Unicode scalar count for a radix tree key or prefix.
///
/// Prevents CWE-400 resource exhaustion via arbitrarily long keys.
const int maxKeyScalars = 65536;

/// Minimum valid Unicode scalar code point.
const int minUnicodeScalar = 0x0000;

/// Maximum valid Unicode scalar code point (U+10FFFF).
const int maxUnicodeScalar = 0x10ffff;

/// Inclusive lower bound of the UTF-16 surrogate range (U+D800).
const int highSurrogateMin = 0xd800;

/// Inclusive upper bound of the UTF-16 surrogate range (U+DFFF).
const int lowSurrogateMax = 0xdfff;

/// Minimum child count for a non-root, non-endpoint inner node.
///
/// In a compressed trie, any non-root node that is not an endpoint must have at
/// least 2 children; otherwise it should have been compressed into a single edge.
const int minNonRootInnerChildren = 2;

/// Threshold child count for merge-back re-compression after key deletion.
const int singleChildThreshold = 1;
