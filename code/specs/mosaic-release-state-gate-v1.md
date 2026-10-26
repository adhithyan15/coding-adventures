# Mosaic release state gate v1

Generalises: [TaskApp local-data operations v1 — Automated upgrade and recovery
gate](task-app-local-data-operations-v1.md#automated-upgrade-and-recovery-gate)

Implementation: `code/scripts/mosaic_release_state_gate.py` (standard library
only), tested by `code/scripts/tests/test_mosaic_release_state_gate.py`.

## Why this exists

Every Mosaic desktop app persists the same way. The generated host asks the
Rust runtime for a `Snapshot` and writes it to a per-user file:

```json
{"bytes":[123,34,...],"schema":"engram-mosaic-app","version":3}
```

`schema` and `version` name the payload's shape; `bytes` is the payload itself,
serde's encoding of a `Vec<u8>` (one JSON integer per byte). The payload is
whatever the app's adapter chose to write — in every app so far, UTF-8 JSON.

A release that cannot read the previous release's file loses the user's data
on the first launch after an upgrade. A host that cannot recognise a damaged
file must not silently overwrite it either; it moves it aside as
`<state>.corrupt` and starts empty. TaskApp's release lane proved both
properties against real extracted bundles from its first release onward. None
of that proof was TaskApp-specific except the final "is my task still there?"
question, so this spec lifts the rest into a gate any app can adopt: Engram
first, then Journal and VisiCalc.

The gate has three halves, each a CLI subcommand:

| Subcommand | Runs | Proves |
| --- | --- | --- |
| `materialize` | before the new build launches | an older release's state is on disk, byte-shaped exactly as that release's host wrote it |
| `verify` | after the new build ran on it | the new build kept it: no quarantine, a compatible schema and version, the expected values |
| `verify-quarantine` | after the new build ran on garbage | the garbage was moved to `.corrupt` byte for byte, not deleted or repaired |

## The fixture

A fixture is a committed JSON file, conventionally
`code/programs/mosaic/<app>/fixtures/release-upgrade-v<version>.json`.

```json
{
  "fixtureSchema": "mosaic/release-upgrade-fixture",
  "fixtureVersion": 1,
  "sourceApp": "engram",
  "sourceVersion": "0.4.0",
  "provenance": "How the state was captured, in prose.",
  "snapshotSchema": "engram-mosaic-app",
  "snapshotVersion": 1,
  "acceptedLiveVersions": [1, 3],
  "expectations": [
    {"liveVersions": [1], "pointer": "/decks/0/name", "equals": "Upgrade fixture deck"},
    {"liveVersions": [3], "pointer": "/session/state/decks/0/name", "equals": "Upgrade fixture deck"}
  ],
  "state": {"decks": [{"name": "Upgrade fixture deck"}]}
}
```

| Field | Type | Meaning |
| --- | --- | --- |
| `fixtureSchema` | string | Exactly `mosaic/release-upgrade-fixture`. |
| `fixtureVersion` | integer | Exactly `1`. A later shape gets a new number, never a silent reinterpretation. |
| `sourceApp` | string | The product, `^[a-z0-9][a-z0-9-]*$` (for example `engram`, `task-app`). Used in error messages. |
| `sourceVersion` | string | The release the state came from. Strict SemVer 2.0.0, no `v` prefix. |
| `provenance` | string, optional | How the state was produced. JSON has no comments; this is the comment. |
| `snapshotSchema` | string | The envelope `schema` that release wrote. Non-empty. |
| `snapshotVersion` | integer | The envelope `version` that release wrote. Non-negative. |
| `acceptedLiveVersions` | list of integers | Every envelope `version` the live file may carry after the new build ran. Non-empty, unique, non-negative. Need not contain `snapshotVersion`. |
| `expectations` | list | Non-empty. Each entry is described below. |
| `state` | object | The semantic payload: the JSON the old release encoded as `bytes`. |

Unknown top-level or expectation keys are rejected, so a typo such as
`expectation` fails loudly instead of disabling a check.

A fixture file must be a regular, non-symlinked file of at most 4 MiB.

### Why `acceptedLiveVersions` is a list

TaskApp's original gate required the live `version` to equal the fixture's.
That holds only while an app never bumps its payload version. Engram's v0.4.0
wrote version 1; the current adapter restores 1 through 3 and *writes* 3. A host
that re-persists after launching therefore turns a v1 file into a v3 file, and a
host that does not leaves it at v1. Both are correct upgrades. Version 2 is
neither: the current adapter never writes it, so a live v2 file after this
upgrade would mean something unexplained happened, and the gate says so.

The list is therefore *what the current code can prove*, not every version the
adapter can read.

### Expectations

```json
{"liveVersions": [3], "pointer": "/session/state/cards/0/front", "equals": "Upgrade fixture front"}
```

- `pointer` is an [RFC 6901](https://www.rfc-editor.org/rfc/rfc6901) JSON
  Pointer evaluated against the **decoded live payload** (not the envelope, and
  not the fixture's `state`). `~1` is `/`, `~0` is `~`, and array indexes are
  decimal without leading zeros. `-` (the past-the-end index) never resolves.
  The empty pointer `""` names the whole document.
- `equals` is any JSON value. Comparison is type-strict: `true` is not `1`, and
  `1` is not `"1"`. Integers and floats compare by numeric value, as JSON does
  not distinguish them.
- `liveVersions` is optional. When present it must be a non-empty subset of
  `acceptedLiveVersions`, and the expectation applies only when the live
  envelope carries one of those versions. When absent the expectation applies
  to every accepted version.

A payload version change usually moves the data — Engram v3 nests the old
collection under `/session/state` — so one pointer cannot name the same value in
both shapes. Scoping by version lets one fixture say where the deck is in each.

**No vacuous pass.** Loading a fixture fails unless *every* accepted live version
has at least one applicable expectation. Without that rule, accepting a new
version while forgetting to describe it would make the gate check nothing for
exactly the case it was extended to cover.

### The state must be captured, never written by hand

`state` must be the real output of the source release's adapter, captured by
running that release's code (for example a throwaway test in a worktree at the
release tag that seeds data through the public API and prints `snapshot()`'s
bytes). A hand-written state proves only that the new build reads what the
author believed the old build wrote. `provenance` records how it was captured.

## `materialize --fixture F --output S`

1. Load and validate `F`.
2. Encode `state` as compact JSON (`,` and `:` separators, sorted keys, UTF-8,
   non-ASCII left unescaped). This is serde_json's own output for a
   `serde_json::Value` (whose maps are sorted), which is what every current
   adapter writes. For Engram v0.4.0 it reproduces the release's bytes exactly.
3. Write `{"bytes":[...],"schema":snapshotSchema,"version":snapshotVersion}`
   (compact, sorted keys) plus one `\n`, as bytes — no platform newline
   translation — to `S`, creating parent directories.
   The file is created with mode `0600`, as it stands in for one person's
   private data.

`S` must not be a symlink or any non-regular file; the gate never writes through
a link into a location the caller did not name.

## `verify --fixture F --state S`

Fails, with a message naming `sourceApp`, unless all of these hold in order:

1. Nothing exists at `S.corrupt` (a dangling symlink counts). A `.corrupt`
   sibling means the new build rejected a compatible file. This is checked
   first because it is the most specific diagnosis: a host that quarantined
   the fixture and then wrote an empty state, or nothing, would otherwise be
   reported as a missing value or a missing file.
2. `S` exists, is a regular file and is not a symlink.
3. `S` is at most 16 MiB, is UTF-8 JSON, and is an object.
4. Its `schema` equals `snapshotSchema`.
5. Its `version` is an integer in `acceptedLiveVersions`.
6. Its `bytes` is a list of integers each in `0..=255` (booleans are not
   integers here, even though Python's `bool` is one).
7. The bytes decode as UTF-8 JSON to an object.
8. Every expectation applicable to that version resolves and equals its value.

## `verify-quarantine --state S --probe P [--app NAME]`

Used after writing invalid bytes to `S`, copying them to `P` and relaunching:

1. `S.corrupt` exists, is a regular file and not a symlink.
2. Its bytes equal `P`'s exactly.
3. If `S` exists, its bytes differ from `P`'s. The host must *move* the rejected
   file aside; a copy left at the live path is quarantined again on every
   launch and never recovers.

`P` is subject to the same regular-file and size rules. No fixture is needed,
because nothing about garbage is app-specific; `--app` only names the product in
error messages.

## Failure modes, summarised

| Symptom | Meaning |
| --- | --- |
| state does not exist | the host wrote elsewhere, or deleted the file |
| `.corrupt` present on `verify` | the new build rejected a compatible file — the upgrade lost data |
| schema mismatch | the host or adapter renamed the payload |
| version not accepted | an unbumped payload change, or an unexplained rewrite |
| invalid bytes | the envelope is malformed (not a 0..255 integer list) |
| pointer missing / value differs | the payload was restored but lost or altered data |
| `.corrupt` missing on `verify-quarantine` | the host deleted or repaired damaged data instead of preserving it |
| `.corrupt` differs from probe | the host rewrote the damaged bytes before preserving them |
| live state equals probe | the host copied instead of moving |
| symlink or non-regular file | refused outright; the gate does not follow links |
| file over the size cap | refused; fixture-seeded state is small, so a huge file is itself a fault |

The CLI exits `0` on success and `1` with a one-line `error: ...` on any
failure. It never evaluates fixture content as code.

## How an app adopts the gate

1. Capture a fixture from the app's latest published release, as above, with
   `acceptedLiveVersions` set to what the current adapter can leave on disk.
2. Add a Rust test to the app's adapter that `include_str!`s the fixture,
   restores its `state` at `snapshotVersion`, asserts the data survives, and
   checks every expectation against the bytes of a fresh `snapshot()` — that is
   what proves the fixture's pointers describe the real current payload rather
   than a guess.
3. In the release workflow, per desktop backend: `materialize` into the
   backend's real live-state path, launch the extracted bundle, `verify`; then
   write invalid bytes, copy them to a probe, relaunch, `verify-quarantine`.
4. Run `test_mosaic_release_state_gate.py` in that workflow and add
   `code/scripts/mosaic_release_state_gate.py` and its test to its `paths:`.

## TaskApp's legacy fixture

TaskApp's `fixtures/release-upgrade-v0.1.0.json` predates this spec and uses
`fixtureSchema: task-app/release-upgrade-fixture`, a single `snapshotVersion`,
and a TaskApp-only `expectedTask` block. It stays readable **through an adapter
in `taskapp_release.py`**, not by converting the file:

- The file is a published contract. `task-mosaic-app`'s Rust test reads it and
  asserts its `fixtureSchema`, and five release-lane steps name it; converting
  it would churn all of them for no behavioural gain.
- `expectedTask` is stronger than a pointer. It requires exactly one task with
  that name anywhere in the workspace, independent of task or project ids; a
  pointer would pin an id that TaskApp is free to change.

The adapter validates the legacy shape exactly as before, builds the gate's
in-memory fixture from it (`acceptedLiveVersions = [snapshotVersion]`, no
pointer expectations), delegates every generic check to the shared module, and
then runs the `expectedTask` walk. Its CLI subcommands
(`materialize-upgrade-fixture`, `verify-upgrade-state`) are unchanged. The
non-empty-expectations rule applies to fixture *files* of the generic schema;
the legacy adapter supplies its own app-specific check in its place.

One behavioural difference: the shared writer emits `\n` as a byte on every
platform, where TaskApp's previous writer used text mode and so wrote `\r\n` on
Windows. Both are the same JSON to every host.
