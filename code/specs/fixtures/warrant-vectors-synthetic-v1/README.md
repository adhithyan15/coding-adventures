# warrant-vectors-synthetic-v1

Synthetic fixtures for the Warrant `vectors` declaration ([VER04j section 5](../../VER04j-warrant-v1-extensions.md),
WX-E).  They exist so the table loader, the generated runner and the sabotage
controls have something real to read.  **They are not a third-party vector set.**

| File | Rows | Columns | Used by |
|---|---|---|---|
| `digests.tfv` | 23 | `msg: Bytes`, `md: Bytes<32>` | `tests/fixtures/wxv.warrant` table `synthetic_digests` |
| `first-bytes.tfv` | 8 | `msg: Bytes`, `out: Result<U8, EmptyErr>` | table `synthetic_results` |

## Provenance

* **Authorship.** The messages are made up by `generate.py` in this directory
  (a byte pattern keyed by the message length, with lengths on both sides of the
  SHA-256 padding boundaries).  They are not text and not taken from NIST CAVS,
  RFC 7748, or any other published vector file.
* **Expected answers.** Python's `hashlib` computes the digests.  The Rust
  implementation under test never produces an expected value.
* **Licence.** Generated data with no creative content; released under the
  repository licence (MIT).
* **Generation procedure.** `python3 generate.py` writes the `.tfv` files and
  prints their SHA-256 pins; `python3 generate.py --check` fails when a checked-in
  file differs from what the script produces (the drift check, run by the
  `warrant-grammar` BUILD).  stdlib only.

## Format

TF01 TFV: `tfv 1`, then one record per row with the fields named by the table's
`column` lines.  `Bytes` are `X<hex>`; a `Result` is the VER04i `M` form
(`M1 S6f6b I7` for `Ok(7)`, `M1 S657272 S454d505459` for `Err(EMPTY)`).

## Pinning

A document pins a fixture by SHA-256 and row count; the generator verifies the
pin *before* it parses the file, copies the verified bytes into the package
(`tests/generated/tables/<name>.tfv`) and the generated test hashes that copy
again.  Changing a byte here without editing the pin in the `.warrant` document
fails generation with W0708.

## Adding a real vector set (an owner decision, not made here)

VER04j open question 9 asks who vets NIST/RFC data before a pin is accepted.
Until the owner decides, this repository ships no such set.  When one is
approved, the hook is exactly this directory layout:

1. add `code/specs/fixtures/<name>-v1/` with the data (as TFV, projected by a
   stdlib-only script with a drift check) and a README recording the source URL,
   the licence and the generation procedure;
2. in the package's `.warrant`, add a `vectors` declaration whose `source` is
   `<name>-v1/<file>.tfv`, whose `sha256` is the file's SHA-256 and whose `rows`
   is its record count;
3. regenerate (`gen_evidence`), which copies the file and embeds its digest.

The loader refuses anything larger than 16 MiB or 100,000 rows, any symlink, and
any digest that does not match; it never fetches a file from the network.
