# Unreleased — Paired benchmark runner fixture consumer

Register the new paired replay reader as `go/programs/benchmark-tool`, using its
actual program identity and path. A canonical fixture edit selects twenty-four
package readers and this program, exports the exact repository input to Kiln,
and invalidates the legacy fallback hash. Extend all-consumer, language-filter,
missing-consumer, export and byte-invalidation checks.
