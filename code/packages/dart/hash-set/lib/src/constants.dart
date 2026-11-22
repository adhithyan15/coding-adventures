/// Centralized constants for the persistent hash set.

/// Default initial capacity of the hash set's underlying hash table.
const int defaultCapacity = 16;

/// Sentinel value stored in the underlying map (membership decided via key presence).
const bool sentinelValue = true;

/// Maximum number of batch elements in high-volume set construction.
const int maxBatchElements = 65536;

/// Redacted placeholder for payload-blind error diagnostics (Standing Rule #20).
const String redactedPlaceholder = '<redacted>';

/// Invalid placeholder for payload-blind error diagnostics (Standing Rule #20).
const String invalidPlaceholder = '<invalid>';
