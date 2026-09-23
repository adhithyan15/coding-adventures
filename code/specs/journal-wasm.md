# journal-wasm — the JSON boundary over journal-core (J2b)

Issue: [#14416](https://github.com/adhithyan15/coding-adventures/issues/14416) (step J2) ·
Core: [`journal-core.md`](journal-core.md)

## What it is

A linear-memory WebAssembly ABI that exposes the pure `journal-core` engine to
JavaScript hosts: the web build, Electron, and the Mosaic `journal-app` (J3). It
follows the repo's `*-wasm` convention exactly as `task-wasm` does it —
`alloc`/`dealloc`, `(ptr, len)` UTF-8 JSON in, `[u32 LE length][UTF-8 JSON]` out
— and holds one `JournalState` for the page.

It adds **no model logic**. Every rule lives in `journal-core`; this crate only
parses, calls, and serialises.

## Envelopes

Every export that returns a value returns a JSON envelope, except `snapshot`,
which returns the raw state. Nothing traps the boundary: parse failures, rejected
commands, and calls before initialisation are all envelopes.

```json
{ "ok": true }
{ "ok": true, "data": … }
{ "ok": false, "error": "entry e9 not found", "code": "entryNotFound" }
```

`code` is a stable camelCase name for the `OpError` variant (`journalNotFound`,
`entryNotFound`, `duplicateId`, `invalidId`, `idMismatch`, `tooManyJournals`,
`duplicateJournalName`, `emptyJournalName`, `invalidJournalName`,
`titleTooLong`, `bodyTooLarge`, `invalidTag`, `lastJournal`,
`moveTargetIsDeleted`), or `parse` / `uninitialised` for boundary failures. Hosts
branch on `code`; `error` is for people.

## Exports

| export | input | data |
| --- | --- | --- |
| `init` | `{ journalId, name, nowMs }` | — (replaces any state) |
| `load` | a snapshot JSON string | — |
| `snapshot` | — | the state as a raw JSON string (`null` before `init`/`load`) |
| `apply` | `{ command, nowMs }` | — |
| `journals` | — | `Journal[]`, alphabetical by name |
| `entry` | `{ id }` | `Entry` |
| `timeline` | `EntryFilter` | `DayGroup[]` |
| `on_this_day` | `{ today, filter }` | `YearGroup[]` |
| `search` | `{ query, filter }` | `SearchHit[]` |
| `tag_counts` | `EntryFilter` | `TagCount[]` |
| `month_activity` | `{ year, month, filter }` | `DayActivity[]` |
| `import_legacy` | `{ journal, entries }` | `{ imported, skipped: [{ index, code }] }` |

`command` is `journal-core`'s `Command` in its serde form (`{"type":"createEntry",
…}`). Dates are ISO `YYYY-MM-DD` strings. Every `filter` may be omitted.

### `load` validates

Deserialising a `JournalState` checks only its *shape*. `load` also runs
`JournalState::validate` and **keeps the current state** unless both pass, so a
corrupt or hostile snapshot is refused whole rather than half-loaded. This is the
obligation `journal-core.md` placed on its loader.

### `import_legacy`

Brings in the TypeScript Journal's stored `Entry[]`:

```ts
{ id: string, title: string, content: string, createdAt: "YYYY-MM-DD", updatedAt: number }
```

Each becomes an entry in `journal`, dated `createdAt`, with both instants set to
`updatedAt` (the TypeScript app never recorded a creation instant). An entry the
core refuses — a bad id, a duplicate, an impossible date, an oversized body — is
**skipped and reported** with its index and code, and the rest still import. An
import is a one-off migration a person watches, so reporting beats refusing the
whole file over one bad row. A row that is not even shaped like an entry fails
the whole call with `parse`, because then the file is not a Journal export at all.

## Output is text, not markup

Titles, bodies, snippets, and tags come back exactly as stored. Hosts must escape
them when rendering; nothing here produces HTML.

## Building

`build-wasm.sh` compiles `wasm32-unknown-unknown` in release and writes
`pkg/journal_engine.wasm`; `js/journal-engine.mjs` is a dependency-free accessor
with one method per export, and `js/smoke.mjs` drives it end to end under Node.
The ABI logic is unit-tested natively (it is plain pointer + JSON code).
