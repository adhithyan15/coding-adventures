# CLOC12 — Upstream port gaps

> Status: tracking list. Seeded by CLOC12.01 with no entries.
> Populated as ported upstream tests get marked `#[ignore]`.

## Numbering

Every gap gets a `gap-NNN` ID, allocated in order. Once allocated, an
ID is never reused — even when the gap closes, the entry stays for
historical context with status `RESOLVED` and a link to the fix PR.

## Format

```
## gap-NNN — short title

- **Status:** OPEN | RESOLVED-in-#NNNN
- **Upstream test:** <ClassName>::<testName>
- **Ported file:**   tests/upstream/<file>.rs
- **Why it fails:**  one paragraph
- **What it needs:** one paragraph; usually points at a CLOC11.* slice or
                     a pass-body PR
```

## Entries

_(none yet — first port lands as CLOC12.02 and will allocate gap-001
onward as needed)_
