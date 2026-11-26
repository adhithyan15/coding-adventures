# Changelog

## 0.1.0

- Add predeclared traversal and buffer-growth experiment plans, validated before
  allocating data or starting collection. Retain separate portable reports,
  source/plan/executable provenance, software work counts and a completion manifest.
- Compare indexed/range summation, identical dependent-link code with ordered
  versus permuted successors, and fixed-increment versus doubling capacity.
- Bound workloads and batches to keep checksum arithmetic safe. Preserve raw
  timings without clamping or timing-based correctness thresholds.
- Stage and close the completion manifest before atomic no-clobber publication;
  refuse unsupported filesystems and retain no visible incomplete manifest.
