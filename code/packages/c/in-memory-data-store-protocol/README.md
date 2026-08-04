# in-memory-data-store-protocol (C)

**CCPP02 port campaign — bucket A (pure-ISO), port #1.** The little intermediate
representation a Redis-style ("RESP") in-memory data store uses between the wire
and its engine. The C port of the Rust `in-memory-data-store-protocol` crate, and
the first *bucket-A* port after the thread slice: a crate that needs **no OS at
all**, so it rides the `iso-harness` (which links nothing and compiles under every
available compiler with `-pedantic-errors` / `/permissive-`) rather than
os-platform.

Two types:

- a **command frame** — an (uppercased) command name plus a vector of raw byte
  args, the shape a decoded request takes on its way into the engine; and
- an **engine response** — the RESP reply *tree* the engine hands back: a simple
  string, an error, an integer, a bulk string (possibly null), or a
  (possibly-null) array of nested responses.

```c
/* Decode wire parts into a command frame (first part → uppercased command). */
imds_arg parts[3] = { {(unsigned char*)"set",3},
                      {(unsigned char*)"k",1}, {(unsigned char*)"v",1} };
imds_command_frame f;
if (imds_command_frame_from_parts(parts, 3, &f) == IMDS_OK) {
    /* f.command == "SET", f.nargs == 2 */
    imds_command_frame_free(&f);
}

/* Build a reply: *3\r\n :1 +OK $-1  as an array tree. */
imds_engine_response items[3], root;
imds_resp_one(&items[0]);
imds_resp_ok(&items[1]);
imds_resp_bulk_null(&items[2]);
imds_resp_array(items, 3, &root);   /* takes ownership of items[] */
/* … serialize root … */
imds_engine_response_free(&root);   /* frees the whole tree */
```

| Function | Purpose |
|----------|---------|
| `imds_command_frame_new` | build a frame from a command name + args (all copied) |
| `imds_command_frame_from_parts` | first wire part → uppercased command, rest → args; `IMDS_NONE` on empty |
| `imds_command_frame_free` | release a frame |
| `imds_resp_simple_string` / `imds_resp_error` | `+…` / `-…` string replies |
| `imds_resp_integer` / `imds_resp_zero` / `imds_resp_one` | `:…` integer replies |
| `imds_resp_bulk_string` / `imds_resp_bulk_null` | `$…` blob (copied) / `$-1` |
| `imds_resp_array` / `imds_resp_array_null` | `*…` (takes ownership of items) / `*-1` |
| `imds_resp_ok` / `imds_resp_null` | `+OK` / `$-1` shortcuts |
| `imds_engine_response_free` | release a response, recursively freeing a nested array tree |

## Faithfulness notes

- **`Option<T>` → status / flag.** `from_parts` returning `Option::None` on an
  empty part list becomes the `IMDS_NONE` status. `BulkString(Option<Vec<u8>>)`
  and `Array(Option<Vec<…>>)` become an `is_null` flag beside the payload, so
  `$-1`/`*-1` stay distinct from an empty-but-present blob/array.
- **`ascii_upper` is byte-exact.** The Rust is
  `byte.to_ascii_uppercase() as char` collected into a `String`: only `a`..=`z`
  shift, and a byte ≥ `0x80` becomes `U+0080..U+00FF` and is **UTF-8-encoded to
  two bytes**. The C reproduces exactly that (worst case `2·len`), so `f.command`
  equals the Rust `command` field's bytes for *any* input, not just ASCII.
- **Ownership.** Rust's `String`/`Vec` become `malloc`'d buffers we own. Every
  value you construct or receive owns its heap and must be released
  (`imds_command_frame_free` / `imds_engine_response_free`); both are safe on a
  zeroed value and on `NULL`. `imds_resp_array` *takes ownership* of the `items`
  buffer you hand it (no copy) — build children, then transfer them.
- **Fallible allocation.** The Rust never fails to allocate (it aborts); the C is
  stricter — allocating constructors return `IMDS_ERR_NOMEM` and unwind cleanly,
  so a partially-built value never leaks.

## Build & test

Pure ISO, no OS, no link libraries.

```sh
cd code/packages/c/in-memory-data-store-protocol
sh tools/run.sh        # macOS / Linux (Windows: tools\run.ps1 via BUILD_windows)
```

Locally (macOS): 89 checks / 0 failed under gcc + clang with `-pedantic-errors`;
clean under ASan+UBSan; 0 leaks.

## Layout

```
in-memory-data-store-protocol/
├── include/imds_protocol/imds_protocol.h   # public API
├── src/imds_protocol.c                      # the IR — one pure-ISO source file
├── tests/imds_protocol_test.c               # tests (frames, responses, tree free)
├── tools/run.sh  · run.ps1                   # build via iso-harness (links nothing)
├── BUILD  · BUILD_windows                    # per-OS build drivers
└── .gitignore
```
