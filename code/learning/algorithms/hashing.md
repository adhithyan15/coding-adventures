# Hashing -- Detecting Change Without Comparing Everything

This document covers hash functions and how the coding-adventures build tool
uses hashing to decide whether a package needs rebuilding. We start from
first principles (what is a hash function?), work up to SHA-256, and then
show exactly how the build tool's two-level hashing scheme works.

**Implementations referenced in this document:**

- Build tool hasher: `code/programs/go/build-tool/internal/hasher/hasher.go`
- Build cache: `code/programs/go/build-tool/internal/cache/cache.go`

---

## Table of Contents

1. [What Is a Hash Function?](#1-what-is-a-hash-function)
2. [Properties of a Good Hash Function](#2-properties-of-a-good-hash-function)
3. [SHA-256](#3-sha-256)
4. [Two-Level Hashing for Build Caching](#4-two-level-hashing-for-build-caching)
5. [Why Sorted Order Matters](#5-why-sorted-order-matters)
6. [Content-Addressable Storage](#6-content-addressable-storage)
7. [How the Build Tool's Hasher Works](#7-how-the-build-tools-hasher-works)
8. [Dependency Hashing](#8-dependency-hashing)
9. [The Build Cache](#9-the-build-cache)
10. [Putting It All Together](#10-putting-it-all-together)

---

## 1. What Is a Hash Function?

A **hash function** takes an input of arbitrary size and produces a
fixed-size output (the "hash", "digest", or "fingerprint"). Think of it
as a summary of the input -- a short string that uniquely (in practice)
identifies the content.

```
  Input (any size)          Hash function          Output (fixed size)
  +------------------+                           +------------------+
  | "Hello, world!"  | -----> SHA-256 ------>    | "315f5bdb76d0..."  |
  +------------------+                           +------------------+
  | (13 bytes)       |                           | (64 hex chars)   |
  +------------------+                           +------------------+

  +------------------+                           +------------------+
  | (entire novel,   | -----> SHA-256 ------>    | "a1b2c3d4e5f6..."  |
  |  500,000 bytes)  |                           | (64 hex chars)   |
  +------------------+                           +------------------+
```

No matter how large or small the input, SHA-256 always produces a
256-bit (32-byte, 64-hex-character) output.

### Analogy: Fingerprints

A hash is like a fingerprint for data. Just as every person has a unique
fingerprint (a small, fixed-size identifier), every file has a unique hash.
If two files produce the same hash, they are (almost certainly) identical.
If they produce different hashes, they are definitely different.

---

## 2. Properties of a Good Hash Function

A hash function useful for change detection must have three properties:

### Determinism

The same input ALWAYS produces the same output. If you hash a file today and
hash the same unchanged file tomorrow, you get the same hash.

```
  hash("Hello") = "185f8db32271..."    (today)
  hash("Hello") = "185f8db32271..."    (tomorrow)
  hash("Hello") = "185f8db32271..."    (on a different computer)
```

This is what makes caching possible. We hash a package's files at build time,
store the hash, and later compare the stored hash with the current hash.
Same hash = nothing changed = no need to rebuild.

### Uniformity

The hash function should spread outputs evenly across the output space. This
means small input changes should produce very different hashes, not similar
ones. Two files that differ by a single byte should have completely unrelated
hashes.

```
  hash("Hello")  = "185f8db32271fe25..."
  hash("Hello!") = "334d016f755cd6dc..."
                     ^^^^^^^^^^^^^^^^
                     Completely different!
```

### Avalanche Effect

Changing even one bit of the input should change approximately half the bits
of the output. This is a stronger form of uniformity.

```
  Input:   "Hello world"
  SHA-256: "64ec88ca00b268e5ba1a35678a1b5316d212f4f366b2477232534a8aeca37f3c"

  Input:   "Hello World"   (capital W)
  SHA-256: "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e"
           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
           Every character of the hash is different!
```

This property means you cannot "guess" the hash of a modified file from the
hash of the original. Any change, no matter how small, produces a completely
new hash.

### Truth Table: Hash Properties

```
  +-----------------------------+------------+-----------+
  | Scenario                    | Same hash? | Rebuild?  |
  +-----------------------------+------------+-----------+
  | No files changed            | Yes        | No        |
  | One file modified           | No         | Yes       |
  | File renamed (same content) | No*        | Yes       |
  | File added                  | No         | Yes       |
  | File removed                | No         | Yes       |
  | Whitespace-only change      | No         | Yes       |
  | Comment-only change         | No         | Yes       |
  +-----------------------------+------------+-----------+

  * Renaming changes the sorted file list, which changes the hash.
```

Note that the hash does not understand semantics. A comment-only change in
a source file changes the hash and triggers a rebuild, even though the
compiled output might be identical. This is a deliberate trade-off: false
positives (unnecessary rebuilds) are safe, while false negatives (missing
a real change) would be a correctness bug.

---

## 3. SHA-256

### What Is SHA-256?

SHA-256 is a member of the SHA-2 (Secure Hash Algorithm 2) family, designed
by the NSA and published by NIST in 2001. The "256" refers to the output
size: 256 bits (32 bytes).

```
  SHA-256 at a glance:
  +-------------------+---------------------------+
  | Output size       | 256 bits (64 hex chars)   |
  | Block size        | 512 bits (64 bytes)       |
  | Rounds            | 64                        |
  | Collision resist. | 2^128 operations          |
  | Speed             | ~200 MB/s (software)      |
  +-------------------+---------------------------+
```

### Why SHA-256 for Build Caching?

We chose SHA-256 because:

1. **Collision resistance** -- The probability that two different files
   produce the same hash is astronomically low (approximately 1 in 2^128).
   For a build system, a false "same hash" would mean skipping a necessary
   rebuild -- a correctness bug. SHA-256 makes this effectively impossible.

2. **Deterministic** -- Same input always gives the same output, on every
   platform, every implementation.

3. **Standard library support** -- Go's `crypto/sha256` is built into the
   standard library. No external dependencies needed.

4. **Good enough speed** -- Hashing source files is much faster than
   compiling them. The overhead of hashing is negligible compared to the
   time saved by skipping unnecessary builds.

### What We Do NOT Need

SHA-256 is a cryptographic hash function, meaning it is designed to resist
intentional manipulation (pre-image attacks, collision attacks). For build
caching, we do not need cryptographic strength -- no one is trying to craft
adversarial source files. A simpler, faster hash (like xxHash or FNV) would
work. But SHA-256 is standard, available everywhere, and fast enough.

---

## 4. Two-Level Hashing for Build Caching

The build tool does not hash the entire package directory as one blob.
Instead, it uses a **two-level hashing scheme**:

```
  Level 1: Hash each source file individually
  Level 2: Concatenate all file hashes, hash the result

  File A contents ---> SHA-256 ---> hash_A = "aaa111..."
  File B contents ---> SHA-256 ---> hash_B = "bbb222..."
  File C contents ---> SHA-256 ---> hash_C = "ccc333..."

  Concatenate:  "aaa111...bbb222...ccc333..."
                          |
                          v
                       SHA-256
                          |
                          v
                  Final hash: "fff999..."
```

### Why Not Just Hash Everything at Once?

You could concatenate all file contents and hash once. The two-level scheme
has no correctness advantage, but it is a common pattern because:

1. **Streaming** -- Each file can be hashed independently as it is read,
   using a streaming SHA-256 (reading 8KB at a time). This avoids loading
   large files entirely into memory.

2. **Debuggability** -- You can print individual file hashes to see exactly
   which file changed. With a single-pass hash, you only know "something
   changed" but not what.

3. **Composability** -- The same per-file hashes can be reused for other
   purposes (e.g., deduplication, comparing specific files).

### The Complete Procedure

From `code/programs/go/build-tool/internal/hasher/hasher.go`:

```
  1. Collect all source files in the package directory
     - Filter by language extensions (.py, .go, .rb, etc.)
     - Always include BUILD files
     - Include special files (go.mod, Gemfile, etc.)

  2. Sort the file list by relative path (lexicographic)
     - This makes the hash deterministic regardless of
       filesystem traversal order

  3. Hash each file's contents with SHA-256
     - Read in 8KB chunks (streaming, memory-efficient)
     - If a file cannot be read, use "error-reading-file"
       as the hash (ensures a rebuild)

  4. Concatenate all individual hashes into one string
     - "aaa111...bbb222...ccc333..."

  5. Hash the concatenated string with SHA-256
     - Produces the final package hash
```

### What Changes the Hash

```
  +-------------------------------+------------------+
  | Action                        | Hash changes?    |
  +-------------------------------+------------------+
  | Edit a .py file               | Yes              |
  | Add a new .py file            | Yes (new hash)   |
  | Remove a .py file             | Yes (fewer hashes)|
  | Rename a .py file             | Yes (sort order) |
  | Edit the BUILD file           | Yes              |
  | Edit a .txt file (not .py)    | No (filtered out)|
  | Change file permissions       | No (contents same)|
  | Touch file (update timestamp) | No (contents same)|
  +-------------------------------+------------------+
```

---

## 5. Why Sorted Order Matters

### The Problem

File system traversal order is not guaranteed. On one machine, `os.Walk`
might return files in this order:

```
  Machine A:   src/add.py, src/multiply.py, tests/test_add.py
  Machine B:   tests/test_add.py, src/multiply.py, src/add.py
```

If we hash files in traversal order, the same package would produce
different hashes on different machines. That breaks determinism -- the
fundamental requirement for caching.

### The Solution

Sort the file list before hashing. Both machines will hash files in the
same order, producing the same final hash:

```
  Sorted:  src/add.py, src/multiply.py, tests/test_add.py

  Machine A: hash(add.py) + hash(multiply.py) + hash(test_add.py) -> SHA-256
  Machine B: hash(add.py) + hash(multiply.py) + hash(test_add.py) -> SHA-256
                                                                      ^
                                                                   Same!
```

### Sorting by Relative Path

We sort by **relative** path, not absolute path. Two developers with
different checkout locations should get the same hash:

```
  Developer A: /home/alice/coding-adventures/code/packages/python/logic-gates/
  Developer B: /home/bob/projects/coding-adventures/code/packages/python/logic-gates/

  Absolute paths differ, but relative paths are the same:
    src/and_gate.py
    src/or_gate.py
    tests/test_gates.py
```

From `hasher.go`:

```go
sort.Slice(files, func(i, j int) bool {
    relI, _ := filepath.Rel(pkg.Path, files[i])
    relJ, _ := filepath.Rel(pkg.Path, files[j])
    return relI < relJ
})
```

---

## 6. Content-Addressable Storage

### The Concept

Content-addressable storage (CAS) is a storage scheme where the address
(identifier) of data is derived from the data's content -- typically its
hash. If two files have the same content, they have the same address.

```
  Traditional storage:        Content-addressable storage:
  +--------+----------+       +------------------+----------+
  | Name   | Content  |       | Hash (address)   | Content  |
  +--------+----------+       +------------------+----------+
  | foo.py | print(1) |       | abc123...        | print(1) |
  | bar.py | print(1) |       |                  |          |
  | baz.py | print(2) |       | def456...        | print(2) |
  +--------+----------+       +------------------+----------+

  foo.py and bar.py have       Same content = same hash =
  different names but          stored only once.
  same content (stored twice)
```

### How the Build Cache Uses This Idea

The build cache does not store file contents -- it stores hashes. But the
principle is the same: **the identity of a package's state is its hash.**

```
  Build cache entry:
  {
      "python/logic-gates": {
          "package_hash": "abc123...",   <-- content-derived identity
          "deps_hash": "def456...",      <-- content-derived identity
          "last_built": "2024-01-15T10:30:00Z",
          "status": "success"
      }
  }
```

If `package_hash` matches the current hash, the package has not changed.
The hash IS the identity. We do not need to compare files byte-by-byte --
comparing two 64-character strings is enough.

### Real-World CAS Systems

- **Git** -- Every object (file, tree, commit) is stored by its SHA-1 hash.
  That is why `git` can instantly tell if two files are identical.
- **Docker** -- Image layers are content-addressed. Identical layers are
  shared across images.
- **IPFS** -- Files are addressed by their hash. If two users upload the
  same file, it is stored once.
- **Nix/Guix** -- Build outputs are stored in `/nix/store/<hash>-<name>/`.
  Same inputs = same hash = same output.

---

## 7. How the Build Tool's Hasher Works

Let us trace the hasher step by step for a hypothetical Python package.

### Input: A Package Directory

```
  code/packages/python/logic-gates/
  +-- BUILD
  +-- pyproject.toml
  +-- src/
  |   +-- logic_gates/
  |       +-- __init__.py
  |       +-- and_gate.py
  |       +-- or_gate.py
  +-- tests/
      +-- test_gates.py
      +-- __init__.py
```

### Step 1: Collect Source Files

The hasher walks the directory and collects files matching Python extensions
(`.py`, `.toml`, `.cfg`) plus BUILD files:

```
  Collected (unsorted):
    code/.../logic-gates/BUILD
    code/.../logic-gates/pyproject.toml
    code/.../logic-gates/src/logic_gates/__init__.py
    code/.../logic-gates/src/logic_gates/and_gate.py
    code/.../logic-gates/src/logic_gates/or_gate.py
    code/.../logic-gates/tests/__init__.py
    code/.../logic-gates/tests/test_gates.py
```

Files NOT collected (filtered out):
- `README.md` (not a Python extension)
- `.gitignore` (not a Python extension)
- `__pycache__/` (directory, not a file)

### Step 2: Sort by Relative Path

```
  Sorted by relative path:
    BUILD
    pyproject.toml
    src/logic_gates/__init__.py
    src/logic_gates/and_gate.py
    src/logic_gates/or_gate.py
    tests/__init__.py
    tests/test_gates.py
```

### Step 3: Hash Each File

```
  BUILD                       -> SHA-256 -> "a1b2c3..."
  pyproject.toml              -> SHA-256 -> "d4e5f6..."
  src/logic_gates/__init__.py -> SHA-256 -> "789abc..."
  src/logic_gates/and_gate.py -> SHA-256 -> "def012..."
  src/logic_gates/or_gate.py  -> SHA-256 -> "345678..."
  tests/__init__.py           -> SHA-256 -> "9abcde..."
  tests/test_gates.py         -> SHA-256 -> "f01234..."
```

Each file is hashed by reading its contents in 8KB chunks (streaming):

```go
// From hasher.go
func hashFile(path string) (string, error) {
    f, err := os.Open(path)
    if err != nil {
        return "", err
    }
    defer f.Close()

    h := sha256.New()
    if _, err := io.Copy(h, f); err != nil {
        return "", err
    }
    return hex.EncodeToString(h.Sum(nil)), nil
}
```

`io.Copy` reads the file in chunks and feeds each chunk to the SHA-256
hasher. This means even a 100MB file uses only ~8KB of memory for hashing.

### Step 4: Concatenate and Hash Again

```
  Concatenated: "a1b2c3...d4e5f6...789abc...def012...345678...9abcde...f01234..."
                                    |
                                    v
                                 SHA-256
                                    |
                                    v
                         Final hash: "xyz789..."
```

```go
// From hasher.go
combined := strings.Join(fileHashes, "")
h := sha256.Sum256([]byte(combined))
return hex.EncodeToString(h[:])
```

### Error Handling

If a file cannot be read (permissions, disk error), the hasher uses
`"error-reading-file"` as that file's hash. This sentinel value guarantees
the final hash will differ from any cached hash, forcing a rebuild:

```go
fh, err := hashFile(f)
if err != nil {
    fh = "error-reading-file"  // ensures cache mismatch -> rebuild
}
```

This is a safe default: when in doubt, rebuild.

---

## 8. Dependency Hashing

### The Problem

Hashing a package's own files is not enough. If package B depends on
package A, and A's files change, then B needs rebuilding too -- even though
B's own files have not changed.

### The Solution: Hash the Dependencies

The hasher computes a second hash for each package: the **dependency hash**
(`deps_hash`). This hash represents the state of all transitive dependencies.

```
  Package B depends on A.
  A depends on nothing.

  deps_hash(A) = hash("")       (no dependencies)
  deps_hash(B) = hash(package_hash(A))

  If A changes:
    package_hash(A) changes  -> deps_hash(B) changes -> B gets rebuilt
```

### How It Works

From `code/programs/go/build-tool/internal/hasher/hasher.go`:

```
  1. Find all transitive dependencies (predecessors in the graph)
     - Walk backwards through the graph using BFS
     - Collect all nodes reachable via reverse edges

  2. Sort the dependency names (determinism!)

  3. Concatenate their package hashes

  4. Hash the concatenation with SHA-256
```

```
  Example:
    B depends on A.
    C depends on A and B.

    deps_hash(A) = SHA-256("")                         = "e3b0c4..."
    deps_hash(B) = SHA-256(package_hash(A))            = "f1a2b3..."
    deps_hash(C) = SHA-256(package_hash(A) + package_hash(B)) = "c4d5e6..."
```

### Why Sort the Dependencies?

For the same reason we sort files: determinism. Without sorting, the
concatenation order depends on map iteration order, which is randomized
in Go. Two runs could produce different hashes for the same input.

```go
// From hasher.go
sorted := make([]string, 0, len(transitiveDeps))
for dep := range transitiveDeps {
    sorted = append(sorted, dep)
}
sort.Strings(sorted)

var combined strings.Builder
for _, dep := range sorted {
    combined.WriteString(packageHashes[dep])
}
```

---

## 9. The Build Cache

The build cache at `code/programs/go/build-tool/internal/cache/cache.go`
stores the hashes computed at build time and compares them on the next build.

### Cache Format

The cache is a JSON file (`.build-cache.json`) mapping package names to
their last-known state:

```json
{
    "python/logic-gates": {
        "package_hash": "abc123...",
        "deps_hash": "def456...",
        "last_built": "2024-01-15T10:30:00Z",
        "status": "success"
    },
    "python/arithmetic": {
        "package_hash": "789abc...",
        "deps_hash": "012def...",
        "last_built": "2024-01-15T10:30:05Z",
        "status": "success"
    }
}
```

### The NeedsBuild Decision

A package needs rebuilding if ANY of these conditions hold:

```
  +---+-------------------------------+-----------------------------------+
  | # | Condition                     | Why                               |
  +---+-------------------------------+-----------------------------------+
  | 1 | Not in cache                  | Never built before                |
  | 2 | package_hash changed          | Source files modified              |
  | 3 | deps_hash changed             | A dependency was modified         |
  | 4 | Last build failed             | Previous attempt did not succeed  |
  +---+-------------------------------+-----------------------------------+
```

```go
// From cache.go
func (c *BuildCache) NeedsBuild(name, pkgHash, depsHash string) bool {
    entry, ok := c.entries[name]
    if !ok {
        return true                    // condition 1
    }
    if entry.Status == "failed" {
        return true                    // condition 4
    }
    if entry.PackageHash != pkgHash {
        return true                    // condition 2
    }
    if entry.DepsHash != depsHash {
        return true                    // condition 3
    }
    return false
}
```

### Decision Flow Diagram

```
  Is package in cache?
        |
        +-- No  --> REBUILD
        |
        +-- Yes
             |
             Was last build "failed"?
                  |
                  +-- Yes --> REBUILD
                  |
                  +-- No
                       |
                       Did package_hash change?
                            |
                            +-- Yes --> REBUILD
                            |
                            +-- No
                                 |
                                 Did deps_hash change?
                                      |
                                      +-- Yes --> REBUILD
                                      |
                                      +-- No  --> SKIP (up to date)
```

### Atomic Writes

The cache file is written atomically to prevent corruption if the process
is interrupted:

```
  1. Write to .build-cache.json.tmp
  2. Rename .build-cache.json.tmp -> .build-cache.json

  If crash during step 1: old cache is intact (tmp is partial)
  If crash during step 2: on POSIX, rename is atomic within a filesystem
```

---

## 10. Putting It All Together

Here is the complete flow of hashing in the build tool, from file change
to rebuild decision:

```
  Developer edits code/packages/python/logic-gates/src/and_gate.py
       |
       v
  Build tool runs
       |
       v
  HashPackage("python/logic-gates"):
    1. Collect .py, .toml, BUILD files
    2. Sort by relative path
    3. SHA-256 each file
    4. Concatenate hashes, SHA-256 again
    -> new_pkg_hash = "NEW_HASH_123..."
       |
       v
  HashDeps("python/logic-gates", graph, packageHashes):
    1. Find transitive predecessors (none for logic-gates)
    2. Hash empty string
    -> new_deps_hash = "e3b0c4..."
       |
       v
  cache.NeedsBuild("python/logic-gates", "NEW_HASH_123...", "e3b0c4..."):
    - Cached package_hash = "OLD_HASH_456..."
    - "NEW_HASH_123..." != "OLD_HASH_456..."
    -> true (REBUILD!)
       |
       v
  Now check python/arithmetic (depends on logic-gates):
       |
       v
  HashPackage("python/arithmetic"):
    -> pkg_hash = "ARITH_789..."  (unchanged -- own files did not change)
       |
       v
  HashDeps("python/arithmetic", graph, packageHashes):
    1. Find transitive predecessors: {python/logic-gates}
    2. Concatenate: packageHashes["python/logic-gates"] = "NEW_HASH_123..."
    3. SHA-256("NEW_HASH_123...")
    -> new_deps_hash = "DEPS_NEW_ABC..."
       |
       v
  cache.NeedsBuild("python/arithmetic", "ARITH_789...", "DEPS_NEW_ABC..."):
    - Cached deps_hash = "DEPS_OLD_DEF..."
    - "DEPS_NEW_ABC..." != "DEPS_OLD_DEF..."
    -> true (REBUILD! -- even though own files unchanged)
```

The key insight: **changes propagate through the dependency graph via
hashing.** When logic-gates changes, its package hash changes, which changes
the deps hash of everything that depends on it, which triggers rebuilds up
the chain. No file comparison is needed -- just comparing short hash strings.

### Performance Impact

For a repo with 50 packages averaging 100 source files each:

```
  Without hashing:  Build all 50 packages every time
  With hashing:     Hash 50 * 100 = 5,000 files (~200ms)
                    Compare 50 hash pairs (~0ms)
                    Build only the 3 packages that changed (~30s saved)
```

The cost of hashing (milliseconds) is negligible compared to the cost of
unnecessary builds (minutes). This is why every serious build system uses
content hashing: Make, Bazel, Buck, Gradle, Turborepo, Nx, and now ours.
