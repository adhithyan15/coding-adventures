# Write once, use many

The point of MYCIN-2026's knowledge layer is not a database of medical facts. It is a
demonstration of a **discipline**: a fact is **written once** — grounded to a byte-stable
primary source, with its provenance carried *inline* — and is thereafter **used many
times**, by independent consumers, with that same provenance flowing through unchanged and
**zero model calls at answer time**.

This document is backed by a machine-checked test
([`recall/test_write_once_use_many.py`](recall/test_write_once_use_many.py), 7/7) so the
claims below are verified, not asserted.

## The one write

Every domain ships a single artifact — `recall/<domain>-edges.adj` — and that file is the
*sole* source of truth. There is no generator, no JSON sidecar, no manifest. One clause is
one grounded fact, and its byte-provenance lives on the clause:

```
relate gene_defect(huntington_disease, htt)
    source "Huntington disease is an autosomal dominant inherited neurodegenerative
            disorder caused by the elongation of CAG repeats ... in the HTT gene."
    locator "https://www.ncbi.nlm.nih.gov/books/NBK559166/"
    trust authoritative
```

Writing it is the expensive part — spider to a primary authority, extract the verbatim
span that names both endpoints, confirm it is byte-stable, decline anything a clean span
cannot defend. (Where one authority phrases the association awkwardly, we ground from
another primary authority rather than omit it — e.g. coal→CWP from CDC/NIOSH. See
[`USMLE-DOMAIN-MAP.md`](USMLE-DOMAIN-MAP.md).) That cost is paid **once**.

## The many uses

After that, the same unchanged library answers in as many shapes as a consumer needs. Each
demo query below **imports the library and writes no fact of its own** — it only asks. All
run on the native CPU engine (`adj-lang-cli`); none invoke a model.

| # | Consumer | Query file | Shape | Example |
|---|----------|-----------|-------|---------|
| 1 | **Forward recall** | [`wom_forward`](recall/wom_forward.query.adj) | subject → object | `gene_defect(huntington_disease, $Gene)` → `htt` |
| 2 | **Reverse lookup** | [`wom_reverse`](recall/wom_reverse.query.adj) | object → subject | `gene_defect($Disease, htt)` → `huntington_disease` |
| 3 | **Enumeration** | [`wom_enumerate`](recall/wom_enumerate.query.adj) | object → {subjects} | `inheritance($Disease, autosomal_dominant)` → `{huntington, marfan}` |
| 4 | **Cross-library join** | [`wom_crosslib`](recall/wom_crosslib.query.adj) | one query, N libraries | `gene_defect` resolved across genetics **and** immuno |

…and beyond this folder, the *same* libraries are reused by:

- **[`board/board_eval.py`](board/board_eval.py)** — the licensing-exam harness lists every
  `*-edges.adj` in `EDGE_FILES` and scores recall/differential/management on the native
  engine (122 recall items, 0 wrong, 100% defensibility at time of writing).
- **[`board/decompose_query.py`](board/decompose_query.py)** — the offline pipeline
  (prose → local model → ADJ query → engine) imports the same libraries; the model only
  *writes the query*, the engine *answers*.

That is six independent consumers of one write, and adding a seventh costs nothing but the
import line.

## What the test pins (and why each matters)

- **(A) Use, not write** — the query files contain no `relate`/`dictionary`/`rulebook`.
  Knowledge lives in the library; consumers only ask. *If a consumer could redefine facts,
  "write once" would be a fiction.*
- **(B) One fact, many views** — the forward and reverse queries over the same edge return
  a **byte-identical** citation. *Reverse lookup is not a second hand-maintained index; it
  is the same written fact read backwards by the SLD resolver.*
- **(C) Provenance is real** — every citation the engine returns appears **character-for-
  character** in the cited library file. *The engine echoes written byte-provenance; it
  does not synthesize a plausible source.*
- **(C′) Correct ownership under composition** — in the cross-library query, `htt`'s span
  comes from genetics-edges and `btk`'s from immuno-edges. *Composition does not blur
  provenance.*
- **(D) Read-only** — the libraries are byte-identical (sha256) before and after all four
  consumers run. *"Write once" is literally once; querying never mutates the source.*
- **(E) Deterministic / offline** — re-running a query yields identical bytes, from the CPU
  engine, with no model in the answer-time loop. *The expensive, fallible model touched the
  fact once, at write time, behind the grounding gate; every later use is cheap, exact, and
  auditable.*

## Why this is the whole thesis in miniature

A large model that answers from weights re-derives — and can re-hallucinate — the fact on
every call, and cites nothing checkable. Here the model's only job was to **write the fact
once**, under an adversarial grounding gate, into an editable content-addressed library.
Every use after that is a CPU query that returns the answer **and the primary-source span
that defends it**. Correcting a fact is a one-line edit to one clause that propagates to all
six-plus consumers at once. That is the payoff of write-once / use-many: intelligence
accumulates in the **grounded library**, not in the weights, and stays auditable and
correctable forever.
