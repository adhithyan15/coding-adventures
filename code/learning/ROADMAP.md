# Learning Coverage Roadmap

This document turns the generated package coverage inventory into an editorial
plan. The inventory answers "is there evidence of learning material for this
package concept?" This roadmap answers "what should we teach next, and why?"

## Baseline Survey

The survey was refreshed from the live package tree on 2026-07-30, before the
first content batch in this roadmap:

| Measure | Baseline |
| --- | ---: |
| Distinct package concepts | 1,227 |
| Dedicated learning coverage | 57 |
| Related coverage | 52 |
| Index-only coverage | 6 |
| Missing coverage | 1,112 |
| P0 concepts with missing coverage | 99 |
| P1 concepts with missing coverage | 94 |

P0 means a concept has implementations in at least 10 established language
ecosystems. P1 means it appears in 5 to 9. Breadth is a useful signal because a
single conceptual lesson can support many package implementations.

The raw document count is not a useful measure of package-companion depth by
itself. The independent human-language curriculum contains many lesson files
and follows its own coverage model. For package learning, the meaningful
measures are concept status and the quality of the linked explanation.

## What The Inventory Can And Cannot Tell Us

The generated report recognizes four structural states:

- **Dedicated:** the filename or a `learning-concepts` annotation claims the
  concept.
- **Related:** a non-index lesson mentions the concept.
- **Index only:** the concept appears only in a learning index.
- **Missing:** no structural evidence was found.

This is intentionally mechanical. An annotation makes work discoverable, but it
does not prove that the prose is correct, complete, or approachable. Editorial
review still needs to ask:

1. Does the lesson explain the problem before presenting the mechanism?
2. Is there a worked example a reader can calculate or trace?
3. Are invariants and failure modes explicit?
4. Does the lesson separate durable ideas from this repository's API choices?
5. Does it point readers toward the packages and specs that make the idea real?

## Priority Model

The learning backlog should be worked in four lanes.

### Lane 1: Broad shared foundations

Start with P0 families that have several related packages. One lesson can give
readers a map across all language mirrors without duplicating package READMEs.

Initial batch:

- WebAssembly binary format, validation, instantiation, and execution
- structured text parsing across JSON, CSV, TOML, and XML
- SQL parsing, logical evaluation, planning, execution, and storage
- one-dimensional and two-dimensional barcodes plus error correction

That batch raised dedicated coverage from 57 to 98 concepts and reduced missing
coverage from 1,112 to 1,065. More importantly, it reduced the P0 missing
backlog from 99 to 56 concepts. These numbers exclude local untracked package
work and are recorded in the generated `COVERAGE.md`.

### Lane 2: Existing notes that are too shallow

Convert `related` and `index-only` concepts into dedicated lessons when the
current material only names them. This is often higher leverage than starting a
new subject because the surrounding curriculum and examples already exist.

### Lane 3: Deep single-language systems

Breadth alone underrates major Rust-first systems such as ADJ, native AOT and
GC, Venture, smart-home, SPICE, and the Semantic IR backends. These may appear
as P3 even when they are among the repository's most important integration
tracks. Each quarterly pass should reserve capacity for these systems.

### Lane 4: Learning maintenance

Every new package family should either link to an existing lesson or add a
backlog entry. Regenerate `COVERAGE.md` in the same change as new learning
material so drift remains visible.

## Next Recommended Batches

After the initial batch, the strongest breadth-weighted candidates are:

1. paint instructions, affine geometry, paths, and rendering VMs
2. HTTP, URLs, RESP, and protocol framing
3. state machines, event loops, and supervised reactors
4. image point operations and geometric transforms
5. document ASTs, CommonMark/GFM, AsciiDoc, and sanitization
6. mini-SQLite storage pages, B-trees, WAL, and transaction semantics

The first depth-weighted candidates are:

1. tagged dynamic values and native garbage collection
2. ADJ rules, formulas, provenance, and explanations
3. the Venture URL-to-pixels browser pipeline
4. local-first smart-home events, commands, policy, and supervision

## Regenerating The Evidence

```bash
python3 code/scripts/learning_coverage_report.py \
  --output code/learning/COVERAGE.md
```

The report is a backlog generator, not a score to optimize blindly. A smaller
number of clear, connected lessons is preferable to hundreds of files that only
repeat package summaries.
