# HL21 — Sharded curriculum ledgers: no one file may serialize the work

**Status:** specification, 2026-08-24

**Owner's directive:** *"Everything that can be split should be split, and
recombined afterwards. No single file should slow work down or cause
conflicts."*

---

## 1. Why this spec exists

This curriculum is authored by many people at once. On 2026-08-24, four Spanish
tranches were in flight simultaneously. Every one of them had to append to the
same handful of files:

| File | Lines (2026-08-24) | Who touches it |
| --- | ---: | --- |
| `core/book-generation.json` | 6,348 | ALL 23 tracks |
| `spanish/curriculum.json` | 7,865 | every Spanish tranche: `path` + `extensions` + `spine[<node>].segments` |
| `spanish/chapters.json` | 5,951 | every Spanish tranche appends a chapter |
| `core/spine.json` | 592 | ALL 23 tracks |
| `spanish/book/book.tex` | 403 | every Spanish tranche appends an `\input` line |

Those counts are *live*. The brief that commissioned this work measured 6,333 /
7,769 / 5,888 / 592 / 400 the day before. Four of the five grew while the spec
was being written, which is the argument in miniature: these files are hot, and
they are hot for every author at once.

### 1.1 The conflict is structural, not incidental

Appending to the end of a JSON array is the single most conflict-prone edit
there is. Git compares hunks by line; two branches that both append both changed
the last few lines; so **every pair of tranches collides**. Not sometimes —
every pair, every time.

Worse, the merge is *always mechanical and always manual*: no judgement is
required to resolve it, but a human has to be there anyway. That is the worst
possible combination. It is pure serialization cost with no compensating review
value, and it scales quadratically in the number of concurrent authors.

The same reasoning applies to `book.tex`, whose `\input{}` list is appended to by
hand, and which has already been forgotten once — a chapter was generated and
its `\input` line was not added, so the chapter simply did not appear in the
book and nothing failed.

### 1.2 This is not a new direction

PR #12443 already did this once, for the generated hash ledgers. Today these are
per-language directories rather than monoliths:

```text
core/generated-book-hashes/<lang>.json       24 files
core/generated-narration-hashes/<lang>.json  24 files
core/lesson-modality/<lang>.json             23 files
core/gentle-ramp-snapshots/<lang>.json       23 files
```

HL21 generalises that shape rather than adding a second one beside it.

---

## 2. The convention: `X.json` may instead be `X.d/`

For any shardable ledger at path `X.json`, the sharded form is a **sibling
directory** named `X.d`, holding one `*.json` file per element:

```text
core/spine.json                              the monolith
core/spine.d/                                the shards       <- read in preference
  _meta.json                                 the document-level keys
  0010-SPINE-MEET-GREET.json                 one node
  0020-SPINE-COURTESY-THANK.json
```

The `.d` suffix is the Unix `conf.d`/`rc.d` idiom, where it has meant exactly
this — *a directory whose contents are concatenated to form the file this would
otherwise be* — for decades. Reusing a convention readers already know beats
inventing one.

Two agents adding two different elements now write two different filenames, and
git merges them without noticing there was ever a question.

### 2.1 Rule 1 — sorted filename order, compared by CODE UNIT

Shards are merged in sorted filename order. Always. The merged result must be
**byte-identical across machines**, because every generated-artifact `--check`
in this package compares bytes.

Specifically `a < b` on the raw string, **never `localeCompare`**:

- `readdirSync` returns whatever the filesystem hands back. That differs between
  APFS, ext4 and NTFS, and it shifts as files are added.
- `localeCompare` consults the host's collation. Under `en-US` it folds case and
  ignores leading punctuation, so `_meta.json` and `0010-A.json` can swap places
  depending on the machine's locale — the exact non-determinism sorting was
  supposed to remove.

`loader.ts`'s own `sortedEntries` already made this choice, for this reason.
HL21 applies it one layer down.

### 2.2 Rule 2 — order that carries meaning must live in the filename

Because merge order *is* filename order, any ledger whose element order means
something must encode that order in its filenames.

`core/spine.json` is the worked example and the cautionary one. Its 33 nodes run
pre-A1 → C2 and are **not alphabetical within a stage**. Naming the shards
`<NODE-ID>.json` — which is what the commissioning brief proposed, and what
looks obviously right — would have silently re-sorted the ladder while still
"round-tripping successfully". So the shard filename carries a zero-padded
ordinal prefix:

```text
0010-SPINE-MEET-GREET.json
0020-SPINE-COURTESY-THANK.json
```

Zero-padded so that string sort and numeric sort agree; `10` sorting before `9`
is a bug rediscovered in every language that has ever had a `sort`.

Ordinals are **spaced by ten** so a node can be inserted as `0015` without
renaming its neighbours. Renaming neighbours would be its own merge conflict,
which is the thing this work exists to remove. `--check` compares the *rebuilt
monolith*, not the filenames, so a hand-inserted `0015` passes without a
renumber; `--shard` renumbers to the canonical stride when someone chooses to
run it.

A ledger whose ids already sort into authored order does not need the prefix.
That must be **verified, not assumed** — `spanish/curriculum.json`'s `path` and
`extensions` ids look sequential (`ES-PATH-001`, `ES-EXT-003-GRAMMAR`) and do
**not** in fact sort into authored order.

### 2.3 Rule 3 — fall back, never guess

If `X.d/` does not exist, `X.json` is read exactly as before. That is what lets
ledgers migrate one at a time, with no flag day and no PR that must move data
and change code at once. A migration that can be done in small pieces will be;
one that cannot, won't.

But if `X.d/` **does** exist and holds no `*.json` shards, that is an **error**,
not an empty ledger. "No spine on disk" and "a spine with no nodes" are opposite
facts, and a loader that returns the second when it means the first hands every
downstream gate a clean bill of health for a corpus that is not there.
`loadModalityManifest` already makes this call, in these words.

### 2.4 `_meta.json`

Every ledger is "a few document-level fields, plus one long array that everybody
appends to". The array is the part that causes conflicts, so the array becomes
one file per element and everything else rides in `_meta.json` — which nobody
appends to, and which therefore nobody collides on.

`_meta.json` is **required**, never defaulted to `{}`. A rebase that dropped it
would otherwise read as a spine that legitimately has no stages.

The leading underscore sorts it away from the element shards under code-unit
order (`_` is 0x5F, above every digit and every uppercase letter) and reads as
"not one of the things" to anyone listing the directory.

### 2.5 The array must be the last top-level key

`JSON.stringify` emits keys in insertion order, so a byte-exact rebuild needs
the array to land back where it started. Rather than invent a `_listAfter`
metadata key to record a position no current ledger needs, the array is appended
last and **`--shard` refuses a ledger whose array is not already last**.

`core/spine.json` is `{ version, stages, strands, strandNote, nodes }`, so
appending is exact. A later ledger that keeps its array in the middle gets a
clear refusal, and whoever migrates it decides then — which beats a silent
reordering discovered as a `--check` failure nobody can account for.

### 2.6 Filenames are validated, not trusted

The shard filename is derived from a field in authored JSON. An id of
`../../../etc/passwd` deciding where a tool writes is exactly the bug that a
"the ids are fine" comment produces two years later. So `--shard` refuses:

- an id that is not `^[A-Z][A-Z0-9-]*$`;
- a Windows reserved device name (`CON`, `PRN`, `AUX`, `NUL`, `COM1`–`COM9`,
  `LPT1`–`LPT9`) — `CON.json` cannot be checked out on Windows at all, so such a
  shard set would silently fail on half the machines that use it;
- a duplicate id, which would overwrite a sibling and lose an element in
  silence.

Command-line paths go through a containment check decided *after* `resolve`,
never by inspecting the string for `..` — `a/b/../../../etc` contains no leading
`..` and still escapes.

### 2.7 Shards may not leave the checkout

git tracks symlinks as first-class objects, so `core/spine.d -> ~/.aws` is
something a pull request can contain. The loader therefore uses `lstatSync`, not
`statSync`, and **refuses** a symlinked shard directory, a symlinked `*.json`
shard, and a symlinked monolith, rather than following any of them. Once
`--unshard` writes the monolith back out from the shards, "reads a file outside
the tree" would otherwise become "commits a file from outside the tree".

Relatedly: parse failures must not echo file bytes. V8 splices the offending
content directly into its `SyntaxError` message, `--check` runs in CI, and CI
logs are read far more widely than the repo. Both the message **and the `cause`
chain** are scrubbed — the cause is what Node's default handler and Vitest
actually print.

---

## 3. Generated artifacts are regenerated, never hand-merged

Some monoliths survive as **generated artifacts**. The rule for all of them:

> A derived file that nothing verifies is worse than no file. It looks
> authoritative, it is committed, it is read by the app — and it drifts the
> first time somebody edits a source and forgets to re-run the generator.

Therefore every generated monolith has a `--check` that runs in CI, and:

- **Never hand-edit a generated artifact.** Edit the shards and regenerate.
- **Never hand-merge a generated artifact.** On a conflict, take either side,
  re-run the generator, and commit the result. Hand-merging a generated file
  produces bytes that no generator would ever emit, and the next `--check`
  fails for reasons nobody can read.

This is already the practice for `core/lesson-modality/`, the generated `.tex`
chapters, and the hash ledgers. HL21 extends it to `core/spine.json` and
(per §5) to `<track>/book/book.tex`.

---

## 4. Status: what has been migrated

### 4.1 `core/spine.json` — DONE

```text
core/spine.d/_meta.json                          version, stages, strands, strandNote
core/spine.d/NNNN-<NODE-ID>.json                 one node each (33 nodes)
```

The shards are the source of truth. **`core/spine.json` is kept, as a generated
artifact**, and that decision has one specific cause:

```ts
// code/programs/typescript/language-ladder/src/curriculum.ts:5
import spineJson from "../../../../learning/human-languages/core/spine.json";
```

That is a static JSON import resolved by Vite into a **browser** bundle, and a
browser cannot `readdirSync`. Rewriting it as `import.meta.glob` was considered
and rejected: the app enforces a 500 kB eager-bundle budget in
`scripts/check-bundle.mjs`, glob imports interact with that budget in ways that
need their own investigation, and this migration should not have to reason about
chunk splitting to land. Keeping a generated monolith costs one `--check` in CI
and nothing else.

It was verified that this is the *only* direct read: a repo-wide grep for the
literal `spine.json` found this import plus documentation references. Every
filesystem-side consumer goes through `loadCurriculumSpine`.

Round-tripping is byte-exact: `unshard(shard(core/spine.json))` reproduces the
committed file with SHA-256
`c230a32258b7c2f492221a22edac19b429c3e357f15cebfd2ac062f80fe19098` unchanged,
asserted by a test over the real ledger rather than only a fixture.

### 4.1a `<track>/chapters.json` — DONE for 20 of 23 tracks

```text
<track>/chapters.d/_meta.json      version, language, note
<track>/chapters.d/<NNNN>.json     one chapter each, named for the chapter number
```

Round-tripping is byte-exact for all twenty, verified against the real committed
ledgers rather than a fixture; the SHA-256 of each rebuild is recorded in the
package CHANGELOG.

Three findings, all of which contradict something this spec assumed:

1. **§5.1 said "no separate prefix is needed".** That is true of the *prefix*
   but not of the *padding*: the chapter number alone, unpadded, re-sorts every
   one of the twenty tracks. Eleven chapters is enough — `10.json` and
   `11.json` both sort before `2.json`. The shard name is the chapter number
   zero-padded to four digits.

2. **`french`, `japanese` and `marwadi` do not round-trip.** Their committed
   `chapters.json` is hand-formatted with inline one-line arrays that
   `JSON.stringify(x, null, 2)` expands. The data is identical; the bytes are
   not. Per §8.5 they are reported rather than reformatted, and keep their
   monoliths. This is a decision waiting for an owner, not an oversight.

3. **The monolith could not be deleted, and §5.1 did not anticipate why.**
   See §4.3.

### 4.3 The eager glob-table cost — why browser-read ledgers keep their monoliths

§4.1 explains `core/spine.json`'s surviving monolith as a *static import* a
browser cannot follow. The real constraint is broader, and it decides the
disposition of every ledger `language-ladder` reads.

`bookhashes.ts` already reads `<track>/chapters.json` through
`import.meta.glob` — the browser equivalent of `readdirSync`, resolved by Vite
at build time. A glob's **modules** are lazy. Its **key table** is not: Vite
expands the glob into one string key plus one `() => import(…)` arrow per
matching file, as ordinary code in the importing module, and that module is on
the eager path.

Sharding takes the match count from 23 files to ~1,020. Measured:

| | largest eager chunk |
| --- | ---: |
| before | 312,216 bytes |
| with `chapters.d/` globbed | 503,765 bytes |

which is through the hard 500 kB ceiling in
`language-ladder/scripts/check-bundle.mjs`. That ceiling is a debt ceiling and
was not raised.

The dodge worth naming, because it looks free and is not: move the four
capability fields into `core/generated-book-hashes/<lang>.json`, which the app
already loads lazily in 23 pieces. That would work and would cost a real check.
The app recomputes each chapter's fingerprint from the **currently authored**
capability, so a capability edited without regenerating the book reads as
`stale`. Sourcing the capability from the same generated file as the hash it is
compared against would make that comparison agree with itself, and the drift it
exists to catch would go silent.

So: **a ledger the browser globs keeps its monolith as a generated artifact.**
The conflict is not removed, it is downgraded — from a hand-merge of JSON to
"take either side, re-run `npm run unshard`", which is §3's rule and is
mechanical. Removing it outright needs the app to stop reading authored ledgers
at all, which is its own piece of work with its own trade to decide.

This applies directly to `<track>/curriculum.json` (§5.2), which the app globs
**eagerly** — a stricter case again.

### 4.2 Tooling

```sh
npm run shard    core/spine.json    # split a monolith into X.d/
npm run unshard  core/spine.json    # rebuild the monolith from X.d/
npm run check:shards                # assert the two agree; what CI runs
```

`check:shards` is wired into `.github/workflows/human-languages-books.yml` and
`code/scripts/verify-human-languages.sh`, beside `check:books`.

---

## 5. Migration plan for the ledgers not yet touched

These were **deliberately not migrated** in the first three PRs: four agents were
actively appending to them, and reshaping a file underneath an open branch
breaks that branch. Each migration below should land when its file is quiet.

### 5.1 `<track>/chapters.json` → `<track>/chapters.d/<NNNN>.json`

**Shard key:** the chapter number, zero-padded, so sorted order is numeric order.

Shape today (`spanish`): `{ version, language, note, chapters[305] }`. The
`chapters` array is already the last top-level key, and chapter numbers are
contiguous `1..305`, so the chapter number *is* the natural ordinal — no
separate prefix is needed and no id-safety question arises.

**Difficulty: low.** This is the cleanest of the remaining migrations and should
go first.

> **DONE for 20 of 23 tracks — see §4.1a.** Three corrections to the paragraph
> above, all found by doing it. The chapter numbers are contiguous `1..n` as
> claimed, but "no separate prefix is needed" understates the requirement: the
> number must be ZERO-PADDED or sorted filename order re-sorts every track in
> the corpus. `french`, `japanese` and `marwadi` do not round-trip byte-exactly
> and were left alone. And the monolith could not be deleted, for the reason in
> §4.3 — which is the finding that matters most here, because it governs §5.2
> and §5.3 as well.

### 5.2 `<track>/curriculum.json` → four sibling directories

Shape today (`spanish`):

| Key | Shape | Shard as |
| --- | --- | --- |
| `version`, `language` | scalars | `_meta.json` |
| `path` | array[298], ids `ES-PATH-001`… | `path.d/<NNNN>-<PATH-ID>.json` |
| `spine` | **object**, 33 keys, each `{ segments, omits, relocates }` | `spine.d/<NODE-ID>.json` |
| `extensions` | array[293], ids `ES-EXT-001-WRITING-RUNWAY`… | `extensions.d/<NNNN>-<EXT-ID>.json` |
| `conceptAliases` | object, 13 keys | left whole, in `_meta.json` |

**`spine` is the single worst conflict point in the entire corpus** — every
content tranche appends to `spine[<node>].segments`, and there are only 33 nodes
for 23 tracks' worth of authors to collide on. It is keyed by node id, so it
needs no ordinal: an object has no meaningful order, and `<NODE-ID>.json` is
both stable and safe.

`path` and `extensions` **do** need ordinal prefixes. Their ids were checked and
are unique and filename-safe, but they do *not* sort into authored order despite
looking sequential. This is the §2.2 trap, live.

**Difficulty: medium.** This needs `shard-cli` extended from "one array in one
`.d/`" to "several keys, each in its own sibling `.d/`", plus an object-valued
shard mode for `spine`. That is a real but contained generalisation of
`ShardPlan`.

> **DONE for 22 of 23 tracks.** `marwadi` is left on its monolith: its `lessons`
> arrays are written inline on one line, so the bytes do not round-trip. Data
> identical, reported not reformatted, per §8.
>
> The monolith is KEPT as a generated artifact, per §4.3 — `language-ladder`
> globs `*/curriculum.json` and a glob's key table is eager code. The conflict on
> `spine[<node>].segments` is downgraded to "regenerate, do not hand-merge",
> not removed. A deliberate trade: not raising the app's 500 kB debt ceiling,
> and not weakening the staleness check that the alternative would have cost.
>
> **Three corrections to the table above, found by doing it:**
>
> 1. **`spine` DOES need an ordinal, and the reasoning above is wrong.** "It is
>    keyed by node id, so it needs no ordinal: an object has no meaningful
>    order" is true of JSON semantics and false of this ledger.
>    `JSON.stringify` emits object keys in INSERTION order; **no** track has its
>    spine keys in sorted order; and all 23 list them in exactly
>    `core/spine.d/`'s ladder order, pre-A1 → C2. It is the shared ordered
>    ladder, mirrored per track. `<NODE-ID>.json` shards merged in sorted order
>    would have scrambled it in 23 files at once, silently, while still
>    "round-tripping successfully" — the §2.2 trap, at the one place this spec
>    said it did not apply.
>
> 2. **`path`/`extensions` ordinals confirmed needed, but not universally.**
>    Spanish diverges at index 3: authored `ES-PATH-004` against sorted
>    `ES-PATH-003-CASA`, because a bare prefix sorts before the same prefix
>    extended. That holds for 20 of the 22 tracks — `japanese` and `urdu` happen
>    to have both lists already in sorted order and would coincidentally survive
>    losing their ordinals. The convention still applies to all 22: those two are
>    one authored id away from joining the other twenty, and nothing would
>    announce it.
>
> 3. **The arrays are not last, and §2.5's refusal had to go.** Every track is
>    `{version, language, path, spine, extensions}`, Spanish adding
>    `conceptAliases`. `_meta.json` now records the top-level key order in
>    `_keys` — but only when the sharded keys are not already a suffix, so only
>    `spanish` has one and no previously-committed shard set changed.

### 5.3 `core/book-generation.json` → `core/book-generation.d/<language>.json`

Shape today: `{ version, sourceBaseUrl, scriptSets, referenceAppendices[6],
glossaries[23], answerKeys[23], indexes[23], targets[949], handwritten[71] }`.

**Every one of the six arrays carries a `language` field on every element** —
that was checked, not assumed. So the per-language file is the right home for
all of them, and the brief's open question ("`scriptSets`,
`referenceAppendices`, `glossaries`, `answerKeys`, `indexes`, `handwritten` need
a home too — decide and document") resolves cleanly:

| Key | Home | Why |
| --- | --- | --- |
| `version`, `sourceBaseUrl` | `_meta.json` | document-level scalars |
| `scriptSets` | `_meta.json` | keyed by *script set*, not language; genuinely shared (8 keys) |
| `referenceAppendices` | `<language>.json` | 6 entries, all language-keyed |
| `glossaries`, `answerKeys`, `indexes` | `<language>.json` | 23 entries each, exactly one per language |
| `targets` | `<language>.json` | 949 entries, 23 languages — the bulk |
| `handwritten` | `<language>.json` | 71 entries across 14 languages |

So `core/book-generation.d/spanish.json` holds Spanish's slice of all six
arrays, and a Spanish tranche touches only that file.

> **Blocker, found while writing this spec.** `targets` is **not contiguous by
> language**. It has 27 runs for 23 languages: `hindi`, `kannada`, `spanish` and
> `telugu` each appear in two separate runs. A per-language split would merge
> those runs and therefore **would not round-trip byte-exactly** — it would
> reorder `targets`.
>
> This migration therefore needs a **one-time normalization commit** that groups
> `targets` by language, landed when no tranche is in flight, before the shard
> can be lossless. Whoever does it must first confirm that nothing depends on
> the current `targets` order (chapter emission order and the derived `book.tex`
> `\input` list are the things to check).

**Difficulty: medium-high**, entirely because of that normalization step.

> **THE BLOCKER ABOVE RESOLVED ITSELF. A DIFFERENT ONE REPLACED IT.**
> The machinery is built and tested; `BOOK_GENERATION_PLAN` is exported from
> `shard-cli.ts` and is deliberately **not** in `SHARD_PLANS`, one line from
> being enabled.
>
> Re-measured at 1,007 `targets` (this spec was written at 949): every one of
> the six arrays is contiguous by language, and `targets` is **23 runs for 23
> languages**, not 27. The split runs for hindi, kannada, spanish and telugu
> closed as later tranches inserted into them. All six arrays are additionally
> in the same alphabetical language order, which is also sorted `<language>.json`
> order — so a per-language split reproduces authored order with **no ordinal
> prefix and no normalization commit**. A test pins that contiguity, so an
> append to the end of `targets` reopens the question loudly.
>
> What blocks it now is formatting. `core/book-generation.json` does not
> round-trip through `JSON.stringify(…, null, 2)` at all:
>
> - 74 differing lines, a contiguous run at **2911–2984**;
> - identical line count either way (6,658);
> - **every difference is leading whitespace only** — twelve `marwadi` entries
>   in `targets` indented two spaces deeper than canonical, a hand-merge artifact;
> - `JSON.parse` of either form is deep-equal to the other.
>
> Sharding proves it: the rebuilt document is byte-identical to the CANONICAL
> reserialization and differs from the COMMITTED file by exactly those 74 lines.
>
> Unlike `chapters.json` and `curriculum.json`, this cannot be worked around by
> skipping a track — it is one file shared by all 23. So the whole ledger waits
> on a deliberate re-indent commit whose entire content is the reformatting. A
> test states the blocker as an executable fact and **fails the day the file is
> re-indented**, which is exactly when the plan should be enabled.
>
> `scriptSets` → `_meta.json` confirmed: 8 keys, keyed by script set, carrying no
> `language`. Every element of the other six arrays carries one.

### 5.4 `<track>/book/book.tex` → **generated, not sharded**

Do not shard it. **Generate it**, and gate it with `--check`. See §6.

---

## 6. `book.tex` is two files glued together

`book.tex` is not one artifact. For Spanish (403 lines) it is:

- **lines 1–85: genuinely AUTHORED front matter.** `\documentclass`,
  `\input{preamble}`, title, author, the `titlepage` block, the CC BY-SA licence
  paragraph, the Preface, `\frontmatter`, `\tableofcontents`, `\mainmatter`.
  Per-track, stable, and no tranche ever touches it.
- **the rest: pure DERIVED data.** A flat `\input{chapters/chNNN-<slug>}` list,
  every entry of which is reconstructible from `chapters.json` (chapter number,
  label) plus `core/book-generation.json` (the `output` path), followed by
  `\backmatter` and the appendix inputs, which come from
  `referenceAppendices` / `glossaries` / `answerKeys` / `indexes`.

Sharding it would be the wrong tool. The `\input` list is not content anybody
should be editing at all; it is a projection of two ledgers that already exist.
So split it by **origin**, not by size:

```text
<track>/book/frontmatter.tex   authored, hand-maintained, rarely changes -> no conflict
<track>/book/backmatter.tex    authored, if appendix ordering needs it
<track>/book/book.tex          GENERATED = frontmatter + derived \input list + backmatter
```

Emitted from `book-cli` alongside everything else, so it gets `--write` and
`--check` exactly like the chapter files, the narration and the hashes already
do.

This removes the conflict entirely *and* removes a hand-maintenance step that
has already been forgotten once. Chapters are already generated per chapter by
`book-cli`, so the lesson → chapter → book granularity is complete below this
level; this closes the last hand-maintained link in the chain.

**Fidelity requirement.** For all 23 tracks the generated `book.tex` must be
byte-identical to what is on disk before any new chapters are added. The
generator lands with `--check` passing against the current hand-maintained
files. If a track's `book.tex` has an ordering quirk the ledgers do not capture,
that is a **finding to report**, not a file to quietly "fix" into agreement with
the generator.

### 6.1 The compile gate

Nothing currently checks that the LaTeX actually **compiles** — only that files
exist and hashes match. That is a real gap: `src/book.ts`'s escape map was once
found to be missing a `ǵ`, which is exactly the class of bug only a compile
catches.

The gate must be **opt-in and fast to skip**. Compiling all 23 books is ~100
seconds (HL-C213), which is too slow for the default unit-test path. It is a
separate script, never wired into `vitest run`.

---

## 7. What this does not do

- It does not change any ledger's **content**, only its storage.
- It does not remove `--check`ed monoliths where a browser bundle needs them.
- It does not touch the lesson Markdown, which is already one file per lesson
  and has never been a conflict point.

---

## 8. Checklist for migrating the next ledger

1. Confirm the array is the last top-level key, or move it.
2. Confirm element ids are unique and match `^[A-Z][A-Z0-9-]*$`, and are not
   Windows reserved names. **Check; do not assume.**
3. Decide whether element order carries meaning. If it does, confirm whether
   sorted-id order already reproduces authored order — and expect the answer to
   be no. Add ordinal prefixes if needed.
4. Grep for direct reads of the monolith (`grep -rn '<name>.json'`). Route them
   through the loader, or keep the monolith as a generated artifact with a
   `--check` and say why.
5. Add a `ShardPlan` entry, run `--shard`, and assert the round trip is
   byte-identical **against the real ledger**, not only a fixture.
6. Wire `--check` into CI and `verify-human-languages.sh`.
7. Land it when the file is quiet. Reshaping a file underneath an open branch
   breaks that branch.
