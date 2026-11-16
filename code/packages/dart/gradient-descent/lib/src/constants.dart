/// Package constants for the ML02 gradient-descent optimizer.
library;

/// Semantic version of this package.
const String gradientDescentVersion = '0.1.0';

/// Maximum number of elements accepted in `weights` / `gradients`.
///
/// Why: the output buffer is allocated with the same length as the inputs.
/// Checking this ceiling *before* allocation bounds memory use for hostile
/// callers (CWE-400) and keeps the limit identical across language lanes.
const int maxVectorLength = 1048576;
