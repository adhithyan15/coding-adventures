# PROPOSED: G-M1 registration (VER13 slice, PIF-016a and ADL-006a)

**Status: proposed, REQUIRES OWNER SIGNATURE. Nothing in this directory's signed
or pinned files has changed.** `catalogue-v1.json`, `buckets-v1.json`,
`evidence-kinds.json` and `non-claims.json` are byte-identical to `main`, so no report's
catalogue pin (AC-14) changes and the release gate behaves as before.

Drafted 2026-11-25. Sources: [VER32](../specs/VER32-proof-interchange-format.md) 9.2 and 11
(PIF-016a), [VER33](../specs/VER33-warrant-algorithm-definition-language.md) 9.1 (ADL-006a),
[VER11a](../specs/VER11a-lean4-proof-kernel.md) 11.2 (the `kernel.*` ids),
[VER34](../specs/VER34-constraint-vm-smt-solver.md) 6.7 (`tcb.vcgen.claim-faithful`), and
[VER31](../specs/VER31-tiered-release-standard.md) section 13.7 (the human table).

## The two proposal files

| File | What it is |
|---|---|
| `catalogue-v2.proposed.json` | `catalogue-v1.json` with its 84 entries byte-for-byte unchanged, plus 13 appended entries (97). Same schema name, `ver13-assumption-catalogue-v1`. |
| `buckets-v2.proposed.json` | `buckets-v1.json` with `revision: 2` and the 13 new ids bucketed (97). The 84 existing entries are unchanged. **Marker:** `decided.by` reads `PROPOSED - requires owner signature` (the schema has no other free field, and the gate refuses an unsigned file anyway). |

When applied, the contents of `catalogue-v2.proposed.json` replace `catalogue-v1.json` (the file
name never changes, VER13 5.2) and those of `buckets-v2.proposed.json` replace
`buckets-v1.json`. The `.proposed.json` names exist only so that this proposal cannot be
mistaken for a signed file.

## Diff summary

Added ids, with the bucket each proposes and how it closes. No existing id changes bucket,
layer or text; no bucket moves.

| Id | Layer | Proposed | Closed by |
|---|---|---|---|
| `kernel.checker.correct` | `wir` | (c) | Independent agreement per claim: Lean's kernel at the pin (PIF-010) and our second checker (KRN-019); VER11a Q6, route R1 |
| `kernel.logic.consistent` | `wir` | (c), open | Owner decision on VER11a Q5 (proposed (a), a mathematical floor). Status `floor`, no discharge, floor flag true. **Registered as (c): this proposal does not decide Q5** |
| `spec.goal.faithful` | `spec` | (b) | Signature on the `pif-signoff 1` goal lines and `goals-sha256` (VER31 13.1) |
| `pif.reader.correct` | `wir` | (c) | A second reader (KRN-019) and the Lean replay harness (PIF-010) each read the same PIF bytes; not a test campaign |
| `pif.prelude.faithful` | `spec` | (b) | Signature on the PIF prelude digest, read through the printer |
| `tcb.pif.gate-binding` | `wir` | (c) | Falsification tests and fuzzing of `pif-check`'s binding (PIF-003) |
| `tcb.pif.printer` | `wir` | (c) | Gate checks both round trips, goals and files (VER32 9.2) |
| `gates.model.faithful` | `spec` | (b) | Signature on the gate model file digest and the goals list |
| `gates.model.code-link` | `wir` | (c) | G-M2: replaced by the `wir.*` ids (PIF-016b) |
| `warrant.model.elaboration-faithful` | `wir` | (c) | Mechanical evidence only: second elaborator, `Eq.refl` differential, specification lemmas. Never a signature |
| `adl.prelude.faithful` | `spec` | (b) | Signature on each ADL layer digest |
| `adl.wir-abi.faithful` | `wir` | (b) | Signature on the encoding file digest. Opened at A3; registered now because PIF-016a names it |
| `tcb.vcgen.claim-faithful` | `wir` | (c) | Retiring the DRAT route or a kernel-reconstructed proof. Opened at A1; registered now because ADL-006a and PIF-016d name it |

Counts after: 97 ids, (a) 35 (unchanged), (b) 10 (was 5), (c) 52 (was 44).

Choices a reviewer should know about:

- `logic_gates.model.faithful` is **not** registered. VER33's scheme `<pkg>.model.faithful`
  is instantiated by VER32 9.2 as `gates.model.faithful`, which this proposal keeps.
- No new evidence kind is defined. The entries reuse kinds already in `evidence-kinds.json`
  (`independent-checker-cross-check`, `mechanised-checker`, `differential-run`,
  `negative-control`, `falsification-test`, `checker-fuzzing`, `checker-corpus`,
  `spec-sync-test`, `sabotage-corpus`), so that file, and every report's pin of it, does not
  change. The one real gap is a kind for a kernel-checked **link theorem**
  (`gates.model.code-link`); PIF-016b defines it at G-M2.
- Layers are chosen as follows: faithfulness of what we *mean* (the (b) ids) is layer `spec`;
  trust in a tool or in the pipeline's connection to code is layer `wir`, where VER13
  consumes every `tcb.*` entry. The owner may prefer otherwise; the layer feeds the
  `proved-to-L` label rule, not the bucket.
- `discharge.backlog` must name a row of `VER-BACKLOG.md` (not a shard), so the ADL ids cite
  `G-M1`, `PIF-016a`, `PIF-016d`, `PIF-003`, `PIF-004`, `PIF-010`, `PIF-019`, `PIF-020` or
  `KRN-019`; the ADL rows themselves are named in each `plan`.

## What the owner must sign, and how

1. **Approve the VER31 amendment, section 13**, by approving the pull request. It says what
   the new ids mean and what a sign-off closes.
2. **Land the format change the proposal needs** (see "Why it does not validate yet"), and add
   the 13 entries to VER13 section 3 and to `generate_assumption_catalogue.py`'s overlay so that
   `generate_assumption_catalogue.py --check` and the AC-14 spec-sync pass. This is
   not done here. Regenerate every report's catalogue pin afterwards
   (`ca-assurance-report --generate`), because the catalogue digest changes.
3. **Sign `buckets-v1.json` revision 2.** After step 2, copy the proposal over
   `buckets-v1.json`, then edit `decided` to name the owner and the signing date
   (`"by": "owner"`). Sign those exact bytes:
   `ssh-keygen -Y sign -f <hardware key> -n coding-adventures-buckets code/assumptions/buckets-v1.json`
   and commit the resulting `buckets-v1.json.sig`. **The signature covers the exact bytes of
   `buckets-v1.json`**: revision 2, all 97 ids, each id's bucket and floor flag, and
   `decided`. It does *not* cover `buckets-v2.proposed.json`, whose marker makes its bytes
   differ. The gate refuses a revision lower than the highest ever on `main` (revision 1 today),
   so 2 is the least that works.
4. Later, at G-M1 signing time, sign the **sign-off** (`<stem>.pif-signoff`) under VER31 13.1. That
   closes the (b) ids above for the logic gates and is a separate act on a separate file.

## Validation

Run from the repository root with Python 3.11 or newer (`release_gate.py` imports `tomllib`):

```
python3 code/scripts/assumption_catalogue.py          # the live files; must stay green
```

The proposal is checked with the repository's own checker functions
(`validate_structure`, `check_identity_and_graph`, `check_entry_shapes`,
`check_backlog_refs`, `check_evidence_kinds` from `assumption_catalogue.py`, and
`release_gate.load_buckets` for coverage, `blocks_proved` and floor agreement):

| Check | Result on the proposal |
|---|---|
| `release_gate.load_buckets(buckets-v2.proposed.json, catalogue-v2.proposed.json)` | OK: revision 2, 97 ids, (a) 35, (b) 10, (c) 52 |
| Identity, graph, shape and backlog-reference rules | no findings |
| Evidence-kind definitions (every named kind defined) | no findings |
| Structure check with the live family list | **13 entries fail only on `family`** (below) |
| Structure check with the five families added in memory | no findings |

### Why it does not validate yet

The catalogue's family list is closed in three places that must change together:
`FAMILIES` in `code/scripts/assumption_catalogue.py`, the same list in
`code/packages/rust/ca-assurance-report/src/catalogue.rs`, and the `family` enum of
`code/specs/schemas/ver13-assumption-catalogue-v1.schema.json`. The new ids have the
families `kernel`, `pif`, `gates`, `warrant` and `adl` (the id prefix, rule AC-14), and
VER32, VER33 and VER11a fixed those spellings. Extending the three lists is a small change,
but it touches a checker the gate trusts and its Rust twin, and it is **not made here**.
`spec` and `tcb` already exist and need nothing.
