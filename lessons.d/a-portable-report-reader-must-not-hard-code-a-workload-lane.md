---
category: Testing & coverage
---

# A portable report reader must not hard-code a workload lane

The C raw-report reader required all command metadata and rejected every
`workload` except `indexed-array-sum-4096`. Cross-replaying a valid guest core
report therefore failed even though the raw pairs and analysis were portable.
The portable core is `config`, `batch_size`, `pairs`, and `analysis`; metadata
is optional as a complete group, and a producer chooses its own workload ID.

When a lane-specific producer schema is reused as a cross-language reader,
check retained reports from other producers before treating local CLI output as
the universal shape. Test both the minimal core and complete generic metadata;
keep the producer's own workload validation at its CLI boundary.

A generic workload ID is nonblank when it contains any scalar outside the six
ASCII whitespace characters. Other metadata fields may have their own blank
policy; do not reuse Unicode `isWhitespace` for workload identifiers.
