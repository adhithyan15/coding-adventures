# W02 — WASI Preview 1

## Overview

WASI (WebAssembly System Interface) Preview 1 is the **standardised ABI** that lets
a WASM module call host-provided system services — file I/O, clocks, command-line
arguments, random bytes, and network sockets — without being tied to any specific
operating system.

Every `wasm-runtime` in this repo already ships a minimal WASI stub (Tier 2 below).
This spec defines the **complete** `wasi_snapshot_preview1` interface so that stub
can grow into a full implementation tier by tier.

> **Scope of this spec:** The ABI contract only. Every function listed here has the
> same binary signature in all 9 languages because WASM defines it. Language-specific
> implementation patterns are noted where the languages diverge.

---

## Layer Position

```
┌─────────────────────────────────────────────────────────────────┐
│                  W02 — WASI Preview 1                           │
│                                                                 │
│  wasi_snapshot_preview1 (42 host functions)                     │
│    ├── args/environ  (4)   — startup parameters                 │
│    ├── clock         (2)   — time & resolution                  │
│    ├── fd_*          (19)  — file descriptor I/O                │
│    ├── path_*        (10)  — filesystem operations              │
│    ├── poll_oneoff   (1)   — I/O event multiplexing             │
│    ├── proc_*        (2)   — process lifecycle                  │
│    ├── random_get    (1)   — entropy                            │
│    ├── sched_yield   (1)   — scheduler hint                     │
│    └── sock_*        (4)   — network sockets                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  W01 — WASM Runtime                             │
│  wasm-runtime → wasm-execution → wasm-module-parser → ...       │
└─────────────────────────────────────────────────────────────────┘
```

WASI sits **above** the WASM runtime (W01). The runtime calls WASI host functions
through the `HostInterface` import resolution mechanism already in place. WASI does
not add new packages; it expands `WasiHost` inside `wasm-runtime`.

---

## The WASI ABI Contract

### Module Name

All WASI Preview 1 imports share a single module name:

```
wasi_snapshot_preview1
```

When a WASM binary is compiled with WASI support (e.g. `rustc --target wasm32-wasi`),
the binary contains import declarations like:

```wat
(import "wasi_snapshot_preview1" "fd_write"
  (func $fd_write (param i32 i32 i32 i32) (result i32)))
(import "wasi_snapshot_preview1" "proc_exit"
  (func $proc_exit (param i32)))
```

The runtime's `HostInterface.resolve_function("wasi_snapshot_preview1", name)` must
return a host function for each of these.

### Calling Convention

WASI Preview 1 uses **only WASM 1.0 value types** — no reference types, no multi-value
returns (results are written back into linear memory via pointer arguments instead).

- **All parameters**: `i32` or `i64`
- **Return value**: always a single `i32` errno (0 = success, non-zero = error code)
- **Pointer arguments**: indices into the WASM module's linear memory (not host
  pointers). The host reads/writes them via `LinearMemory`.
- **`proc_exit`** is the one exception: it takes an `i32` and never returns (it
  raises an exception / panics in the host).

### Error Code Conventions

Every WASI function (except `proc_exit`) returns an `errno` value as an `i32`:

```
0   → ESUCCESS   — no error
non-zero → error (see Errno enum below)
```

Multi-value "returns" (e.g. a size + an error) are expressed as two separate writes
into caller-supplied pointer locations plus an errno return.

Example — `args_sizes_get`:
```
args_sizes_get(argc_ptr: i32, argv_buf_size_ptr: i32) → errno: i32

On success (errno=0):
  memory[argc_ptr..+4]          = number of arguments (i32, little-endian)
  memory[argv_buf_size_ptr..+4] = total byte length of all arg strings (i32)
```

---

## Type Definitions

All multi-byte integers in linear memory are **little-endian**, matching the WASM
spec.

### Errno

The complete set of WASI error codes. Returned as an `i32`.

```
0   ESUCCESS          No error
1   E2BIG             Argument list too long
2   EACCES            Permission denied
3   EADDRINUSE        Address in use
4   EADDRNOTAVAIL     Address not available
5   EAFNOSUPPORT      Address family not supported
6   EAGAIN            Resource unavailable or operation would block
7   EALREADY          Connection already in progress
8   EBADF             Bad file descriptor
9   EBUSY             Device or resource busy
10  ECANCELED         Operation canceled
11  ECHILD            No child processes
12  ECONNABORTED      Connection aborted
13  ECONNREFUSED      Connection refused
14  ECONNRESET        Connection reset
15  EDEADLK           Resource deadlock would occur
16  EDESTADDRREQ      Destination address required
17  EDOM              Mathematics argument out of domain of function
18  EDQUOT            Reserved (quota exceeded)
19  EEXIST            File exists
20  EFAULT            Bad address
21  EFBIG             File too large
22  EHOSTUNREACH      Host is unreachable
23  EIDRM             Identifier removed
24  EILSEQ            Illegal byte sequence
25  EINPROGRESS       Operation in progress
26  EINTR             Interrupted function
27  EINVAL            Invalid argument
28  EIO               I/O error
29  EISCONN           Socket is connected
30  EISDIR            Is a directory
31  ELOOP             Too many levels of symbolic links
32  EMFILE            File descriptor value too large
33  EMLINK            Too many links
34  EMSGSIZE          Message too large
35  EMULTIHOP         Reserved (multihop attempted)
36  ENAMETOOLONG      Filename too long
37  ENETDOWN          Network is down
38  ENETRESET         Connection aborted by network
39  ENETUNREACH       Network unreachable
40  ENFILE            Too many files open in system
41  ENOBUFS           No buffer space available
42  ENODEV            No such device
43  ENOENT            No such file or directory
44  ENOEXEC           Executable file format error
45  ENOLCK            No locks available
46  ENOLINK           Reserved (link severed)
47  ENOMEM            Not enough space
48  ENOMSG            No message of the desired type
49  ENOPROTOOPT       Protocol not available
50  ENOSPC            No space left on device
51  ENOSYS            Function not supported         ← current stub returns this
52  ENOTCONN          The socket is not connected
53  ENOTDIR           Not a directory or a symbolic link to a directory
54  ENOTEMPTY         Directory not empty
55  ENOTRECOVERABLE   State not recoverable
56  ENOTSOCK          Not a socket
57  ENOTSUP           Not supported, or operation not supported on socket
58  ENOTTY            Inappropriate I/O control operation
59  ENXIO             No such device or address
60  EOVERFLOW         Value too large to be stored in data type
61  EOWNERDEAD        Previous owner died
62  EPERM             Operation not permitted
63  EPIPE             Broken pipe
64  EPROTO            Protocol error
65  EPROTONOSUPPORT   Protocol not supported
66  EPROTOTYPE        Protocol wrong type for socket
67  ERANGE            Result too large
68  EROFS             Read-only file system
69  ESPIPE            Invalid seek
70  ESRCH             No such process
71  ESTALE            Reserved (stale file handle)
72  ETIMEDOUT         Connection timed out
73  ETXTBSY           Text file busy
74  EXDEV             Cross-device link
```

> Note: WASI errno values are **not the same** as POSIX errno values. The mapping
> above is WASI's own numbering (from the witx specification).

### Filetype

The `filetype` field in `fdstat` and `filestat` uses these values:

```
0  UNKNOWN           The type of the file descriptor or file is unknown
1  BLOCK_DEVICE      A block device
2  CHARACTER_DEVICE  A character device (e.g. /dev/null)
3  DIRECTORY         A directory
4  REGULAR_FILE      A regular file
5  SOCKET_DGRAM      A datagram socket
6  SOCKET_STREAM     A byte-stream socket
7  SYMBOLIC_LINK     A symbolic link
```

### Whence (for fd_seek)

```
0  SET    Seek relative to start-of-file
1  CUR    Seek relative to current position
2  END    Seek relative to end-of-file
```

### Memory Layout: Ciovec (read-only scatter/gather buffer)

Used by `fd_write`, `sock_send`, etc. An array of these is passed as `(iovs_ptr, iovs_len)`.

```
offset  size  type    field
  0      4    i32     buf      ← pointer into linear memory
  4      4    i32     buf_len  ← byte length of the buffer
──────
total: 8 bytes per entry

Array of N ciovecs occupies N × 8 bytes starting at iovs_ptr.
```

To write "Hello\n" (6 bytes at address 100):
```
memory[iovs_ptr + 0] = 100   (buf ptr)
memory[iovs_ptr + 4] = 6     (buf len)
iovs_len = 1
```

### Memory Layout: Iovec (read-write scatter/gather buffer)

Used by `fd_read`, `sock_recv`, etc. Same layout as Ciovec but the buffer is writable.

```
offset  size  type    field
  0      4    i32     buf      ← pointer into linear memory (writable)
  4      4    i32     buf_len  ← byte length of the buffer
──────
total: 8 bytes per entry
```

### Memory Layout: Filestat

Written by `fd_filestat_get` and `path_filestat_get`.

```
offset  size  type    field
  0      8    u64     dev      ← device ID
  8      8    u64     ino      ← inode number
 16      1    u8      filetype ← see Filetype enum above
 17      7    (pad)
 24      8    u64     nlink    ← number of hard links
 32      8    u64     size     ← size in bytes
 40      8    u64     atim     ← last access time (ns since Unix epoch)
 48      8    u64     mtim     ← last modification time
 56      8    u64     ctim     ← last status change time
──────
total: 64 bytes
```

### Memory Layout: Fdstat

Written by `fd_fdstat_get`.

```
offset  size  type    field
  0      1    u8      filetype         ← see Filetype enum
  1      1    (pad)
  2      2    u16     flags            ← fd flags (APPEND=1, DSYNC=2, NONBLOCK=4, RSYNC=8, SYNC=16)
  4      4    (pad)
  8      8    u64     rights_base      ← base rights bitmask
 16      8    u64     rights_inheriting← rights for files opened through this fd
──────
total: 24 bytes
```

### Memory Layout: Dirent

Written into the buffer by `fd_readdir`.

```
offset  size  type    field
  0      8    u64     d_next    ← cookie for next entry (opaque)
  8      8    u64     d_ino     ← inode number
 16      4    u32     d_namlen  ← length of d_name in bytes
 20      4    u8      d_type    ← filetype of the entry
──────
24 bytes header, followed immediately by d_namlen bytes of UTF-8 filename (no NUL)
```

### Memory Layout: Event / Subscription (poll_oneoff)

`poll_oneoff` takes an array of subscriptions and writes an array of events.

**Subscription (input):**
```
offset  size  field
  0      8    userdata    ← u64, passed through to the event
  8      1    tag         ← 0=clock, 1=fd_read, 2=fd_write
  9      7    (pad)
 16     var   union body  ← clock or fd body (see below)
──────
total: 48 bytes
```

**Subscription clock body (tag=0, at offset 16):**
```
offset  size  field
 16      8    id          ← clock ID (0=realtime, 1=monotonic)
 24      8    timeout     ← timeout in nanoseconds
 32      8    precision   ← accepted slop in nanoseconds
 40      2    flags       ← 0=relative, 1=absolute
```

**Subscription fd body (tag=1 or 2, at offset 16):**
```
offset  size  field
 16      4    fd          ← file descriptor to wait on
```

**Event (output, written by poll_oneoff):**
```
offset  size  field
  0      8    userdata    ← echoed from subscription
  8      2    error       ← errno for this event
 10      1    type        ← event type (matches subscription tag)
 11      5    (pad)
 16      8    fd_readwrite.nbytes  ← bytes available (for fd events)
 24      2    fd_readwrite.flags   ← 0=normal, 1=hangup
──────
total: 32 bytes
```

---

## Standard File Descriptors

Every WASI process starts with three pre-opened file descriptors:

```
fd=0  stdin   — character device, read-only
fd=1  stdout  — character device, write-only
fd=2  stderr  — character device, write-only
```

Implementations may also pre-open directories at `fd=3` and above. This is the WASI
**capability model**: the module only gets access to the directories explicitly granted
to it at startup. It cannot open `/etc/passwd` unless `/etc` was pre-opened.

Pre-opened directories are exposed to the WASM module via `fd_fdstat_get` (filetype=3,
DIRECTORY) and are the root for all `path_*` calls.

---

## Rights / Capabilities

Every file descriptor carries two 64-bit bitmasks describing what operations are
permitted:

- **rights_base**: Operations on the fd itself
- **rights_inheriting**: Rights granted to fds opened through this fd (for directories)

Key right bits:

```
bit  name
  0  FD_DATASYNC
  1  FD_READ
  2  FD_SEEK
  3  FD_FDSTAT_SET_FLAGS
  4  FD_SYNC
  5  FD_TELL
  6  FD_WRITE
  7  FD_ADVISE
  8  FD_ALLOCATE
  9  PATH_CREATE_DIRECTORY
 10  PATH_CREATE_FILE
 11  PATH_LINK_SOURCE
 12  PATH_LINK_TARGET
 13  PATH_OPEN
 14  FD_READDIR
 15  PATH_READLINK
 16  PATH_RENAME_SOURCE
 17  PATH_RENAME_TARGET
 18  PATH_FILESTAT_GET
 19  PATH_FILESTAT_SET_SIZE
 20  PATH_FILESTAT_SET_TIMES
 21  FD_FILESTAT_GET
 22  FD_FILESTAT_SET_SIZE
 23  FD_FILESTAT_SET_TIMES
 24  PATH_SYMLINK
 25  PATH_REMOVE_DIRECTORY
 26  PATH_UNLINK_FILE
 27  POLL_FD_READWRITE
 28  SOCK_SHUTDOWN
 29  SOCK_ACCEPT
```

For the stub, these can be returned as `0xFFFFFFFFFFFFFFFF` (all rights) since we
don't enforce them. A real implementation restricts them appropriately.

---

## Syscall Reference

### Args / Environ (4 functions)

These give a WASM program access to command-line arguments and environment variables
supplied by the host. They work in two phases: first query the sizes, then fetch the
data.

#### `args_sizes_get`

```
args_sizes_get(
  argc_ptr:          i32,  ← write: number of arguments (u32)
  argv_buf_size_ptr: i32   ← write: total byte size of all arg strings incl. NUL
) → errno: i32
```

Writes the argument count and total buffer size needed to hold all argument strings
(each NUL-terminated). The caller uses these to allocate memory before calling
`args_get`.

**Example:** `["my-program", "--flag"]` → argc=2, buf_size=22 (11 + 7 + 2 NUL bytes)

**Errors:** Always ESUCCESS in a correct implementation.

#### `args_get`

```
args_get(
  argv:     i32,  ← write: array of pointers (u32[argc]), each pointing into argv_buf
  argv_buf: i32   ← write: packed NUL-terminated argument strings
) → errno: i32
```

Writes the argument strings into `argv_buf` (NUL-terminated, packed contiguously) and
fills `argv` with pointers into `argv_buf`.

**Memory layout after call:**
```
argv_buf: "my-program\0--flag\0"
argv[0]  = address of "my-program"
argv[1]  = address of "--flag"
```

#### `environ_sizes_get`

```
environ_sizes_get(
  environ_count_ptr:    i32,  ← write: number of env vars (u32)
  environ_buf_size_ptr: i32   ← write: total byte size of all env strings incl. NUL
) → errno: i32
```

Same pattern as `args_sizes_get` but for environment variables.

#### `environ_get`

```
environ_get(
  environ:     i32,  ← write: array of pointers (u32[count])
  environ_buf: i32   ← write: packed NUL-terminated "KEY=VALUE" strings
) → errno: i32
```

Same pattern as `args_get` but for `KEY=VALUE` environment strings.

---

### Clock (2 functions)

WASI defines four clock IDs:

```
0  REALTIME           Wall clock (may jump on NTP adjustments)
1  MONOTONIC          Monotonically increasing (never jumps backward)
2  PROCESS_CPUTIME_ID CPU time used by this process
3  THREAD_CPUTIME_ID  CPU time used by this thread
```

All time values are **nanoseconds** as a `u64`.

#### `clock_res_get`

```
clock_res_get(
  id:             i32,  ← clock ID (0-3)
  resolution_ptr: i32   ← write: resolution in nanoseconds (u64)
) → errno: i32
```

Writes the resolution (granularity) of the specified clock. For a typical system
wall clock this is 1 000 000 ns (1 ms). For a monotonic clock it may be 1 ns.

**Errors:** `EINVAL` if `id` is not a known clock.

#### `clock_time_get`

```
clock_time_get(
  id:        i32,  ← clock ID (0-3)
  precision: i64,  ← requested precision in ns (hint, may be ignored)
  time_ptr:  i32   ← write: current time in nanoseconds (u64)
) → errno: i32
```

Writes the current value of the specified clock. The `precision` parameter is a
hint — implementations may ignore it and always return the finest available
resolution.

**Implementation note:** On Unix hosts, this maps to `clock_gettime(CLOCK_REALTIME)`
(id=0) and `clock_gettime(CLOCK_MONOTONIC)` (id=1). On Windows, use
`GetSystemTimeAsFileTime` / `QueryPerformanceCounter`.

---

### File Descriptors (19 functions)

All fd operations require the fd to have appropriate rights. A stub returning ENOSYS
for unimplemented calls is safe — programs that need the feature will fail gracefully
rather than crashing.

#### `fd_advise`

```
fd_advise(
  fd:     i32,  ← file descriptor
  offset: i64,  ← starting offset in file
  len:    i64,  ← byte length of region
  advice: i32   ← 0=NORMAL 1=SEQUENTIAL 2=RANDOM 3=WILLNEED 4=DONTNEED 5=NOREUSE
) → errno: i32
```

Hints to the OS how the application intends to use the given file region. This is a
pure hint — returning ESUCCESS without doing anything is a valid implementation.

#### `fd_allocate`

```
fd_allocate(
  fd:     i32,  ← file descriptor (must be a regular file)
  offset: i64,  ← starting offset
  len:    i64,  ← byte count to pre-allocate
) → errno: i32
```

Pre-allocates disk space without modifying file size or data. Equivalent to
`posix_fallocate`. Safe to return ENOSYS on platforms that don't support it.

#### `fd_close`

```
fd_close(fd: i32) → errno: i32
```

Closes a file descriptor, releasing all associated resources. After a successful
close, the fd number may be reused by subsequent opens.

**Errors:** `EBADF` if fd is not open.

#### `fd_datasync`

```
fd_datasync(fd: i32) → errno: i32
```

Flushes all data written to fd to stable storage, but not necessarily metadata
(directory entries, access times). Equivalent to `fdatasync(2)`.

#### `fd_fdstat_get`

```
fd_fdstat_get(
  fd:       i32,  ← file descriptor
  stat_ptr: i32   ← write: Fdstat struct (24 bytes)
) → errno: i32
```

Writes an `Fdstat` structure describing the type and access rights of an open fd.
Used by libc to determine if a fd is a TTY, file, or socket.

For the stub, stdout/stderr should report `filetype=CHARACTER_DEVICE` and
`filetype=DIRECTORY` for pre-opened directories.

#### `fd_fdstat_set_flags`

```
fd_fdstat_set_flags(
  fd:    i32,  ← file descriptor
  flags: i32   ← new fd flags (APPEND=1, DSYNC=2, NONBLOCK=4, RSYNC=8, SYNC=16)
) → errno: i32
```

Modifies the flags of an open fd (e.g. add/remove O_NONBLOCK). Most implementations
can return ESUCCESS without doing anything for the common case.

#### `fd_fdstat_set_rights`

```
fd_fdstat_set_rights(
  fd:                 i32,
  rights_base:        i64,  ← new base rights bitmask
  rights_inheriting:  i64   ← new inheriting rights bitmask
) → errno: i32
```

Restricts the rights of an fd. Rights can only be removed, never added. Return
`ENOTCAPABLE` (not in the errno table above — use ENOSYS) if the new rights exceed
the current ones.

#### `fd_filestat_get`

```
fd_filestat_get(
  fd:       i32,  ← file descriptor
  stat_ptr: i32   ← write: Filestat struct (64 bytes)
) → errno: i32
```

Writes the `Filestat` struct for the file underlying this fd. Equivalent to `fstat(2)`.

#### `fd_filestat_set_size`

```
fd_filestat_set_size(
  fd:   i32,
  size: i64   ← new file size in bytes
) → errno: i32
```

Truncates or extends the file to exactly `size` bytes. Equivalent to `ftruncate(2)`.

#### `fd_filestat_set_times`

```
fd_filestat_set_times(
  fd:        i32,
  atim:      i64,  ← new access time (ns since epoch), if flag set
  mtim:      i64,  ← new modification time, if flag set
  fst_flags: i32   ← which fields to update: ATIM=1, MTIM=2, ATIM_NOW=4, MTIM_NOW=8
) → errno: i32
```

Updates access and/or modification times. `fst_flags` controls which fields are
written and whether to use the provided value or the current time.

#### `fd_pread`

```
fd_pread(
  fd:       i32,  ← file descriptor (must support FD_READ + FD_SEEK)
  iovs:     i32,  ← pointer to array of Iovec structs
  iovs_len: i32,  ← number of Iovec entries
  offset:   i64,  ← byte offset in file to read from
  nread:    i32   ← write: total bytes read (u32)
) → errno: i32
```

Reads from a specific file offset without moving the current position. Equivalent to
`pread(2)`.

#### `fd_pwrite`

```
fd_pwrite(
  fd:       i32,
  iovs:     i32,  ← pointer to array of Ciovec structs
  iovs_len: i32,
  offset:   i64,  ← byte offset in file to write at
  nwritten: i32   ← write: total bytes written (u32)
) → errno: i32
```

Writes to a specific file offset without moving the current position. Equivalent to
`pwrite(2)`.

#### `fd_read`

```
fd_read(
  fd:       i32,  ← file descriptor (fd=0 for stdin)
  iovs:     i32,  ← pointer to array of Iovec structs
  iovs_len: i32,  ← number of Iovec entries
  nread:    i32   ← write: total bytes read (u32)
) → errno: i32
```

Reads from the current position of fd into the provided iov buffers, advancing the
position. Returns 0 bytes read on EOF.

**Iov loop:**
```python
total = 0
for i in range(iovs_len):
    buf_ptr = memory.load_i32(iovs + i * 8)
    buf_len = memory.load_i32(iovs + i * 8 + 4)
    data    = read_from_fd(fd, buf_len)
    memory.store_bytes(buf_ptr, data)
    total  += len(data)
    if len(data) < buf_len:
        break  # short read, done
memory.store_i32(nread, total)
```

#### `fd_readdir`

```
fd_readdir(
  fd:          i32,  ← directory file descriptor
  buf:         i32,  ← pointer to output buffer
  buf_len:     i32,  ← size of output buffer
  cookie:      i64,  ← 0 to start, or d_next from previous call
  bufused_ptr: i32   ← write: bytes written into buf (u32)
) → errno: i32
```

Lists directory entries starting at `cookie` (0 = beginning). Each entry is a
`Dirent` header followed immediately by `d_namlen` bytes of filename (no NUL
terminator). Entries are packed contiguously.

If the buffer is too small to hold even one entry, the caller must retry with a
larger buffer. `bufused_ptr` < `buf_len` signals end-of-directory.

#### `fd_renumber`

```
fd_renumber(
  fd: i32,  ← source fd
  to: i32   ← destination fd number
) → errno: i32
```

Atomically replaces the fd table slot `to` with `fd`, closing the old `to` if open.
Equivalent to `dup2(fd, to)`.

#### `fd_seek`

```
fd_seek(
  fd:            i32,
  offset:        i64,  ← byte offset (signed, relative to whence)
  whence:        i32,  ← 0=SET 1=CUR 2=END
  newoffset_ptr: i32   ← write: resulting file offset (u64)
) → errno: i32
```

Changes the current read/write position of an fd. Returns the new absolute position.

**Errors:** `ESPIPE` if fd does not support seeking (e.g. pipes, sockets).

#### `fd_sync`

```
fd_sync(fd: i32) → errno: i32
```

Flushes all data and metadata of fd to stable storage. Equivalent to `fsync(2)`.
Stronger than `fd_datasync` — also persists directory entries and inode metadata.

#### `fd_tell`

```
fd_tell(
  fd:         i32,
  offset_ptr: i32  ← write: current file position (u64)
) → errno: i32
```

Returns the current file position without changing it. Equivalent to
`lseek(fd, 0, SEEK_CUR)`.

#### `fd_write` ← **Already Implemented (Tier 2)**

```
fd_write(
  fd:       i32,  ← 1=stdout 2=stderr (others: EBADF or ignore)
  iovs:     i32,  ← pointer to array of Ciovec structs
  iovs_len: i32,  ← number of Ciovec entries
  nwritten: i32   ← write: total bytes written (u32)
) → errno: i32
```

Writes scatter/gather buffers to fd. The current implementation captures stdout/stderr
via callbacks. A full implementation writes to real file descriptors.

**Iov loop (already in all 9 languages):**
```python
total = 0
for i in range(iovs_len):
    buf_ptr = memory.load_u32(iovs + i * 8)
    buf_len = memory.load_u32(iovs + i * 8 + 4)
    data    = memory.load_bytes(buf_ptr, buf_len)
    write_to_fd(fd, data)
    total  += buf_len
memory.store_i32(nwritten, total)
return ESUCCESS
```

---

### Path Operations (10 functions)

All path functions take a `dirfd` as their first argument — a pre-opened directory fd
that serves as the root for the relative path. This is the WASI capability model: the
module cannot reach files outside directories granted to it.

Path strings are **not NUL-terminated** — they are passed as `(path_ptr, path_len)`.

#### `path_create_directory`

```
path_create_directory(
  fd:       i32,  ← pre-opened directory fd
  path:     i32,  ← pointer to path string
  path_len: i32   ← byte length of path
) → errno: i32
```

Creates a new directory at `path` relative to `fd`. Equivalent to `mkdirat(2)`.

**Errors:** `EEXIST` if path already exists. `ENOTDIR` if a component of path is not
a directory.

#### `path_filestat_get`

```
path_filestat_get(
  fd:       i32,  ← pre-opened directory fd
  flags:    i32,  ← 0=no-follow, 1=follow-symlinks
  path:     i32,
  path_len: i32,
  stat_ptr: i32   ← write: Filestat (64 bytes)
) → errno: i32
```

Gets file metadata for the given path. With `flags=0`, does not follow the final
symlink (`lstat`). With `flags=1`, follows it (`stat`).

#### `path_filestat_set_times`

```
path_filestat_set_times(
  fd:        i32,
  flags:     i32,  ← follow-symlink flag
  path:      i32,
  path_len:  i32,
  atim:      i64,
  mtim:      i64,
  fst_flags: i32   ← ATIM=1, MTIM=2, ATIM_NOW=4, MTIM_NOW=8
) → errno: i32
```

Updates timestamps of the file at path. See `fd_filestat_set_times` for `fst_flags`.

#### `path_link`

```
path_link(
  old_fd:       i32,  ← directory fd for source
  old_flags:    i32,  ← follow-symlink flag for source
  old_path:     i32,
  old_path_len: i32,
  new_fd:       i32,  ← directory fd for destination
  new_path:     i32,
  new_path_len: i32
) → errno: i32
```

Creates a hard link from `old_path` (relative to `old_fd`) to `new_path` (relative to
`new_fd`). Equivalent to `linkat(2)`.

**Errors:** `EXDEV` if old and new are on different filesystems.

#### `path_open`

```
path_open(
  fd:                  i32,  ← pre-opened directory fd
  dirflags:            i32,  ← 1=SYMLINK_FOLLOW
  path:                i32,
  path_len:            i32,
  oflags:              i32,  ← CREAT=1, DIRECTORY=2, EXCL=4, TRUNC=8
  fs_rights_base:      i64,  ← rights for the opened fd
  fs_rights_inheriting:i64,  ← rights for fds opened through this (if directory)
  fdflags:             i32,  ← APPEND=1, DSYNC=2, NONBLOCK=4, RSYNC=8, SYNC=16
  opened_fd_ptr:       i32   ← write: new fd number (u32)
) → errno: i32
```

Opens or creates a file/directory at `path`. This is the WASI equivalent of `openat(2)`.

The returned fd inherits at most `fs_rights_base` rights. The caller cannot grant
rights it doesn't have.

**Errors:**
- `ENOENT`: path does not exist and `CREAT` not set
- `EEXIST`: path exists and both `CREAT` and `EXCL` are set
- `EISDIR`: tried to open a directory for writing
- `ENOTDIR`: `DIRECTORY` oflag set but path is not a directory

#### `path_readlink`

```
path_readlink(
  fd:         i32,
  path:       i32,
  path_len:   i32,
  buf:        i32,  ← write: symlink target (not NUL-terminated)
  buf_len:    i32,
  bufused:    i32   ← write: bytes written (u32)
) → errno: i32
```

Reads the target of a symbolic link. The result is NOT NUL-terminated. If the
buffer is too small, the result is truncated and `bufused = buf_len`.

#### `path_remove_directory`

```
path_remove_directory(
  fd:       i32,
  path:     i32,
  path_len: i32
) → errno: i32
```

Removes an **empty** directory. Equivalent to `unlinkat(fd, path, AT_REMOVEDIR)`.

**Errors:** `ENOTEMPTY` if the directory is not empty.

#### `path_rename`

```
path_rename(
  fd:           i32,  ← directory fd for source
  old_path:     i32,
  old_path_len: i32,
  new_fd:       i32,  ← directory fd for destination
  new_path:     i32,
  new_path_len: i32
) → errno: i32
```

Renames a file or directory, equivalent to `renameat(2)`. Atomic on POSIX systems.

#### `path_symlink`

```
path_symlink(
  old_path:     i32,  ← symlink target (contents of the link)
  old_path_len: i32,
  fd:           i32,  ← directory fd for the new link
  new_path:     i32,
  new_path_len: i32
) → errno: i32
```

Creates a symbolic link `new_path` (relative to `fd`) whose content is `old_path`.
Note: `old_path` is the target string, not a path relative to any fd. It is stored
verbatim as the symlink content.

#### `path_unlink_file`

```
path_unlink_file(
  fd:       i32,
  path:     i32,
  path_len: i32
) → errno: i32
```

Removes a regular file. Equivalent to `unlinkat(fd, path, 0)`.

**Errors:** `EISDIR` if path is a directory (use `path_remove_directory` instead).

---

### Poll (1 function)

#### `poll_oneoff`

```
poll_oneoff(
  in:              i32,  ← pointer to array of Subscription structs
  out:             i32,  ← pointer to output array of Event structs
  nsubscriptions:  i32,  ← number of Subscription entries
  nevents_ptr:     i32   ← write: number of events written (u32)
) → errno: i32
```

Waits for one or more of the subscribed events to occur. Blocks until at least one
event fires. Equivalent to `poll(2)` or `select(2)`.

**Subscription types:**
- `tag=0` (CLOCK): Wake after a timeout. Useful for `sleep`.
- `tag=1` (FD_READ): Wake when fd has data to read.
- `tag=2` (FD_WRITE): Wake when fd can accept writes.

Writes one `Event` struct per triggered subscription into `out`. The `nevents_ptr`
field receives the count of events written (may be less than `nsubscriptions`).

**Implementation complexity:** This is the hardest WASI function to implement
correctly, requiring OS-level event multiplexing. It is safe to return ENOSYS until
Tier 4 is targeted.

---

### Process (2 functions)

#### `proc_exit` ← **Already Implemented (Tier 2)**

```
proc_exit(rval: i32) → (never returns)
```

Terminates the WASM process with exit code `rval`. This function never returns —
the host implementation throws/raises `ProcExitError(rval)`, which the runtime
catches and converts into a clean result.

**Implementation in all 9 languages:**
```python
def proc_exit(args):
    code = args[0]          # i32 exit code
    raise ProcExitError(code)
```

Exit code 0 means success. Any non-zero value means failure. The exit code is
conventionally `args[0]` (in WASI, not WASM argument 0).

#### `proc_raise`

```
proc_raise(sig: i32) → errno: i32
```

Sends a signal to the calling process. Signal numbers follow the WASI `signal` enum
(not POSIX numbers):

```
1=HUP 2=INT 3=QUIT 4=ILL 5=TRAP 6=ABRT 7=BUS 8=FPE 9=KILL 10=USR1
11=SEGV 12=USR2 13=PIPE 14=ALRM 15=TERM 16=CHLD 17=CONT 18=STOP
19=TSTP 20=TTIN 21=TTOU 22=URG 23=XCPU 24=XFSZ 25=VTALRM 26=PROF
27=WINCH 28=POLL 29=PWR 30=SYS
```

In practice, most WASM programs only ever call this with `SIGABRT` (6) for assertion
failures. Safe to return ENOSYS for all signals in Tier 1-3.

---

### Random (1 function)

#### `random_get`

```
random_get(
  buf:     i32,  ← pointer to output buffer
  buf_len: i32   ← number of random bytes to write
) → errno: i32
```

Fills `buf` with `buf_len` cryptographically random bytes. This is the recommended
source of entropy for seeding PRNGs inside WASM.

**Implementation notes by language:**

| Language   | Source                                   |
|------------|------------------------------------------|
| Python     | `os.urandom(n)`                          |
| Go         | `crypto/rand.Read`                       |
| TypeScript | `crypto.getRandomValues` / `randomFill`  |
| Ruby       | `SecureRandom.random_bytes(n)`           |
| Rust       | `getrandom` crate or `OsRng`             |
| Elixir     | `:crypto.strong_rand_bytes(n)`           |
| Lua        | `math.random` (not crypto-safe; note it) |
| Perl       | `Crypt::Random` or `/dev/urandom`        |
| Swift      | `SystemRandomNumberGenerator`            |

The bytes are written into WASM linear memory starting at `buf`.

---

### Scheduler (1 function)

#### `sched_yield`

```
sched_yield() → errno: i32
```

Yields the processor to other threads/processes. A hint to the scheduler — the
current process is willing to give up its time slice.

In a single-threaded WASM runtime, this is a no-op. Return ESUCCESS.

---

### Sockets (4 functions)

WASI Preview 1 includes a minimal socket API. Full socket creation (`socket()`,
`bind()`, `connect()`) is **not** in Preview 1 — sockets must be pre-opened by the
host and passed in as fds, just like pre-opened directories.

#### `sock_accept`

```
sock_accept(
  fd:            i32,  ← listening socket fd (pre-opened)
  flags:         i32,  ← 0=blocking, NONBLOCK=4
  result_fd_ptr: i32   ← write: new connected socket fd (u32)
) → errno: i32
```

Accepts an incoming connection on a listening socket. Writes the new fd number.
Equivalent to `accept4(2)`.

#### `sock_recv`

```
sock_recv(
  fd:            i32,
  ri_data:       i32,  ← pointer to array of Iovec structs
  ri_data_len:   i32,
  ri_flags:      i32,  ← 0=normal, RECV_PEEK=1, RECV_WAITALL=2
  ro_datalen:    i32,  ← write: bytes received (u32)
  ro_flags:      i32   ← write: output flags (RECV_DATA_TRUNCATED=1)
) → errno: i32
```

Receives data from a socket into the iov buffers. Equivalent to `recvmsg(2)`.

#### `sock_send`

```
sock_send(
  fd:          i32,
  si_data:     i32,  ← pointer to array of Ciovec structs
  si_data_len: i32,
  si_flags:    i32,  ← reserved, must be 0
  so_datalen:  i32   ← write: bytes sent (u32)
) → errno: i32
```

Sends data through a connected socket. Equivalent to `sendmsg(2)`.

#### `sock_shutdown`

```
sock_shutdown(
  fd:  i32,
  how: i32  ← RECV=0, SEND=1, BOTH=2
) → errno: i32
```

Shuts down socket send/receive channels without closing the fd. Equivalent to
`shutdown(2)`.

---

## Implementation Tiers

The 42 WASI functions divide naturally into 5 implementation tiers, ordered by
complexity and dependency on host resources:

```
Tier 1 — ENOSYS Fallback (ALL 42 functions)
─────────────────────────────────────────────────────────────────────
Status: ✅ Done in all 9 languages

Every WASI function returns ENOSYS (52) by default. This lets any WASM
binary load and run as long as it doesn't actually call the unimplemented
functions. Pure-computation modules (like our square() test) never call
WASI at all, so they work fine.

Functions: all 42
Enables: Loading any WASM binary without import errors

Tier 2 — stdio (2 functions)
─────────────────────────────────────────────────────────────────────
Status: ✅ Done in all 9 languages

fd_write    Write to stdout/stderr via callbacks
proc_exit   Terminate with exit code

Enables: Hello World programs, print-debugging, any program that
         writes output to stdout/stderr.

Tier 3 — Stateless I/O (8 functions)
─────────────────────────────────────────────────────────────────────
Status: 🔲 Not yet implemented

These functions have no host-persistent state — they read from the
environment or generate values without needing fd tables, VFS, etc.
They are safe to implement without a full filesystem abstraction.

args_get             Read command-line arguments from host-supplied list
args_sizes_get       Query argument count and buffer size
environ_get          Read environment variables
environ_sizes_get    Query environment count and buffer size
clock_res_get        Get clock resolution
clock_time_get       Get current time (wall or monotonic)
random_get           Fill buffer with crypto-random bytes
sched_yield          No-op in single-threaded runtime → ESUCCESS

Enables: Programs that parse command-line flags, use timestamps,
         seed random number generators, or use sleep-via-poll.

Tier 4 — Filesystem (31 functions)
─────────────────────────────────────────────────────────────────────
Status: 🔲 Not yet implemented

All fd_* and path_* operations. Requires:
  - An fd table: map of fd number → open file/directory state
  - A VFS (virtual filesystem): abstract over host OS filesystem
    with WASI capability checks (pre-opened directories only)
  - Path sandboxing: prevent escaping via "../../../etc/passwd"

Functions:
  fd_advise, fd_allocate, fd_close, fd_datasync
  fd_fdstat_get, fd_fdstat_set_flags, fd_fdstat_set_rights
  fd_filestat_get, fd_filestat_set_size, fd_filestat_set_times
  fd_pread, fd_pwrite, fd_read, fd_readdir, fd_renumber
  fd_seek, fd_sync, fd_tell
  path_create_directory, path_filestat_get, path_filestat_set_times
  path_link, path_open, path_readlink
  path_remove_directory, path_rename, path_symlink, path_unlink_file
  poll_oneoff (for file-based I/O events)
  proc_raise

Enables: File reading/writing, config files, logging to disk,
         directory listing, any program that uses the filesystem.

Tier 5 — Network (4 functions)
─────────────────────────────────────────────────────────────────────
Status: 🔲 Not yet implemented

Requires pre-opened socket fds from the host. The host creates
and binds sockets, then passes them to the WASM module as fds.

  sock_accept
  sock_recv
  sock_send
  sock_shutdown

Enables: Network servers and clients (HTTP, TCP, UDP) inside WASM.
```

---

## Test Strategy

### Tier 2 (Already Tested)

The existing `wasm-runtime` test in all 9 languages already validates `fd_write` and
`proc_exit` via the Hello World pattern:

```wat
;; hello.wat — hand-assembled "Hello, World!\n" printer
(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory 1)
  (data (i32.const 0) "Hello, World!\n")  ;; 14 bytes at offset 0
  ;; iov: buf_ptr=0, buf_len=14, at offset 100
  (func (export "_start")
    (i32.store (i32.const 100) (i32.const 0))    ;; buf ptr
    (i32.store (i32.const 104) (i32.const 14))   ;; buf len
    (call $fd_write
      (i32.const 1)    ;; fd=stdout
      (i32.const 100)  ;; iovs ptr
      (i32.const 1)    ;; iovs count
      (i32.const 200)) ;; nwritten ptr
    drop))
```

Expected: `stdout callback` receives `"Hello, World!\n"`.

### Tier 3 Tests

**args/environ** — supply `["my-wasm-program", "--count", "42"]`:
```
args_sizes_get → argc=3, buf_size=30
args_get       → argv[0]→"my-wasm-program", argv[1]→"--count", argv[2]→"42"
```

**clock_time_get** — call twice with clock_id=1 (MONOTONIC):
```
t1 = clock_time_get(1, 0)
t2 = clock_time_get(1, 0)
assert t2 >= t1                  # monotonic guarantee
assert t2 - t1 < 1_000_000_000  # less than 1 second elapsed
```

**random_get** — request 16 bytes, check non-zero:
```
random_get(buf_ptr, 16)
bytes = memory[buf_ptr..buf_ptr+16]
# statistically: at least one byte should be non-zero
assert any(b != 0 for b in bytes)
```

### Tier 4 Tests (Future)

For each path_* / fd_* function, provide a virtual filesystem with known content and
verify reads/writes/stats match expectations. The sandbox test should verify that
`../` path traversal is rejected with `ENOTCAPABLE`.

### Tier 5 Tests (Future)

Pre-open a `(host_server_socket, wasm_fd)` pair. Verify `sock_recv` delivers data
sent by the host into WASM memory, and `sock_send` delivers WASM data to the host.

---

## Language-Specific Implementation Notes

### Storing WASI State

Each language's WASI host implementation will need to carry additional state as tiers are
implemented:

| Field              | Purpose                                    |
|--------------------|--------------------------------------------|
| `args: [str]`      | Command-line arguments (Tier 3)            |
| `environ: {str:str}` | Environment variables (Tier 3)           |
| `fd_table: {int: FdEntry}` | Open file descriptors (Tier 4)   |
| `preopens: {int: str}` | Pre-opened directory paths (Tier 4)  |

### Memory Access

All pointer arguments refer to WASM linear memory. Each language accesses it
differently, but the reading pattern is identical:

```python
# Python
buf_ptr = memory.load_i32_unsigned(iovs_ptr + i * 8)
buf_len = memory.load_i32_unsigned(iovs_ptr + i * 8 + 4)
data    = memory.load_bytes(buf_ptr, buf_len)
```

```go
// Go
bufPtr := memory.LoadU32(iovsPtr + uint32(i)*8)
bufLen := memory.LoadU32(iovsPtr + uint32(i)*8 + 4)
data   := memory.LoadBytes(bufPtr, bufLen)
```

```typescript
// TypeScript
const bufPtr = memory.loadI32(iovsPtr + i * 8) >>> 0
const bufLen = memory.loadI32(iovsPtr + i * 8 + 4) >>> 0
```

### Time Representation

WASI times are `u64` nanoseconds since the Unix epoch (1970-01-01T00:00:00Z).

| Language   | Current wall time in nanoseconds              |
|------------|-----------------------------------------------|
| Python     | `int(time.time_ns())`                         |
| Go         | `time.Now().UnixNano()`                       |
| TypeScript | `BigInt(Date.now()) * 1_000_000n`             |
| Ruby       | `Process.clock_gettime(Process::CLOCK_REALTIME, :nanosecond)` |
| Rust       | `SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()` |
| Elixir     | `:os.system_time(:nanosecond)`                |
| Lua        | `math.floor(os.time() * 1e9)`                 |
| Perl       | `Time::HiRes::time() * 1e9`                   |
| Swift      | `Int64(Date().timeIntervalSince1970 * 1e9)`   |

Note: Times must be written as **u64 little-endian** into WASM memory (8 bytes).

---

## References

- [WASI snapshot_preview1 witx spec](https://github.com/WebAssembly/WASI/blob/snapshot-01/phases/snapshot/witx/wasi_snapshot_preview1.witx)
- [Wasmtime WASI implementation (Rust)](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi)
- [WASI SDK](https://github.com/WebAssembly/wasi-sdk) — compile C/Rust to WASI WASM
- [WebAssembly Interface Types proposal](https://github.com/WebAssembly/interface-types) (basis for Preview 2)
