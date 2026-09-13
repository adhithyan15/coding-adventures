# Lessons Learned

A record of mistakes made during development and the durable fixes for them.
Read the categories relevant to what you are about to touch before starting —
BUILD files, CI, native extensions, and the language-specific traps below are
where this repo has repeatedly cost itself a day.

**One lesson per file.** Every `*.md` beside this one is a single lesson. That
is not tidiness: it is what stops concurrent PRs from conflicting. This used to
be one 640KB `lessons.md`, and because every branch appended to it, every pair
of concurrent PRs collided — additively, resolved by "keep both" every time, and
blocking auto-merge every time.

## Adding a lesson

```bash
python3 code/scripts/lessons.py new "The claim, as a sentence" --category Rust
```

Write the claim as the title. `A grep for a shape finds the shapes it can see`
is a lesson; `Regex bug` is a filing mistake — the title is what someone skims,
and it should state the thing they need to know.

The filename is derived from the title and **is** the lesson's identity. There
is no `id:` field and no ordinal prefix, both of which have already failed here:
an ordinal scheme elsewhere renamed 21 files on a single insertion, and a
separate `id` field once let two branches write `MR-EXT-038` into two
differently-named files, which git merged silently because only the filenames
were compared.

So two branches writing different lessons never conflict, and two branches
writing the *same* lesson land on the same filename and conflict loudly — which
is correct, because they need reconciling.

## Reading them

```bash
python3 code/scripts/lessons.py index            # category, slug, title
python3 code/scripts/lessons.py render           # rebuild the aggregate view
python3 code/scripts/lessons.py validate         # what CI checks
```

`render` writes a single-file `lessons.md` for reading in bulk. It is
**generated and gitignored** — a committed aggregate would reintroduce exactly
the shared file this layout removes. Edit the shard, never the aggregate.

## Categories

- Security boundaries
- Supply chain & CI pinning
- BUILD files & dependency management
- Cross-platform & Windows BUILD_windows
- Workspace & package metadata
- Python
- Ruby
- Lua
- Perl
- Elixir
- Swift
- C#
- Haskell
- TypeScript / JavaScript
- Rust
- Native extensions & FFI
- Compiler / VM / language pipeline
- Cryptography & security review
- Testing & coverage
- CI & GitHub Actions
- QR / format-marker / file-format specifics
- Repo policy / workflow reminders
- Mosaic compiler pipeline

A lesson with no `category:` renders under `Uncategorised`, which is a fine
place to leave one you cannot file confidently. Adding a category means editing
this list — deliberately, in one small file — rather than inventing a heading by
typo.
