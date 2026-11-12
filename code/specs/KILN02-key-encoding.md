# KILN02 — Kiln Key Encoding, Version 1

## Status

Accepted (2026-11-11). Implements §8 of
[KILN00](KILN00-kiln-build-system.md). Key-schema version **1**.

## 1. Why a key needs its own spec

A Kiln key is the answer to one question: *"has exactly this work already
passed?"* Two machines, two operating systems, or two implementations in two
languages must compute **the same 32 bytes** for the same action, or the
ledger silently stops producing hits. Worse, two *different* actions must
never produce the same bytes, or the ledger produces a false hit — the one
failure Kiln is designed never to have.

Both properties come from the **encoding**: how an action's description is
turned into the byte string that SHA-256 hashes. A careless encoding — say,
joining fields with commas — lets two different actions collide:

```
env name "a,b" = value "test"    →  "a,b,test"
env name "a"   = value "b,test"  →  "a,b,test"     ← same bytes, different work
```

So the rule throughout this spec is: **every variable-length value carries
its own length, and every section carries its own name.** Nothing is ever
inferred from a separator. This is the same idea as the length-framed source
hashing already used by the build tool's `internal/hasher`.

## 2. Building blocks

Everything is built from five encodings. `||` means concatenation.

| Name | Encoding | Example |
|---|---|---|
| `u8(n)` | one byte | `u8(1)` = `01` |
| `u64(n)` | eight bytes, unsigned, big-endian | `u64(3)` = `00 00 00 00 00 00 00 03` |
| `bytes(b)` | `u64(len(b)) || b` | `bytes("hi")` = `00…02 68 69` |
| `str(s)` | `bytes(UTF-8 of s)`; `s` must be valid UTF-8; no normalisation | `str("é")` = `00…02 c3 a9` |
| `list(xs)` | `u64(count) || x₁ || x₂ || …` | `list([])` = `00…00` |

Strings are encoded **byte-exactly**: Kiln never applies Unicode
normalisation, case folding or line-ending conversion. Two strings that look
the same but differ in bytes are different strings.

## 3. The preimage

The **preimage** is the byte string that gets hashed. It is a fixed header
followed by nine **sections** in a fixed order. Each section starts with its
name as `str(name)`, so a section can never be mistaken for another.

```
preimage = str("kiln-key")
        || u64(schema_version)          -- 1 for this spec
        || u64(semantics_version)       -- expansion semantics, KILN01
        || section("platform")
        || section("host")
        || section("action")
        || section("recipe")
        || section("inputs")
        || section("reads")
        || section("consumed")
        || section("tools")
        || section("env")

key = SHA-256(preimage)     -- written as 64 lowercase hex characters
```

All nine sections are always present, even when empty, so the position of
every section is fixed.

### 3.1 `platform`

```
str("platform") || str(os) || str(arch)
```

`os` is one of `linux`, `macos`, `windows`. `arch` is one of `x64`, `arm64`.
Any other value is an error.

### 3.2 `host`

```
str("host") || u8(0)                      -- no host fingerprint
str("host") || u8(1) || str(image_id)     -- with one
```

Present (tag 1) exactly when the action uses a tool that the Managed provider
did not supply (KILN00 §8, item 3). `image_id` is whatever `Host.ImageID()`
returns, byte-exact.

### 3.3 `action`

```
str("action") || str(target) || str(phase)
```

`phase` is one of `fetch`, `build`, `test`. `target` is the target's full
name, e.g. `code/packages/rust/parser:lib`.

### 3.4 `recipe`

```
str("recipe") || bytes(canonical_recipe)
```

The canonical expanded recipe of **this phase only**, as produced by
expansion (KILN01). KILN02 treats it as opaque bytes. For a legacy action
(KILN00 §15.1) it is the concatenation, framed, of the legacy files:

```
canonical_recipe = str("legacy-shell") || list([ file(BUILD), file(BUILD_windows)? ])
file(name)       = str(name) || bytes(contents)
```

where `BUILD_windows` is included only if it exists. An existing but empty
`BUILD_windows` is included (as an empty file) and so gives a different
recipe from an absent one.

### 3.5 `inputs` and 3.6 `reads`

```
str("inputs") || list(entries)
str("reads")  || list(entries)

entry = str(path) || u8(kind) || u8(exec) || bytes(digest)
```

| Part | Meaning |
|---|---|
| `path` | Repository-relative, `/`-separated, valid UTF-8. No leading `/`, no empty, `.` or `..` segments, no trailing `/`. |
| `kind` | `1` regular file, `2` symbolic link. Directories are not entries. |
| `exec` | `1` if the file is executable as `Source` reports it (Git's `100755`), else `0`. Always `0` for links. |
| `digest` | For a file: the 32-byte SHA-256 of its contents. For a link: the 32-byte SHA-256 of its target path's UTF-8 bytes. |

Entries are sorted by `path`, comparing UTF-8 bytes (Go's `bytes.Compare`,
not locale order). A duplicate path is an error. The executable bit comes
from `Source`, not the disk, so a Windows checkout produces the same entry
as a Linux one.

### 3.7 `consumed`

```
str("consumed") || list(entries)
entry = str(target) || str(phase) || bytes(key)       -- key: 32 raw bytes
```

Sorted by `target`, then by `phase`, comparing bytes. A duplicate
`(target, phase)` is an error.

### 3.8 `tools`

```
str("tools") || list(entries)
entry = str(name) || str(version) || u8(kind) || bytes(identity)
```

| `kind` | Tool | `identity` |
|---|---|---|
| `0` | Optional and absent | empty |
| `1` | Managed download | the 32-byte SHA-256 of the verified archive |
| `2` | Supplied by the host | the full output of the tool's check command, byte-exact |
| `3` | Built in this repository | the 32-byte key of the action that built it |

The identity's length is checked against its kind: empty for kind `0`,
exactly 32 bytes for kinds `1` and `3`, any length for kind `2`.

Sorted by `name`, comparing bytes. A duplicate name is an error.

### 3.9 `env`

```
str("env") || list(entries)
entry = str(name) || str(value) || u8(origin)      -- origin: 0 = env, 1 = pass
```

Sorted by `name`, comparing bytes. A duplicate name is an error.

## 4. Errors are errors

An implementation must **refuse** to produce a key, never repair its input,
when it sees:

- an unknown `os`, `arch`, `phase`, entry `kind` or tool `kind`;
- an `exec` or `origin` byte other than `0` or `1`, or `exec = 1` on a link;
- invalid UTF-8 in any string;
- a malformed path (§3.5), including the empty path;
- a digest or consumed key that is not exactly 32 bytes, or a tool identity
  of the wrong length for its kind (§3.8);
- a list that is not **strictly** ascending by bytes. An equal neighbour is a
  duplicate, a descending pair is unsorted, and both are errors.

These rules apply to every section, `consumed` included. Silently fixing an
input would let two implementations disagree about which action they keyed.

Two things are deliberately **allowed**:

- Empty strings other than paths: an empty target, tool name, tool version,
  env name or env value is valid, because nothing here forbids it and
  framing keeps it unambiguous.
- A backslash in a path is an ordinary byte, not a separator. `Source`
  always reports `/`-separated paths, so `a\b` is a single segment.

## 5. Versions

- **`schema_version`** is `1` for this document. Any change to sections 2–4
  — a new section, a reordered field, a new tag — is a new schema version
  and a new spec revision.
- **`semantics_version`** belongs to KILN01 (expansion). It changes when the
  same BUILD and rule files would expand to different canonical recipes.
  Until KILN01 is accepted it is `0`, which only legacy actions use.

Bumping either version changes every key, which empties the ledger. That is
deliberate: it is the only safe response to a change in meaning.

## 6. Golden vectors

`code/specs/fixtures/kiln/key-vectors-v1.json` lists actions with their
expected preimage (hex) and key. Every implementation must reproduce every
vector exactly, and must reject every entry in its `invalid` list. The
vectors cover at least:

1. The minimal action: every list empty, no host fingerprint.
2. The comma-collision pair from §1, plus the same shape on tool name and
   version, each of which must produce **different** keys. (A pair split
   across target and phase is impossible to build, because `phase` only
   admits three values.)
3. Non-ASCII paths and a path whose byte order differs from its
   case-insensitive order (`B` sorts before `a`).
4. A link entry and an executable file.
5. Each tool `kind`.
6. A host fingerprint present versus absent (different keys).
7. A legacy recipe with and without `BUILD_windows`.
8. Invalid inputs: unsorted inputs, a duplicate tool, a `..` path, a
   31-byte digest, an unknown phase, invalid UTF-8.

The file's format: a top-level `schema_version` (must equal this spec's), a
`valid` array of `{name, description, action, preimage_hex, key_hex}`, and an
`invalid` array of `{name, action, error}` where `error` is a substring of the
expected refusal. Byte strings in an action are hex (`recipe_hex`,
`digest_hex`, `key_hex`, `identity_hex`). Because JSON strings cannot carry
invalid UTF-8, an action may give `target_hex` to override `target`. Unknown
fields are rejected.

The Go vectors were cross-checked against an independent encoder written
from this spec alone; all 13 valid vectors matched byte for byte.

CI checks that the Go implementation reproduces the file (KILN00 §10.2:
golden vectors are what stop a change to key computation from shipping
without a version bump).
