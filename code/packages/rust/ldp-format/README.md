# ldp-format

**LANG22 PR 11d** — versioned binary serialiser for `.ldp`
(language-runtime profile) artefacts.  Pure data crate.  Zero
external dependencies; std-only.

## Why

Three downstream consumers need to share one on-disk format:

- **`aot-with-pgo`** (LANG22 PR 11e) — reads `.ldp` to promote
  `type_hint` fields and emit speculation guards
- **`jit-core`** (LANG22 PR 11f) — writes `.ldp` on shutdown so a
  later AOT-PGO build can consume the JIT's observations
- **`lang-perf-suggestions`** (LANG22 PR 11g) — reads `.ldp` and
  surfaces the developer-facing "annotate `n: int` to skip
  122ms warmup" reports

Bundling the format into one consumer would couple the others.
Extracting it as a pure data crate (the same shape as
`interpreter-ir` for code or `constraint-instructions` for solver
programs) means bug fixes propagate everywhere on a single bump.

## Format (version 1)

Little-endian throughout.  See `src/lib.rs` module docs for the
complete byte layout.

```
Header (32 bytes):
  magic = "LDP\0"
  version (major, minor) = (1, 0)
  language [u8; 16] = "twig\0..." (NUL-padded ASCII)
  flags                              (bit 0 = closed-world, bit 1 = JIT-source)
  record_count
  reserved

String table:
  count
  for each: u16 length + bytes + NUL terminator

Module records (one per IIR module):
  module_name_idx
  function_count
  for each function:
    function_name_idx
    param_count + per-param type_idx
    call_count, total_self_time_ns
    type_status, promotion_state
    instr_count
    for each instr:
      instr_index, opcode_idx
      observation_count, observed_kind
      observation_count_at_promotion
      time_to_first_observation_ns, time_to_promotion_ns
      types_seen[] = (type_idx, type_count)*
      ic_entry_count = 0 (reserved for v2)
```

## Public API

```rust
use ldp_format::{LdpFile, Header, ModuleRecord, FunctionRecord,
                 InstructionRecord, TypeStatus, PromotionState,
                 ObservedKind, read, write};

let file = LdpFile {
    header: Header { version: (1, 0), language: "twig".into(), flags: 0 },
    modules: vec![ /* ModuleRecord{...} */ ],
};

// Write to any std::io::Write sink:
let mut bytes = Vec::new();
write(&file, &mut bytes).unwrap();

// Read from any std::io::Read source:
let restored = read(&bytes[..]).unwrap();
assert_eq!(restored, file);
```

## Determinism

`write` is **deterministic**: byte-identical input produces
byte-identical output.  The string table is built in
first-occurrence order during the write so identical files always
produce identical byte layouts.  Verified by
`writer_is_deterministic` test.

## Forward compatibility

- `read` rejects unknown `magic` (returns `BadMagic`).
- `read` rejects `version_major != 1` (returns
  `UnsupportedMajorVersion`).
- `_pad` and `reserved` fields exist so a v1.1 / v1.2 writer can
  add small optional fields without breaking v1.0 readers.
- All public enums (`TypeStatus`, `PromotionState`,
  `ObservedKind`) are `#[non_exhaustive]` — adding variants
  doesn't break downstream `match` consumers.

## Tests

```bash
cargo test -p ldp-format
```

13 unit tests covering:
- Empty file round-trip
- Rich file round-trip (multiple modules, functions, instructions, types)
- Deterministic writes (byte-identical output for same input)
- String-table dedup (100 modules with shared strings stay <150 bytes/module)
- Reject bad magic
- Reject unsupported major version
- Reject truncated input mid-record
- Reject `language` longer than 16 bytes on write
- Reject non-ASCII `language` on write
- Unicode in module / function names round-trips correctly
- Coverage of all 4 `ObservedKind` variants
- Coverage of all 9 `TypeStatus × PromotionState` combinations
- Reject corrupted `observed_kind` byte without panicking

## Where this crate sits

```
LANG22 PR 11a: aot-no-profile           ← independent of profiles
LANG22 PR 11d: ldp-format               ← THIS CRATE — pure data
LANG22 PR 11e: aot-with-pgo             ← reads .ldp via this crate
LANG22 PR 11f: jit-core writes profile  ← writes .ldp via this crate
LANG22 PR 11g: lang-perf-suggestions    ← reads .ldp via this crate
```

See [`code/specs/LANG22-typing-spectrum-aot-jit.md`](../../specs/LANG22-typing-spectrum-aot-jit.md)
for the full AOT/JIT/PGO compilation story.
