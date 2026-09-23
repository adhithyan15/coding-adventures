# journal-core

The headless engine behind **Journal** — the fourth product of the Mosaic
program ([#14415](https://github.com/adhithyan15/coding-adventures/issues/14415)),
whose bar is Day One. Dated entries across several journals, with tags, stars,
a timeline, "on this day" recall, and full-text search.

## Where this sits in the stack

```
journal-core        ← you are here: model + commands + projections. Pure.
  └ journal-wasm    (J2b) JSON ABI for web, Electron, and every Mosaic host
      …consumed by the Mosaic journal-app (J3)
```

Same split as Trestle's `task-core` and Engram's `engram-core`: the model is
written once, in Rust, and all nine Mosaic backends reach it through one facade
instead of each re-implementing it. The design is
[`code/specs/journal-core.md`](../../../specs/journal-core.md).

It is **pure**: no I/O, no clock (time arrives as `now_ms`), no id generation
(the host mints UUID v7s). `serde` is an opt-in feature; the only dependency is
`datetime-core`.

## Usage

```rust
use journal_core::projections::{on_this_day, search, timeline};
use journal_core::{apply, Command, Date, EntryFilter, EntryId, JournalId, JournalState};

let mut state = JournalState::new(JournalId::from("personal"), "Personal", 0);
apply(&mut state, Command::CreateEntry {
    id: EntryId::from("e1"),
    journal: JournalId::from("personal"),
    date: Date::parse_iso("2025-09-23").unwrap(),
    title: "First light".into(),
    body: "Walked to the lighthouse before breakfast.".into(),
}, 1_000)?;
apply(&mut state, Command::SetTags {
    id: EntryId::from("e1"),
    tags: vec!["Travel".into(), "travel".into()], // one tag: case-insensitive
}, 2_000)?;

let all = EntryFilter::default();
let days = timeline(&state, &all);                       // grouped by day
let recall = on_this_day(&state, Date::parse_iso("2026-09-23").unwrap(), &all);
assert_eq!(recall[0].years_ago, 1);
let hits = search(&state, "lighthouse", &all);           // ranked, with snippets
```

A rejected command returns an `OpError` and leaves the state exactly as it was,
so a host can apply optimistically and keep the old state on error.

## Testing

```sh
cargo test -p journal-core
cargo test -p journal-core --all-features   # includes the JSON wire contract
```
