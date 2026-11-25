# Metadata projection must not skip the declared workload validation

A report API can intentionally return only its numerical core while still
owning validation of optional metadata supplied at its input. Dropping a field
from the result must not silently bypass a published validation contract.

Validate workload labels before projection. Test accepted Unicode identifiers,
ASCII whitespace and NUL separately, and verify the input report stays intact.
A value-mapping API cannot detect duplicate keys already collapsed by a JSON
parser; document that separate boundary instead of implying raw-ingress checks.
