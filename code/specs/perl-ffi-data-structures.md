# Perl FFI Data Structure Packages — FFI::Platypus Wrappers for Rust cdylib

## 1. Overview

The starter packages (Spec 4) include pure Perl implementations of five
data structures: bitset, directed-graph, matrix, tree, and immutable-list.
Pure Perl is ideal for learning and portability, but performance suffers for
large inputs — Perl loops are roughly 10-50x slower than compiled Rust for
numerical and graph workloads.

This spec adds **FFI-accelerated** variants that call into Rust shared
libraries via `FFI::Platypus`. The pattern is well-established on CPAN:

```
CPAN Pattern:               Our Pattern:
  JSON::PP   (pure Perl)      CodingAdventures::Bitset      (pure Perl)
  JSON::XS   (C extension)    CodingAdventures::Bitset::FFI (Rust via FFI)
  JSON::MaybeXS (pick best)   (future: auto-selection)
```

The FFI versions have the **same public API** as the pure Perl versions.
Users can swap one for the other with a single `use` statement change.

### What This Spec Covers

1. **5 new Rust crates** (`*-ffi`) that expose C-compatible FFI surfaces
   from the existing Rust data structure libraries.
2. **5 new Perl `::FFI` modules** that wrap the Rust shared libraries using
   FFI::Platypus and present Perl OO interfaces.
3. The **opaque pointer pattern** for managing Rust heap objects from Perl.
4. **Memory safety rules** for the FFI boundary.

### What This Spec Does NOT Cover

- Replacing the pure Perl implementations — both coexist.
- Auto-selection (a `::MaybeFFI` module) — that is a future extension.
- XS bindings — FFI::Platypus is used exclusively.

---

## 2. Where It Fits

```
Application code
    |
    +---> use CodingAdventures::Bitset;         # Pure Perl (Spec 4)
    |
    +---> use CodingAdventures::Bitset::FFI;    # This spec
              |
              v
          FFI::Platypus (libffi runtime)
              |
              v
          libbitset_ffi.dylib / .so              # Rust cdylib
              |
              v
          bitset crate (existing Rust library)
```

### Layer Diagram

```
Perl Layer:
  CodingAdventures::Bitset::FFI          (Perl OO wrapper)
  CodingAdventures::DirectedGraph::FFI   (Perl OO wrapper)
  CodingAdventures::Matrix::FFI          (Perl OO wrapper)
  CodingAdventures::Tree::FFI            (Perl OO wrapper)
  CodingAdventures::ImmutableList::FFI   (Perl OO wrapper)
      |
FFI Layer:
  FFI::Platypus 2.00+                    (CPAN module, uses libffi)
  FFI::Platypus::Lang::Rust              (optional: Rust type names)
      |
Rust Layer:
  bitset-ffi            (cdylib)  --> bitset            (rlib)
  directed-graph-ffi    (cdylib)  --> directed-graph    (rlib)
  matrix-ffi            (cdylib)  --> matrix            (rlib)
  tree-ffi              (cdylib)  --> tree              (rlib)
  immutable-list-ffi    (cdylib)  --> immutable-list    (rlib)
```

---

## 3. Architecture: The Opaque Pointer Pattern

This is the central design pattern for the entire spec. Understanding it is
essential.

### 3.1 The Problem

Rust structs live on the Rust heap. Perl code cannot directly access Rust
memory — the two languages have different memory models, different type
systems, and different garbage collectors (Perl uses reference counting;
Rust uses ownership and borrowing).

### 3.2 The Solution: Opaque Pointers

We treat Rust structs as **opaque blobs** from Perl's perspective. Perl
holds a raw pointer (an integer-sized memory address) and passes it back
to Rust for every operation:

```
Perl:  $ptr = bitset_new(100)     # Receive opaque pointer
Perl:  bitset_set($ptr, 42)       # Pass pointer back to Rust
Perl:  bitset_free($ptr)          # Tell Rust to deallocate
```

### 3.3 Rust Side: Box::into_raw / Box::from_raw

```rust
// Create: allocate on heap, leak the Box, return raw pointer
#[no_mangle]
pub extern "C" fn bitset_new(size: usize) -> *mut Bitset {
    Box::into_raw(Box::new(Bitset::new(size)))
}

// Method: borrow the pointer, call the method, don't free
#[no_mangle]
pub extern "C" fn bitset_set(ptr: *mut Bitset, index: usize) {
    let bs = unsafe { &mut *ptr };
    bs.set(index);
}

// Free: reclaim the Box, let Rust drop it
#[no_mangle]
pub extern "C" fn bitset_free(ptr: *mut Bitset) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)); }
    }
}
```

**Key rules:**
- `Box::into_raw` transfers ownership to the caller (Perl). Rust will
  **not** drop the struct — Perl is responsible for calling `_free`.
- `Box::from_raw` reclaims ownership. Rust drops the struct.
- Between creation and free, we borrow the pointer with `&*ptr` or
  `&mut *ptr`. This does not transfer ownership.

### 3.4 Perl Side: Blessed Opaque Pointer

```perl
package CodingAdventures::Bitset::FFI;
use strict;
use warnings;
use FFI::Platypus 2.00;

my $ffi = FFI::Platypus->new(api => 2);
$ffi->lib('./target/release/libbitset_ffi.dylib');

$ffi->attach(bitset_new  => ['size_t']          => 'opaque');
$ffi->attach(bitset_set  => ['opaque', 'size_t'] => 'void');
$ffi->attach(bitset_get  => ['opaque', 'size_t'] => 'uint8');
$ffi->attach(bitset_free => ['opaque']           => 'void');

sub new {
    my ($class, $size) = @_;
    my $ptr = bitset_new($size);
    return bless { ptr => $ptr }, $class;
}

sub set {
    my ($self, $index) = @_;
    bitset_set($self->{ptr}, $index);
    return $self;
}

sub get {
    my ($self, $index) = @_;
    return bitset_get($self->{ptr}, $index);
}

sub DESTROY {
    my ($self) = @_;
    bitset_free($self->{ptr}) if $self->{ptr};
    $self->{ptr} = undef;
}
```

**Why DESTROY works:** Perl's reference counting is deterministic. When the
last reference to a `CodingAdventures::Bitset::FFI` object goes out of
scope, `DESTROY` is called immediately — no GC delay. This means Rust
memory is freed promptly.

### 3.5 Panic Safety

A Rust panic that crosses the FFI boundary is **undefined behavior**. Every
`extern "C"` function must catch panics:

```rust
#[no_mangle]
pub extern "C" fn bitset_set(ptr: *mut Bitset, index: usize) -> i32 {
    let result = std::panic::catch_unwind(|| {
        let bs = unsafe { &mut *ptr };
        bs.set(index);
    });
    match result {
        Ok(()) => 0,   // success
        Err(_) => -1,  // error (e.g., index out of bounds)
    }
}
```

The Perl wrapper checks the return code and dies on error:

```perl
sub set {
    my ($self, $index) = @_;
    my $rc = bitset_set($self->{ptr}, $index);
    die "Bitset::FFI: set failed (index $index out of bounds?)\n" if $rc != 0;
    return $self;
}
```

---

## 4. Rust cdylib Crates

### 4.1 Crate Layout

Each data structure gets a `*-ffi` crate alongside the existing core crate:

```
code/packages/rust/
  bitset/                    # Existing core crate (rlib)
  bitset-ffi/                # NEW: cdylib wrapper
    Cargo.toml
    src/lib.rs
    BUILD
    README.md
    CHANGELOG.md
  directed-graph/            # Existing
  directed-graph-ffi/        # NEW
  ...
```

### 4.2 Cargo.toml Template

```toml
[package]
name = "bitset-ffi"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
bitset = { path = "../bitset" }
```

The `crate-type = ["cdylib"]` directive tells Cargo to produce a
C-compatible shared library (`.dylib` on macOS, `.so` on Linux, `.dll` on
Windows).

### 4.3 FFI Function Naming Convention

All exported functions follow the pattern: `<struct>_<method>`.

| Convention | Example |
|-----------|---------|
| Constructor | `bitset_new` |
| Method | `bitset_set`, `bitset_get` |
| Destructor | `bitset_free` |
| String return | `bitset_to_string` |
| String free | `string_free` |
| Result length | `bitset_iter_set_bits_len` |
| Result element | `bitset_iter_set_bits_get` |

### 4.4 Attributes on Every Function

```rust
#[no_mangle]        // Prevent symbol mangling
pub extern "C"      // Use C calling convention
fn bitset_new(...)  // Function name = symbol name
```

---

## 5. Type Mapping

### 5.1 Scalar Types

| Rust Type | C Type | FFI::Platypus Type | Perl Side |
|-----------|--------|-------------------|-----------|
| `*mut Struct` | `void*` | `opaque` | Integer (pointer) |
| `usize` | `size_t` | `size_t` | Integer |
| `isize` | `ssize_t` | `ssize_t` | Integer |
| `bool` | `uint8_t` | `uint8` | 0 or 1 |
| `f64` | `double` | `double` | Float |
| `i32` | `int32_t` | `sint32` | Integer |
| `i64` | `int64_t` | `sint64` | Integer |
| `*const c_char` | `const char*` | `string` | String |

### 5.2 Complex Returns: Arrays and Lists

Rust functions that return vectors or lists cannot return them directly
across FFI (variable-length arrays don't have a C ABI). We use the
**indexed access pattern**:

```rust
// Rust: store result in a thread-local buffer
static RESULT_BUF: RefCell<Vec<usize>> = RefCell::new(Vec::new());

#[no_mangle]
pub extern "C" fn bitset_iter_set_bits(ptr: *const Bitset) -> usize {
    let bs = unsafe { &*ptr };
    let indices: Vec<usize> = bs.iter_set_bits().collect();
    let len = indices.len();
    RESULT_BUF.with(|buf| *buf.borrow_mut() = indices);
    len  // return count
}

#[no_mangle]
pub extern "C" fn bitset_iter_set_bits_get(index: usize) -> usize {
    RESULT_BUF.with(|buf| buf.borrow()[index])
}
```

Perl side:

```perl
sub iter_set_bits {
    my ($self) = @_;
    my $len = bitset_iter_set_bits($self->{ptr});
    return map { bitset_iter_set_bits_get($_) } 0 .. $len - 1;
}
```

**Alternative approaches considered:**

| Approach | Pros | Cons | Decision |
|----------|------|------|----------|
| Indexed access (thread-local) | Simple FFI, no memory management | Not thread-safe | **Chosen** |
| Comma-separated string | Very simple | Parsing overhead, string alloc | Too hacky |
| Opaque iterator | Clean API | Complex (next/has_next/free) | Over-engineered |
| Caller-allocated buffer | No global state | Caller must know size upfront | Awkward |

Thread safety is not a concern because Perl's threading model (`ithreads`)
clones the entire interpreter, so each thread gets its own `RESULT_BUF`.

### 5.3 String Returns

Strings returned from Rust must be heap-allocated and freed by the caller:

```rust
#[no_mangle]
pub extern "C" fn bitset_to_string(ptr: *const Bitset) -> *mut c_char {
    let bs = unsafe { &*ptr };
    let s = bs.to_string();
    CString::new(s).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)); }
    }
}
```

Perl side:

```perl
sub to_string {
    my ($self) = @_;
    my $ptr = bitset_to_string($self->{ptr});
    my $str = $ffi->cast('opaque', 'string', $ptr);
    string_free($ptr);
    return $str;
}
```

**Critical rule:** Never call C's `free()` on a Rust-allocated string.
Rust may use a different allocator. Always use the companion `string_free`
function.

---

## 6. Package Matrix

### 6.1 Rust FFI Crates (5 new)

| Crate | Depends On | Key Exports |
|-------|-----------|-------------|
| `bitset-ffi` | `bitset` | `bitset_new`, `bitset_free`, `bitset_set`, `bitset_get`, `bitset_clear`, `bitset_popcount`, `bitset_and`, `bitset_or`, `bitset_xor`, `bitset_not`, `bitset_iter_set_bits`, `bitset_iter_set_bits_get`, `bitset_to_string`, `string_free` |
| `directed-graph-ffi` | `directed-graph` | `graph_new`, `graph_free`, `graph_add_node`, `graph_add_edge`, `graph_topological_sort`, `graph_topo_len`, `graph_topo_get`, `graph_independent_groups`, `graph_groups_count`, `graph_group_len`, `graph_group_get`, `graph_affected_nodes`, `graph_affected_len`, `graph_affected_get`, `string_free` |
| `matrix-ffi` | `matrix` | `matrix_new_2d`, `matrix_zeros`, `matrix_free`, `matrix_rows`, `matrix_cols`, `matrix_get`, `matrix_set`, `matrix_add`, `matrix_subtract`, `matrix_scale`, `matrix_transpose`, `matrix_dot`, `matrix_to_string`, `string_free` |
| `immutable-list-ffi` | `immutable-list` | `ilist_new`, `ilist_free`, `ilist_push`, `ilist_pop`, `ilist_get`, `ilist_set`, `ilist_len` |
| `tree-ffi` | `tree` | `tree_new`, `tree_free`, `tree_add_child`, `tree_parent`, `tree_children_len`, `tree_children_get`, `tree_depth`, `tree_height`, `tree_leaves_len`, `tree_leaves_get`, `tree_preorder_len`, `tree_preorder_get`, `tree_postorder_len`, `tree_postorder_get`, `tree_level_order_len`, `tree_level_order_get`, `tree_lca`, `tree_to_ascii`, `string_free` |

### 6.2 Perl FFI Modules (5 new)

| Module | Location | Wraps |
|--------|----------|-------|
| `CodingAdventures::Bitset::FFI` | `code/packages/perl/bitset/lib/CodingAdventures/Bitset/FFI.pm` | `bitset-ffi` |
| `CodingAdventures::DirectedGraph::FFI` | `code/packages/perl/directed-graph/lib/CodingAdventures/DirectedGraph/FFI.pm` | `directed-graph-ffi` |
| `CodingAdventures::Matrix::FFI` | `code/packages/perl/matrix/lib/CodingAdventures/Matrix/FFI.pm` | `matrix-ffi` |
| `CodingAdventures::Tree::FFI` | `code/packages/perl/tree/lib/CodingAdventures/Tree/FFI.pm` | `tree-ffi` |
| `CodingAdventures::ImmutableList::FFI` | `code/packages/perl/immutable-list/lib/CodingAdventures/ImmutableList/FFI.pm` | `immutable-list-ffi` |

---

## 7. BUILD File Pattern

Each Perl package with an FFI module has a BUILD that first compiles the
Rust crate, then runs Perl tests:

```bash
# Build the Rust cdylib (relative path from perl package to rust crate)
cd ../../rust/bitset-ffi && cargo build --release
# Install Perl deps and run tests
cd ../../perl/bitset && cpanm --installdeps --quiet . && prove -l -v t/
```

The Rust crate outputs to `target/release/libbitset_ffi.dylib` (macOS) or
`target/release/libbitset_ffi.so` (Linux). The Perl FFI module locates the
library using a relative path from the package directory.

### Library Discovery

```perl
use File::Spec;
use FFI::Platypus 2.00;

my $lib_name = 'bitset_ffi';
my $lib_dir = File::Spec->catdir(
    File::Spec->rel2abs(File::Spec->curdir),
    '..', '..', 'rust', 'bitset-ffi', 'target', 'release'
);

my $ffi = FFI::Platypus->new(api => 2);
$ffi->lib(File::Spec->catfile($lib_dir, "lib${lib_name}.dylib"));
# TODO: handle .so on Linux, .dll on Windows
```

A more robust approach uses `FFI::CheckLib`:

```perl
use FFI::CheckLib qw(find_lib_or_die);
$ffi->lib(find_lib_or_die(lib => 'bitset_ffi', libpath => $lib_dir));
```

---

## 8. Perl Module Structure

Each FFI module lives alongside the pure Perl module in the same package:

```
code/packages/perl/bitset/
  lib/CodingAdventures/
    Bitset.pm                 # Pure Perl (Spec 4)
    Bitset/
      FFI.pm                  # FFI wrapper (this spec)
  t/
    00-load.t                 # Tests pure Perl
    01-bitset.t               # Tests pure Perl
    02-ffi-load.t             # Tests FFI module loads
    03-ffi-bitset.t           # Tests FFI (same test logic as 01)
```

**API compatibility:** The FFI module implements the exact same methods as
the pure Perl module. Tests should be sharable — a test helper module can
run the same test cases against both backends:

```perl
# t/lib/BitsetTests.pm
sub run_tests {
    my ($class) = @_;  # 'CodingAdventures::Bitset' or '...::Bitset::FFI'
    my $bs = $class->new(100);
    $bs->set(42);
    is($bs->get(42), 1, "$class: set and get");
    ...
}
```

---

## 9. Test Strategy

### 9.1 Per-Package Test Counts

Each FFI package runs two test suites:
1. **FFI-specific tests** — library loading, DESTROY, error handling.
2. **API compatibility tests** — same tests as pure Perl (shared).

| Package | FFI-specific | API shared | Total |
|---------|-------------|-----------|-------|
| bitset | 10 | 40 | 50 |
| directed-graph | 10 | 35 | 45 |
| matrix | 10 | 35 | 45 |
| tree | 10 | 35 | 45 |
| immutable-list | 8 | 30 | 38 |
| **Total** | **48** | **175** | **~223** |

### 9.2 FFI-Specific Test Categories

| # | Test Category | Example |
|---|--------------|---------|
| 1 | Library loads | `use_ok('CodingAdventures::Bitset::FFI')` |
| 2 | Constructor returns object | `isa_ok($bs, 'CodingAdventures::Bitset::FFI')` |
| 3 | DESTROY called on scope exit | Create in block, verify no leak |
| 4 | Double-free safety | Call DESTROY twice, no crash |
| 5 | Null pointer safety | Operations on freed object die gracefully |
| 6 | Error propagation | Out-of-bounds set → Perl die |
| 7 | String return/free | `to_string` returns valid Perl string |
| 8 | Array return | `iter_set_bits` returns correct list |
| 9 | Large dataset | 100K elements — verify correctness |
| 10 | API matches pure Perl | Same method names, same semantics |

### 9.3 Rust FFI Crate Tests

Each Rust `-ffi` crate should also have its own Rust tests (`#[cfg(test)]`)
that verify the FFI functions work correctly when called from Rust (before
the Perl layer is added):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitset_create_and_free() {
        let ptr = bitset_new(100);
        assert!(!ptr.is_null());
        bitset_set(ptr, 42);
        assert_eq!(bitset_get(ptr, 42), 1);
        bitset_free(ptr);
    }
}
```

---

## 10. Implementation Sequence

```
Phase 1: Rust FFI crates (all 5 in parallel)
    bitset-ffi, directed-graph-ffi, matrix-ffi, tree-ffi, immutable-list-ffi
    ↓
Phase 2: Perl FFI modules (start with bitset — simplest API)
    CodingAdventures::Bitset::FFI
    ↓
Phase 3: Remaining Perl FFI modules (parallel)
    CodingAdventures::DirectedGraph::FFI
    CodingAdventures::Matrix::FFI
    CodingAdventures::Tree::FFI
    CodingAdventures::ImmutableList::FFI
```

**Why bitset first:** It has the simplest API (scalars in, scalars out, one
array return). It proves the entire pattern — Rust cdylib, opaque pointers,
FFI::Platypus attachment, DESTROY cleanup — before tackling more complex
structures like directed-graph (which involves string parameters) or tree
(which involves nested returns).

---

## 11. Memory Safety Rules

These rules must be followed in every FFI function to prevent memory leaks,
use-after-free, and undefined behavior:

### 11.1 Ownership Rules

| Operation | Rust side | Perl side |
|-----------|-----------|-----------|
| Create | `Box::into_raw(Box::new(...))` | Receives opaque pointer, stores in blessed hash |
| Borrow | `unsafe { &*ptr }` or `unsafe { &mut *ptr }` | Passes pointer to FFI function |
| Free | `unsafe { drop(Box::from_raw(ptr)); }` | Calls `_free` in `DESTROY` |

### 11.2 String Ownership

| Direction | Ownership | Rule |
|-----------|-----------|------|
| Perl → Rust | Perl owns | Rust borrows via `CStr::from_ptr`, must NOT free |
| Rust → Perl | Rust allocates | Perl must call `string_free` after copying |

### 11.3 Array Ownership

| Direction | Ownership | Rule |
|-----------|-----------|------|
| Perl → Rust | Perl owns | Rust borrows via `slice::from_raw_parts`, must NOT free |
| Rust → Perl | Thread-local buffer | Perl reads via indexed access, buffer reused next call |

### 11.4 DESTROY Contract

Every Perl `DESTROY` method must:
1. Check if the pointer is defined and non-null.
2. Call the corresponding `_free` function exactly once.
3. Set the pointer to `undef` to prevent double-free.

```perl
sub DESTROY {
    my ($self) = @_;
    if ($self->{ptr}) {
        bitset_free($self->{ptr});
        $self->{ptr} = undef;
    }
}
```

---

## 12. Trade-Offs

### 12.1 FFI::Platypus vs XS

| | FFI::Platypus | XS |
|-|---------------|-----|
| Compiler needed at install | No (uses libffi) | Yes (C compiler) |
| Learning curve | Low (Perl-only) | High (XS macros, perlguts) |
| Performance | ~60-70% of XS | Maximum |
| Debugging | Standard Perl tools | Valgrind, gdb |
| Code readability | High | Low |
| **Decision** | **FFI::Platypus** | — |

For data structures, the 30-40% overhead of FFI vs XS is negligible
compared to the 10-50x speedup over pure Perl.

### 12.2 Separate `-ffi` Crates vs Feature Flags

| | Separate crates | Feature flags |
|-|-----------------|---------------|
| Core crate pollution | None | Adds cdylib + FFI deps |
| Build independence | Each builds alone | All features build together |
| Cargo workspace impact | More members | Fewer members |
| **Decision** | **Separate crates** | — |

Separate crates keep the core Rust libraries pure — no `#[no_mangle]`,
no `extern "C"`, no `catch_unwind`. The FFI surface is an adapter layer.

### 12.3 `::FFI` Submodule vs Replacing Pure Perl

| | Separate `::FFI` module | Replace pure Perl |
|-|------------------------|-------------------|
| User choice | Explicit opt-in | Forced |
| Educational value | Compare both implementations | Only one available |
| Fallback | Pure Perl always available | None if Rust not compiled |
| **Decision** | **Separate `::FFI` module** | — |

Both implementations coexist. Users explicitly choose which to use. This
matches the CPAN convention (JSON::PP vs JSON::XS) and supports the
project's educational goals.

---

## 13. Dependencies

### 13.1 Perl Dependencies

| Module | Version | Purpose | Source |
|--------|---------|---------|--------|
| `FFI::Platypus` | >= 2.00 | Foreign function interface | CPAN |
| `FFI::CheckLib` | >= 0.28 | Library discovery | CPAN |
| `Test2::V0` | any | Testing | CPAN |

### 13.2 Rust Dependencies

Each `-ffi` crate depends only on its corresponding core crate. No external
Rust dependencies.

### 13.3 Build-Time Requirements

- **Rust toolchain** (cargo, rustc) — to compile cdylib crates.
- **Perl 5.26+** — for the Perl modules.
- **cpanm** — to install FFI::Platypus.

---

## 14. Rust Workspace Integration

The 5 new `-ffi` crates must be added to the Rust workspace at
`code/packages/rust/Cargo.toml`:

```toml
[workspace]
members = [
    # ... existing members ...
    "bitset-ffi",
    "directed-graph-ffi",
    "immutable-list-ffi",
    "matrix-ffi",
    "tree-ffi",
]
```

**Important lesson from `lessons.md`:** Only add workspace members in the
same commit that pushes the crate directory. Otherwise, all Rust packages
fail to compile in CI with "failed to load manifest" errors.

---

## 15. Future Extensions

- **Auto-selection module:** A `CodingAdventures::Bitset::MaybeFFI` that
  tries to load the FFI version and falls back to pure Perl.
- **Benchmark suite:** Compare pure Perl vs FFI for each data structure
  at various input sizes.
- **Additional data structures:** Wrap other Rust packages (cache,
  state-machine, etc.) as the need arises.
- **Thread-safe wrappers:** For Perl ithreads, wrap the opaque pointer
  in a `CLONE` method that creates a deep copy.
- **WASM alternative:** Compile Rust crates to WASM and call from Perl
  via `Wasm::Wasmtime` — cross-platform, no native compilation needed.
- **FFI::Platypus::Bundle integration:** Bundle Rust source inside the
  Perl CPAN distribution so `cargo build` runs automatically at install
  time via `FFI::Build::File::Cargo`.
