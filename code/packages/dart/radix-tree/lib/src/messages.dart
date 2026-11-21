/// Diagnostic and error messages for the radix tree package, strictly payload-blind.
///
/// Standing Rule #20: Every error id, message, placeholder, and structural string
/// template is maintained in this file so operational code carries no string
/// literals and no diagnostic can ever echo caller keys or values.
library;

/// Error identifier for malformed or out-of-bounds keys.
const String errorIdInvalidKey = 'invalid-key';

/// Error identifier for absent keys requested via index operator.
const String errorIdKeyNotFound = 'key-not-found';

/// Message thrown when a key contains lone surrogates or out-of-range scalars.
const String messageInvalidScalar =
    'Radix tree key contains an invalid Unicode scalar.';

/// Message thrown when a key exceeds the maximum allowed scalar length.
const String messageKeyTooLong =
    'Radix tree key exceeds maximum allowed length.';

/// Message thrown when an exact key lookup via index operator fails.
const String messageKeyNotFound = 'Radix tree key not found.';

/// Redacted placeholder for payload confidentiality (CWE-209).
const String redactedPlaceholder = '<redacted>';

/// Placeholder for invalid or unparseable elements.
const String invalidPlaceholder = '<invalid>';

/// Prefix used in the payload-blind [RadixTree.toString] representation.
const String radixTreeStringPrefix = 'RadixTree(count: ';

/// Middle separator for node count in [RadixTree.toString].
const String radixTreeNodeCountPrefix = ', nodeCount: ';

/// Middle separator for edge count in [RadixTree.toString].
const String radixTreeEdgeCountPrefix = ', edgeCount: ';

/// Suffix used in the payload-blind [RadixTree.toString] representation.
const String radixTreeStringSuffix = ')';

/// Prefix used in the payload-blind [RadixEntry.toString] representation.
const String radixEntryStringPrefix = 'RadixEntry(';

/// Key-value separator in the payload-blind [RadixEntry.toString] representation.
const String radixEntryStringSeparator = ', ';

/// Suffix used in the payload-blind [RadixEntry.toString] representation.
const String radixEntryStringSuffix = ')';
