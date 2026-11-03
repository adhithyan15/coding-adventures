# bounded-source-file

Read a source file without ever holding more than a byte budget.

Compiler front ends take files from users. `std::fs::read_to_string` reads
the whole file before anything can measure it, so a 10 GB file costs 10 GB
just to be refused, `/dev/zero` never returns, and a FIFO with no writer
blocks forever. This crate is the one bounded reader the Twig toolchain
(`twig-module-driver`, so the `twig-vm` CLI), `twig-aot` and `lang-aot`
share.

## How it reads

| Step | Catches |
|------|---------|
| `fs::metadata(path).is_file()` | a FIFO, tty, device or directory, before `open` |
| open with `O_NONBLOCK \| O_NOCTTY` (Unix) | a path swapped for a FIFO or tty after that check: `open` still returns, and a tty never becomes the controlling terminal |
| the open handle's `is_file()` and `len()` | the swap, and a file too big now, before a byte is read |
| reading through `take(limit + 1)` and counting | a file that grew after the size check |

At most `limit + 1` bytes are ever buffered. Errors are `io::Error`s, so
callers keep their `?`:

| Kind | Meaning |
|------|---------|
| `InvalidInput` | not a regular file |
| `FileTooLarge` | more than `limit` bytes |
| `InvalidData` | not UTF-8 |

## Usage

```rust
use std::path::Path;

let source = bounded_source_file::read_to_string(Path::new("main.tw"), 1 << 20)?;
# Ok::<(), std::io::Error>(())
```

`read_bounded(reader, limit)` applies the last step to any `Read`.

## Where it fits

It sits under every Twig file reader: `twig_module_driver::read_source`
(the twig-vm CLI and every file of an import graph) passes the compiler's
`MAX_SOURCE_BYTES`, `twig-aot` the same, and `lang-aot` its 16 MiB
`MAX_SOURCE_FILE_BYTES`. Introduced by the #18474 security review.

## Tests

```bash
cargo test -p bounded-source-file
```

The limit, a sparse 1 GiB file refused by size, a reader longer than the
limit cut off at exactly `limit + 1` bytes, invalid UTF-8, a directory, a
missing file, `/dev/zero`, and (Unix) a FIFO refused without blocking,
both through `read_to_string` and through the non-blocking open alone.
