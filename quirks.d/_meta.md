# Quirks

A record of the rough edges the verification tools meet in the world: a
language, a toolchain, an existing package, a CI runner. It is the companion of
`lessons.d/`: a lesson records a mistake *we* made, while a quirk records a
property of something else that the tools must cope with. Introduced by VER26
(the `ct-compare` pilot), section 6.

**One quirk per file**, named from its title, exactly like `lessons.d/`, so
concurrent PRs never conflict on a shared file.

## Adding a quirk

```bash
python3 code/scripts/lessons.py new-quirk "The claim, as a sentence" \
  --package rust/ct-compare --lane rust --rung E4 --status open
```

## Frontmatter (required, closed)

| Field | Meaning |
|---|---|
| `package` | The package it was found in, or `*` |
| `lane` | The language or toolchain, or `*` |
| `rung` | `E0`-`E6`, `bench`, `report` or `infra` |
| `status` | `open`, `mitigated` (a workaround exists) or `accepted` (a documented limit) |

## What a quirk must never contain

An untriaged counterexample, failing seed, fuzz tape or crashing input (VER00
section 7.3). Name the class of failure and point to the private triage record
instead. `python3 code/scripts/lessons.py validate`, which CI runs, rejects
`seed:`/`tape:`/`counterexample:`/`reproducer:` lines and long hexadecimal runs.
