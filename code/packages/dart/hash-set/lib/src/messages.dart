/// Centralized localizable diagnostic messages complying with Standing Rule #20.

/// Diagnostic message when an initial capacity parameter is invalid or non-positive.
const String messageInvalidCapacity =
    'hash-set: initial capacity must be positive and not exceed limit';

/// Diagnostic message when a batch size exceeds the safety limit.
const String messageInvalidBatchElements =
    'hash-set: batch size exceeds maximum allowed bound';

/// Diagnostic message when a disjoint comparison encounters an unexpected state.
const String messageDisjointFailed =
    'hash-set: disjoint check evaluation failure';

/// Diagnostic message when an internal set invariant is violated.
const String messageInternalInconsistency =
    'hash-set: internal set state inconsistency detected';
