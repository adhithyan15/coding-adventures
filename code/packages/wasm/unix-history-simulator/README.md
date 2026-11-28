# HOS Unix history WebAssembly guest

This package is a small, reusable WebAssembly guest implementing the HOS00
file-read teaching image. It models an in-memory five-inode filesystem, path
resolution, a four-slot descriptor table, per-open byte offsets, reads, and
directory entries. It runs without host imports and does not read host files,
inspect process arguments, or format console output.

The guest is semantic teaching code, not a historical Unix executable or
syscall ABI. The external Node adapter translates commands and guest results
to the canonical fixture event format. The package itself has no shell or
JSON dependency.

## Guest ABI

The WAT source is `src/unix_history.wat`. Build it into a Wasm 1.0 module with
the fixture-side Rust helper, which uses the repository's `wasm-wast-parser`
and `wasm-module-encoder` crates through local Cargo path dependencies. The
helper is build/test tooling only; it is not part of the guest and adds no
host imports to the generated module. The module exports `memory`, `reset`,
`resolve`, `open`, `read`, `close`, `kind`, `descriptor_inode`,
`descriptor_offset`, `entry_count`, `entry_name_ptr`, and `entry_name_len`.

`resolve(pathPtr, pathLength)` accepts UTF-8 bytes in linear memory and returns
an inode or a negative error code. `open(inode)` returns the lowest free
descriptor slot or a negative code. `read(fd, destination, capacity)` copies
at most `capacity` bytes into guest memory and advances only that descriptor's
offset. `close(fd)` releases the slot. Directory entry functions expose the
guest-owned names as bounded pointers into the same memory.

The error values are `EINVAL=-1`, `ENOENT=-2`, `ENOTDIR=-3`, `EISDIR=-4`,
`EBADF=-5`, and `EMFILE=-6`. Path length is limited to 4096 bytes, read
capacity to 4096 bytes, and path traversal to 512 components. Guest path input
is restricted to `[4096, 8192)` and the only guest `read` output region is
`[8192, 12288)`. Both the guest and wrapper validate ranges before copying;
failed reads leave descriptor offsets unchanged. The image is immutable under
guest operations and contains root entries `hello`, `empty`, and `etc`; `/etc`
contains `notes`.

The exported linear memory is visible to the embedding host so it can marshal
input and observe output. The host is the trusted embedder and can directly
change any byte in that memory; the bounds guarantee applies to guest API
operations, not to arbitrary writes by a host that owns the instance.

## Build and tests

Each BUILD front invokes the fixture-side helper with the Rust 1.100.0 toolchain,
locked Cargo dependencies, and Cargo offline mode. It then runs component checks
and the nine shared `historical-unix-v1/file-read.json` cases from fixture-
adjacent adapters. The adapters are outside this package so the reusable guest
remains free of host file and process capabilities. Replay evidence covers the
combined WAT guest and host adapter.

The CI toolchains are Node 22.23.5, Python 3.13.17, and Rust 1.100.0. Run
`BUILD` from this package directory; the Windows counterpart is `BUILD_windows`.
