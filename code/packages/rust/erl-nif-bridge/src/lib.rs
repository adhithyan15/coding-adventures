// Erlang's NIF API uses non-snake-case names inherited from C headers:
// ERL_NIF_TERM, ErlNifFunc, enif_make_int, etc. Allow these to match docs.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

//! # erl-nif-bridge — Zero-dependency Rust wrapper for Erlang's erl_nif C API
//!
//! This crate provides safe Rust wrappers around Erlang's Native Implemented
//! Function (NIF) C API using raw `extern "C"` declarations. No Rustler, no
//! erl_nif crate, no bindgen, no build-time header requirements. Compiles on
//! any platform with just a Rust toolchain.
//!
//! ## How it works
//!
//! When the BEAM (Erlang's VM) loads a NIF shared library, it resolves all
//! `erl_nif_*` symbols against the running OTP runtime (`erts`). We declare
//! these functions as `extern "C"` — the dynamic linker does the rest at
//! load time, exactly like Python's `libpython` or Ruby's `libruby`.
//!
//! ## The BEAM term model
//!
//! BEAM represents every Erlang value as an `ERL_NIF_TERM` — a single
//! machine word (usize) that is either:
//! - A tagged immediate: small integers, atoms, empty list `[]`, etc.
//! - A pointer to a heap-allocated term: tuples, binaries, large integers.
//!
//! This is almost identical to Ruby's `VALUE`. You never allocate or free
//! BEAM terms directly — the GC handles memory. Terms "belong" to an
//! environment (`ErlNifEnv`), which acts as the allocation arena.
//!
//! ## NIF entry convention
//!
//! Every NIF function has the signature:
//!
//! ```text
//! fn(env: ErlNifEnv, argc: c_int, argv: *const ERL_NIF_TERM) -> ERL_NIF_TERM
//! ```
//!
//! Arguments arrive in `argv[0..argc]`. Return one term. To signal an error,
//! call `enif_make_badarg` or `enif_raise_exception`.
//!
//! ## Why zero dependencies?
//!
//! - **Compiles everywhere** — no Erlang headers needed at build time
//! - **No Rustler macros** — every symbol is explicit and grep-able
//! - **No version conflicts** — works with any OTP 22+
//! - **Fully auditable** — no proc-macro magic

use std::ffi::{c_char, c_int, c_long, c_uint, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// Core BEAM term and environment types
// ---------------------------------------------------------------------------
//
// ERL_NIF_TERM is Erlang's equivalent of Ruby's VALUE or Python's PyObject*.
// It fits in one machine word and is either a tagged immediate or a pointer.

/// A BEAM term — a tagged machine word (like Ruby's VALUE).
///
/// Never create or interpret the bits of this type directly. Always use the
/// `enif_make_*` / `enif_get_*` functions to construct and inspect terms.
pub type ERL_NIF_TERM = usize;

/// Opaque pointer to a NIF environment.
///
/// The environment is an allocation arena: terms created via `enif_make_*`
/// "belong" to this env and are valid until the env is freed. Process
/// environments live for the duration of one NIF call; you can create
/// long-lived envs with `enif_alloc_env` for sending messages.
pub type ErlNifEnv = *mut c_void;

// ---------------------------------------------------------------------------
// Struct types used by the NIF API
// ---------------------------------------------------------------------------

/// Describes one exported NIF function.
///
/// Fill an array of these and pass it to `ErlNifEntry.funcs`. The array must
/// end with a sentinel entry (all zeros / null pointers). `flags` is normally
/// 0; set to `ERL_NIF_DIRTY_JOB_CPU_BOUND` or `ERL_NIF_DIRTY_JOB_IO_BOUND`
/// for functions that may block.
#[repr(C)]
pub struct ErlNifFunc {
    /// The Erlang function name as a null-terminated C string (e.g. `b"add\0"`).
    pub name: *const c_char,
    /// The function arity (number of arguments).
    pub arity: c_uint,
    /// The actual C function implementing this NIF.
    pub fptr: unsafe extern "C" fn(
        env: ErlNifEnv,
        argc: c_int,
        argv: *const ERL_NIF_TERM,
    ) -> ERL_NIF_TERM,
    /// Scheduling flags. 0 = normal scheduler, 1 = dirty CPU, 2 = dirty IO.
    pub flags: c_uint,
}

/// The module descriptor passed to `ERL_NIF_INIT`.
///
/// The BEAM runtime reads this struct when loading the NIF library to
/// discover which functions to bind and which lifecycle callbacks to call.
/// `major` must be 2. `minor` encodes the OTP version; use 16 for OTP 26+.
#[repr(C)]
pub struct ErlNifEntry {
    /// Must be `ERL_NIF_MAJOR_VERSION` (2).
    pub major: c_int,
    /// OTP version-dependent minor version; use `ERL_NIF_MINOR_VERSION` (16).
    pub minor: c_int,
    /// The Erlang module name this NIF is loaded into (null-terminated C string).
    pub name: *const c_char,
    /// Number of functions in the `funcs` array.
    pub num_of_funcs: c_int,
    /// Pointer to an array of `ErlNifFunc` descriptors.
    pub funcs: *const ErlNifFunc,
    /// Called when the module is first loaded. Return 0 for success.
    pub load: Option<
        unsafe extern "C" fn(
            env: ErlNifEnv,
            priv_data: *mut *mut c_void,
            load_info: ERL_NIF_TERM,
        ) -> c_int,
    >,
    /// Called on hot-code reload. Typically left as `None` (deprecated in OTP 20+).
    pub reload: Option<
        unsafe extern "C" fn(
            env: ErlNifEnv,
            priv_data: *mut *mut c_void,
            load_info: ERL_NIF_TERM,
        ) -> c_int,
    >,
    /// Called when a new version is loaded over the running module.
    pub upgrade: Option<
        unsafe extern "C" fn(
            env: ErlNifEnv,
            priv_data: *mut *mut c_void,
            old_priv_data: *mut *mut c_void,
            load_info: ERL_NIF_TERM,
        ) -> c_int,
    >,
    /// Called when the module is unloaded. Free any `priv_data` here.
    pub unload: Option<unsafe extern "C" fn(env: ErlNifEnv, priv_data: *mut c_void)>,
    /// Must be `"beam.vanilla\0"`. Identifies the BEAM variant.
    pub vm_variant: *const c_char,
    /// Options bitfield. 1 = delay BEAM halt until async threads finish.
    pub options: c_uint,
    /// Must be `std::mem::size_of::<ErlNifResourceTypeInit>()`.
    pub sizeof_ErlNifResourceTypeInit: usize,
    /// Minimum ERTS version required, e.g. `"erts-13.0\0"`.
    pub min_erts: *const c_char,
}

/// Opaque resource type handle returned by `enif_open_resource_type`.
///
/// Resource types are the BEAM equivalent of Python capsules or Node.js
/// external data — they let you wrap an arbitrary Rust heap object in an
/// Erlang term so the GC can manage its lifetime.
#[repr(C)]
pub struct ErlNifResourceType {
    _opaque: [u8; 0],
}

/// Flags for `enif_open_resource_type`.
pub type ErlNifResourceFlags = c_int;
/// Create the resource type if it does not exist.
pub const ERL_NIF_RT_CREATE: ErlNifResourceFlags = 1;
/// Take over an existing resource type from a previous module version.
pub const ERL_NIF_RT_TAKEOVER: ErlNifResourceFlags = 2;

/// String encoding constants for `enif_get_atom` / `enif_make_string`.
///
/// Latin-1 is the original encoding; UTF-8 is preferred for new code.
pub const ERL_NIF_LATIN1: c_int = 1;
pub const ERL_NIF_UTF8: c_int = 4;

/// A binary (byte string) value returned by `enif_inspect_binary` or
/// allocated by `enif_alloc_binary`.
///
/// `data` points to the binary's bytes; `size` is the byte count.
/// The `_priv` field is internal BEAM bookkeeping — never touch it.
#[repr(C)]
pub struct ErlNifBinary {
    /// Number of bytes in this binary.
    pub size: usize,
    /// Pointer to the raw bytes.
    pub data: *mut u8,
    /// Internal BEAM bookkeeping fields. Do not read or write these.
    pub _priv: [u8; 32],
}

// ---------------------------------------------------------------------------
// Version constants
// ---------------------------------------------------------------------------

/// Major version of the NIF API. Must always be 2.
pub const ERL_NIF_MAJOR_VERSION: c_int = 2;
/// Minor version for OTP 26+. Encodes feature availability.
pub const ERL_NIF_MINOR_VERSION: c_int = 16;

/// Scheduling flag: run this NIF on the dirty CPU scheduler thread pool.
/// Use for CPU-bound work that takes more than 1 ms (e.g. matrix multiply).
pub const ERL_NIF_DIRTY_JOB_CPU_BOUND: c_uint = 1;
/// Scheduling flag: run this NIF on the dirty I/O scheduler thread pool.
/// Use for blocking I/O operations (file reads, network calls, etc.).
pub const ERL_NIF_DIRTY_JOB_IO_BOUND: c_uint = 2;

// ---------------------------------------------------------------------------
// The erl_nif C API — extern "C" declarations
// ---------------------------------------------------------------------------
//
// These symbols are exported by the BEAM runtime (erts). The dynamic linker
// resolves them when our shared library is loaded via `:erlang.load_nif/2`.
// We declare them here so Rust knows the types without needing erl_nif.h.

#[allow(non_snake_case)]
extern "C" {
    // -- Integer conversions -----------------------------------------------
    //
    // Each `enif_get_*` function extracts a value from a term. Returns 1
    // on success (the term was the right type), 0 on failure. Writes the
    // result into the provided pointer.

    /// Extract a C int from an Erlang integer term.
    pub fn enif_get_int(env: ErlNifEnv, term: ERL_NIF_TERM, ip: *mut c_int) -> c_int;
    /// Extract a C uint from an Erlang integer term.
    pub fn enif_get_uint(env: ErlNifEnv, term: ERL_NIF_TERM, ip: *mut c_uint) -> c_int;
    /// Extract a C long from an Erlang integer term.
    pub fn enif_get_long(env: ErlNifEnv, term: ERL_NIF_TERM, ip: *mut c_long) -> c_int;
    /// Extract a double from an Erlang float term.
    pub fn enif_get_double(env: ErlNifEnv, term: ERL_NIF_TERM, dp: *mut f64) -> c_int;
    /// Create an Erlang integer term from a C int.
    pub fn enif_make_int(env: ErlNifEnv, i: c_int) -> ERL_NIF_TERM;
    /// Create an Erlang integer term from a C uint.
    pub fn enif_make_uint(env: ErlNifEnv, i: c_uint) -> ERL_NIF_TERM;
    /// Create an Erlang integer term from a C long.
    pub fn enif_make_long(env: ErlNifEnv, i: c_long) -> ERL_NIF_TERM;
    /// Create an Erlang float term from a double.
    pub fn enif_make_double(env: ErlNifEnv, d: f64) -> ERL_NIF_TERM;
    // NOTE: enif_make_int64 and enif_get_int64 are intentionally NOT declared here.
    //
    // In OTP 26 on Linux, these specific int64 variants may not be exported as
    // direct symbols from beam.smp (depending on how OTP was compiled). Using
    // them as extern "C" produces `undefined symbol: enif_get_int64` at dlopen()
    // time — the same static-inline problem seen in Python's PyLong_Check.
    //
    // Replacement: enif_get_long / enif_make_long are the original, always-exported
    // forms. On all 64-bit POSIX systems (Linux x86_64, macOS arm64/x86_64),
    // c_long is 64 bits, making these functionally identical to the int64 variants.

    // -- Atoms -------------------------------------------------------------
    //
    // Atoms are interned strings — the BEAM keeps a global atom table and
    // deduplicates them. Creating the same atom twice returns identical terms.
    // Atoms are the idiomatic way to encode symbols, status codes, and keys.

    /// Create an Erlang atom from a null-terminated C string.
    pub fn enif_make_atom(env: ErlNifEnv, name: *const c_char) -> ERL_NIF_TERM;
    /// Create an Erlang atom from a C string with an explicit byte length.
    pub fn enif_make_atom_len(
        env: ErlNifEnv,
        name: *const c_char,
        len: usize,
    ) -> ERL_NIF_TERM;
    /// Return 1 if `term` is an atom, 0 otherwise.
    pub fn enif_is_atom(env: ErlNifEnv, term: ERL_NIF_TERM) -> c_int;
    /// Copy the atom's name into `buf` (up to `size` bytes, including NUL).
    /// `encoding` should be `ERL_NIF_UTF8`. Returns the number of bytes written.
    pub fn enif_get_atom(
        env: ErlNifEnv,
        term: ERL_NIF_TERM,
        buf: *mut c_char,
        size: c_uint,
        encoding: c_int,
    ) -> c_int;
    /// Get the byte length of an atom's name.
    pub fn enif_get_atom_length(
        env: ErlNifEnv,
        term: ERL_NIF_TERM,
        len: *mut c_uint,
        encoding: c_int,
    ) -> c_int;

    // -- Strings / binaries ------------------------------------------------
    //
    // Erlang strings are either char-code lists or binaries. The binary
    // representation is far more efficient and is preferred in modern Erlang.
    // `enif_alloc_binary` allocates a mutable binary buffer; after filling it,
    // `enif_make_binary` transfers ownership to the GC.

    /// Create an Erlang char-list from a null-terminated C string.
    pub fn enif_make_string(
        env: ErlNifEnv,
        s: *const c_char,
        encoding: c_int,
    ) -> ERL_NIF_TERM;
    /// Create an Erlang char-list from a C string with explicit length.
    pub fn enif_make_string_len(
        env: ErlNifEnv,
        s: *const c_char,
        len: usize,
        encoding: c_int,
    ) -> ERL_NIF_TERM;
    /// Allocate a mutable binary buffer of `size` bytes.
    /// Returns 1 on success. Fill `bin.data[0..bin.size]` then call
    /// `enif_make_binary` to create the final immutable term.
    pub fn enif_alloc_binary(size: usize, bin: *mut ErlNifBinary) -> c_int;
    /// Transfer a binary buffer to the GC and return an immutable binary term.
    pub fn enif_make_binary(env: ErlNifEnv, bin: *mut ErlNifBinary) -> ERL_NIF_TERM;
    /// Inspect an existing binary term without copying. `bin.data` points
    /// directly into the BEAM heap — read-only.
    pub fn enif_inspect_binary(
        env: ErlNifEnv,
        term: ERL_NIF_TERM,
        bin: *mut ErlNifBinary,
    ) -> c_int;
    /// Release a binary allocated with `enif_alloc_binary` (if not passed
    /// to `enif_make_binary`).
    pub fn enif_release_binary(bin: *mut ErlNifBinary);
    /// Allocate a new binary term directly (combined alloc + make).
    /// Returns a pointer to the writable bytes; the term is written to `*termp`.
    pub fn enif_make_new_binary(
        env: ErlNifEnv,
        size: usize,
        termp: *mut ERL_NIF_TERM,
    ) -> *mut u8;

    // -- Lists -------------------------------------------------------------
    //
    // Erlang lists are singly-linked cons cells. The empty list is `[]`.
    // `enif_make_list` builds a list from varargs; prefer
    // `enif_make_list_from_array` for Rust code.

    /// Build a list from a variadic argument list.
    pub fn enif_make_list(env: ErlNifEnv, cnt: c_uint, ...) -> ERL_NIF_TERM;
    /// Prepend `head` to `tail` — equivalent to `[head | tail]` in Erlang.
    pub fn enif_make_list_cell(
        env: ErlNifEnv,
        head: ERL_NIF_TERM,
        tail: ERL_NIF_TERM,
    ) -> ERL_NIF_TERM;
    /// Build a list from a C array of terms. More convenient for Rust.
    pub fn enif_make_list_from_array(
        env: ErlNifEnv,
        arr: *const ERL_NIF_TERM,
        cnt: c_uint,
    ) -> ERL_NIF_TERM;
    /// Destructure a list: write `head` and `tail` into the provided pointers.
    /// Returns 1 on success (list was non-empty), 0 otherwise.
    pub fn enif_get_list_cell(
        env: ErlNifEnv,
        list: ERL_NIF_TERM,
        head: *mut ERL_NIF_TERM,
        tail: *mut ERL_NIF_TERM,
    ) -> c_int;
    /// Get the number of elements in a proper list.
    pub fn enif_get_list_length(
        env: ErlNifEnv,
        list: ERL_NIF_TERM,
        len: *mut c_uint,
    ) -> c_int;
    /// Return 1 if `term` is a list (including `[]`), 0 otherwise.
    pub fn enif_is_list(env: ErlNifEnv, term: ERL_NIF_TERM) -> c_int;

    // -- Tuples ------------------------------------------------------------
    //
    // Tuples are fixed-size heterogeneous collections. `{:ok, value}` and
    // `{:error, reason}` are the standard Elixir/Erlang result conventions.

    /// Build a tuple from varargs.
    pub fn enif_make_tuple(env: ErlNifEnv, cnt: c_uint, ...) -> ERL_NIF_TERM;
    /// Build a tuple from a C array of terms.
    pub fn enif_make_tuple_from_array(
        env: ErlNifEnv,
        arr: *const ERL_NIF_TERM,
        cnt: c_uint,
    ) -> ERL_NIF_TERM;
    /// Inspect a tuple: write its arity and element pointer. The element
    /// pointer `*array` points directly into the BEAM heap.
    pub fn enif_get_tuple(
        env: ErlNifEnv,
        tpl: ERL_NIF_TERM,
        arity: *mut c_int,
        array: *mut *const ERL_NIF_TERM,
    ) -> c_int;

    // -- Resources ---------------------------------------------------------
    //
    // Resources are the NIF equivalent of Python capsules or Node.js external
    // objects — they let you wrap an arbitrary Rust heap pointer in an Erlang
    // term so the GC manages its lifetime via your destructor.
    //
    // Lifecycle:
    //   1. In `load`: call `enif_open_resource_type` to register a type.
    //   2. To create: `enif_alloc_resource` → fill data → `enif_make_resource`.
    //   3. To read: `enif_get_resource` → cast pointer → read data.
    //   4. Destructor: BEAM calls your `dtor` when the term is GC'd.

    /// Register (or re-register on upgrade) a resource type with the BEAM.
    /// `dtor` is called when the last reference to any resource of this type
    /// is dropped.
    pub fn enif_open_resource_type(
        env: ErlNifEnv,
        module_str: *const c_char,
        name_str: *const c_char,
        dtor: Option<unsafe extern "C" fn(ErlNifEnv, *mut c_void)>,
        flags: ErlNifResourceFlags,
        tried: *mut ErlNifResourceFlags,
    ) -> *mut ErlNifResourceType;
    /// Allocate a resource object of `size` bytes. Returns a void pointer
    /// that you fill in, then pass to `enif_make_resource`.
    pub fn enif_alloc_resource(rtype: *mut ErlNifResourceType, size: usize) -> *mut c_void;
    /// Create an Erlang term that wraps the resource object.
    pub fn enif_make_resource(env: ErlNifEnv, obj: *mut c_void) -> ERL_NIF_TERM;
    /// Extract the raw pointer from a resource term.
    /// Returns 1 if the term is a resource of the given type, 0 otherwise.
    pub fn enif_get_resource(
        env: ErlNifEnv,
        term: ERL_NIF_TERM,
        rtype: *mut ErlNifResourceType,
        objp: *mut *mut c_void,
    ) -> c_int;
    /// Decrement the reference count of a resource object.
    pub fn enif_release_resource(obj: *mut c_void);
    /// Increment the reference count (keep the resource alive past GC).
    pub fn enif_keep_resource(obj: *mut c_void);

    // -- Error / exception -------------------------------------------------

    /// Raise a `badarg` exception — the standard Erlang error for bad input.
    /// The return value is a sentinel term that should be returned immediately
    /// from the NIF. (The actual exception is recorded in the env.)
    pub fn enif_make_badarg(env: ErlNifEnv) -> ERL_NIF_TERM;
    /// Raise an arbitrary exception term. Return the result from the NIF.
    pub fn enif_raise_exception(env: ErlNifEnv, reason: ERL_NIF_TERM) -> ERL_NIF_TERM;

    // -- Type checks -------------------------------------------------------

    /// Return 1 if `term` is an integer or float.
    pub fn enif_is_number(env: ErlNifEnv, term: ERL_NIF_TERM) -> c_int;
    /// Return 1 if `term` is a binary.
    pub fn enif_is_binary(env: ErlNifEnv, term: ERL_NIF_TERM) -> c_int;
    /// Return 1 if `term` is a tuple.
    pub fn enif_is_tuple(env: ErlNifEnv, term: ERL_NIF_TERM) -> c_int;
    /// Return 1 if `term` is a map.
    pub fn enif_is_map(env: ErlNifEnv, term: ERL_NIF_TERM) -> c_int;

    // -- Maps --------------------------------------------------------------
    //
    // Erlang maps are %{key => value} hashes — the idiomatic structured-data
    // type. Building/inspecting them from a NIF requires four operations:
    // create empty, put a key, look up a key, iterate all entries.

    /// Create an empty map term `%{}`.
    pub fn enif_make_new_map(env: ErlNifEnv) -> ERL_NIF_TERM;
    /// Functionally insert `(key, value)` into `map_in`, writing the new
    /// map term into `*map_out`. Returns 1 on success, 0 if the key was
    /// already present (use `enif_make_map_update` to overwrite).
    pub fn enif_make_map_put(
        env: ErlNifEnv,
        map_in: ERL_NIF_TERM,
        key: ERL_NIF_TERM,
        value: ERL_NIF_TERM,
        map_out: *mut ERL_NIF_TERM,
    ) -> c_int;
    /// Look up `key` in `map`. Writes the value into `*value` and returns 1
    /// if found, 0 otherwise.
    pub fn enif_get_map_value(
        env: ErlNifEnv,
        map: ERL_NIF_TERM,
        key: ERL_NIF_TERM,
        value: *mut ERL_NIF_TERM,
    ) -> c_int;
    /// Get the number of entries in a map.
    pub fn enif_get_map_size(
        env: ErlNifEnv,
        map: ERL_NIF_TERM,
        size: *mut usize,
    ) -> c_int;

    // -- Map iterators -----------------------------------------------------
    //
    // For walking every key/value pair. The iterator state is opaque to us;
    // BEAM defines it as a small struct, so we reserve enough bytes.

    /// Initialize iterator `iter` over `map` starting at the first entry
    /// (`entry = ERL_NIF_MAP_ITERATOR_FIRST = 1`). Returns 1 on success.
    pub fn enif_map_iterator_create(
        env: ErlNifEnv,
        map: ERL_NIF_TERM,
        iter: *mut ErlNifMapIterator,
        entry: c_int,
    ) -> c_int;
    /// Release any resources held by the iterator.
    pub fn enif_map_iterator_destroy(env: ErlNifEnv, iter: *mut ErlNifMapIterator);
    /// Advance the iterator. Returns 1 if a next entry exists, 0 at end.
    pub fn enif_map_iterator_next(env: ErlNifEnv, iter: *mut ErlNifMapIterator) -> c_int;
    /// Read the current pair into `*key` and `*value`. Returns 1 if
    /// the iterator is at a valid entry, 0 if past the end.
    pub fn enif_map_iterator_get_pair(
        env: ErlNifEnv,
        iter: *mut ErlNifMapIterator,
        key: *mut ERL_NIF_TERM,
        value: *mut ERL_NIF_TERM,
    ) -> c_int;

    // -- Pids and process messaging ----------------------------------------
    //
    // A pid identifies a BEAM process. From a NIF we can:
    //  - get the pid of the current process via `enif_self`
    //  - extract a pid from a term via `enif_get_local_pid`
    //  - send a message to a pid via `enif_send`
    //
    // `enif_send(NULL, to_pid, msg_env, msg)` is the off-scheduler-thread
    // form: it copies the term out of `msg_env` and into the recipient's
    // mailbox. This is the BEAM analog of N-API's threadsafe function call
    // and is exactly what we need from a Rust I/O thread.

    /// Write the calling process's pid into `*pid`. Returns its address
    /// on success, NULL if called from a thread that has no current process
    /// (e.g. a dirty NIF can call this; a pure background thread cannot).
    pub fn enif_self(env: ErlNifEnv, pid: *mut ErlNifPid) -> *mut ErlNifPid;
    /// Extract a local pid from a term. Returns 1 on success.
    pub fn enif_get_local_pid(
        env: ErlNifEnv,
        term: ERL_NIF_TERM,
        pid: *mut ErlNifPid,
    ) -> c_int;
    /// Build a term from a pid (for sending pids back to Elixir code).
    pub fn enif_make_pid(env: ErlNifEnv, pid: *const ErlNifPid) -> ERL_NIF_TERM;
    /// Send `msg` (allocated in `msg_env`) to `to_pid`. Returns 1 on
    /// success, 0 if the destination process is dead.
    ///
    /// Pass `caller_env = NULL` from a non-scheduler thread (e.g. Rust
    /// background I/O thread). In that case `msg_env` MUST be a long-lived
    /// env created with `enif_alloc_env` — its terms are *consumed* by
    /// the send, so the env must be freed with `enif_free_env` afterwards.
    pub fn enif_send(
        caller_env: ErlNifEnv,
        to_pid: *const ErlNifPid,
        msg_env: ErlNifEnv,
        msg: ERL_NIF_TERM,
    ) -> c_int;

    // -- Environment management --------------------------------------------

    /// Allocate a new independent environment (for async/message-passing use).
    /// Terms allocated in this env persist until `enif_free_env`.
    pub fn enif_alloc_env() -> ErlNifEnv;
    /// Free an environment allocated with `enif_alloc_env`.
    pub fn enif_free_env(env: ErlNifEnv);

    // -- Memory ------------------------------------------------------------

    /// Allocate raw memory from BEAM's allocator. Must be freed with `enif_free`.
    pub fn enif_alloc(size: usize) -> *mut c_void;
    /// Free memory allocated by `enif_alloc`.
    pub fn enif_free(ptr: *mut c_void);
}

// ---------------------------------------------------------------------------
// Map iterator state
// ---------------------------------------------------------------------------
//
// BEAM defines `ErlNifMapIterator` as an opaque struct; the actual layout is:
//
//     struct enif_map_iterator_t {
//         ERL_NIF_TERM map;
//         ErlNifUInt size;
//         ErlNifUInt idx;
//         union { ... } u;
//     };
//
// On 64-bit platforms the size is 5 machine words; we reserve 8 for safety.
// We never read the fields ourselves — only BEAM does.

/// Opaque map iterator. Pass a stack-allocated instance to
/// `enif_map_iterator_create` then `enif_map_iterator_destroy` when done.
///
/// The internal layout is reserved by BEAM (5 machine words on 64-bit
/// platforms; we round up to 8 for safety). Construct one via
/// `ErlNifMapIterator::zeroed()` — never read or write the bytes
/// directly.
#[repr(C)]
pub struct ErlNifMapIterator {
    // Public so callers can zero-initialize without going through a method,
    // but the field name is reserved and must never be read or written
    // directly by user code.
    pub __private_internal_state: [usize; 8],
}

impl ErlNifMapIterator {
    /// Construct a zero-initialized iterator suitable for passing to
    /// `enif_map_iterator_create`. The BEAM populates the internal state
    /// inside that call.
    pub const fn zeroed() -> Self {
        Self { __private_internal_state: [0usize; 8] }
    }
}

/// `entry` argument to `enif_map_iterator_create`: start at the first key.
pub const ERL_NIF_MAP_ITERATOR_FIRST: c_int = 1;

// ---------------------------------------------------------------------------
// Pid type
// ---------------------------------------------------------------------------

/// A BEAM process identifier. Layout-compatible with C `ErlNifPid`.
///
/// The single field is itself an `ERL_NIF_TERM` that the BEAM treats
/// specially. Never construct one by hand — only use `enif_self`,
/// `enif_get_local_pid`, or copy from another `ErlNifPid` you already have.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ErlNifPid {
    pub pid: ERL_NIF_TERM,
}

// ---------------------------------------------------------------------------
// Safe helper functions — the "bridge" layer
// ---------------------------------------------------------------------------
//
// These are the safe Rust wrappers that extension authors actually call.
// They handle NUL-termination, null checks, and the conversion dance between
// Rust types and BEAM terms.

/// Convert a Rust `&str` to an Erlang atom term.
///
/// Equivalent to `:my_atom` in Elixir or `my_atom` in Erlang.
/// Atoms are interned globally; the same string always produces the same term.
///
/// C equivalent: `enif_make_atom(env, "my_atom")`
pub unsafe fn atom(env: ErlNifEnv, s: &str) -> ERL_NIF_TERM {
    enif_make_atom_len(env, s.as_ptr() as *const c_char, s.len())
}

/// Convert a Rust `i64` to an Erlang integer term.
///
/// Uses `enif_make_long` (always-exported) rather than `enif_make_int64`
/// (not reliably exported in all OTP 26 beam.smp builds on Linux).
/// On 64-bit POSIX platforms c_long == i64, so the behavior is identical.
pub unsafe fn make_i64(env: ErlNifEnv, i: i64) -> ERL_NIF_TERM {
    enif_make_long(env, i as c_long)
}

/// Convert a Rust `f64` to an Erlang float term.
///
/// C equivalent: `enif_make_double(env, d)`
pub unsafe fn make_f64(env: ErlNifEnv, d: f64) -> ERL_NIF_TERM {
    enif_make_double(env, d)
}

/// Try to extract an `i64` from an Erlang integer term.
///
/// Returns `None` if the term is not an integer.
/// Uses `enif_get_long` (always-exported) rather than `enif_get_int64`.
/// On 64-bit POSIX platforms c_long == i64, so the behavior is identical.
pub unsafe fn get_i64(env: ErlNifEnv, term: ERL_NIF_TERM) -> Option<i64> {
    let mut val: c_long = 0;
    if enif_get_long(env, term, &mut val) != 0 {
        Some(val as i64)
    } else {
        None
    }
}

/// Try to extract an `f64` from an Erlang float or integer term.
///
/// Returns `None` if the term is neither a float nor an integer.
pub unsafe fn get_f64(env: ErlNifEnv, term: ERL_NIF_TERM) -> Option<f64> {
    let mut d: f64 = 0.0;
    if enif_get_double(env, term, &mut d) != 0 {
        return Some(d);
    }
    // Erlang integers can also be coerced to float.
    // Use enif_get_long (always exported) instead of enif_get_int64.
    let mut i: c_long = 0;
    if enif_get_long(env, term, &mut i) != 0 {
        return Some(i as f64);
    }
    None
}

/// Convert a Rust `&[ERL_NIF_TERM]` to an Erlang list.
///
/// C equivalent: `enif_make_list_from_array(env, arr.as_ptr(), arr.len())`
pub unsafe fn make_list(env: ErlNifEnv, terms: &[ERL_NIF_TERM]) -> ERL_NIF_TERM {
    enif_make_list_from_array(env, terms.as_ptr(), terms.len() as c_uint)
}

/// Convert a Rust `&[f64]` to an Erlang list of floats.
pub unsafe fn make_f64_list(env: ErlNifEnv, values: &[f64]) -> ERL_NIF_TERM {
    let terms: Vec<ERL_NIF_TERM> = values.iter().map(|&d| enif_make_double(env, d)).collect();
    enif_make_list_from_array(env, terms.as_ptr(), terms.len() as c_uint)
}

/// Convert an Erlang list of floats/ints to a Rust `Vec<f64>`.
///
/// Returns `None` if any element is not numeric.
/// Iterates via `enif_get_list_cell` — O(n) allocation-free traversal.
pub unsafe fn get_f64_list(env: ErlNifEnv, list: ERL_NIF_TERM) -> Option<Vec<f64>> {
    let mut len: c_uint = 0;
    if enif_get_list_length(env, list, &mut len) == 0 {
        return None;
    }
    let mut result = Vec::with_capacity(len as usize);
    let mut current = list;
    loop {
        let mut head: ERL_NIF_TERM = 0;
        let mut tail: ERL_NIF_TERM = 0;
        if enif_get_list_cell(env, current, &mut head, &mut tail) == 0 {
            break; // empty list
        }
        let d = get_f64(env, head)?;
        result.push(d);
        current = tail;
    }
    Some(result)
}

/// Wrap a Rust heap value in an Erlang resource term.
///
/// This is the BEAM equivalent of Python's capsules or Node.js's `napi_wrap`.
/// The value is heap-allocated, its pointer stored in a BEAM resource object,
/// and the BEAM GC will call your destructor when the term is collected.
///
/// Safety: `rtype` must have been opened with a destructor that calls
/// `Box::from_raw::<T>()`. The type `T` must match what was used to open
/// the resource type.
pub unsafe fn wrap_resource<T>(
    env: ErlNifEnv,
    rtype: *mut ErlNifResourceType,
    val: T,
) -> ERL_NIF_TERM {
    let size = std::mem::size_of::<T>();
    let ptr = enif_alloc_resource(rtype, size) as *mut T;
    ptr::write(ptr, val);
    let term = enif_make_resource(env, ptr as *mut c_void);
    enif_release_resource(ptr as *mut c_void); // transfer ownership to BEAM
    term
}

/// Unwrap a Rust heap value from an Erlang resource term.
///
/// Returns `None` if the term is not a resource of `rtype`.
/// The returned pointer is valid as long as the BEAM term is alive.
pub unsafe fn unwrap_resource<T>(
    env: ErlNifEnv,
    term: ERL_NIF_TERM,
    rtype: *mut ErlNifResourceType,
) -> Option<*mut T> {
    let mut ptr: *mut c_void = ptr::null_mut();
    if enif_get_resource(env, term, rtype, &mut ptr) != 0 {
        Some(ptr as *mut T)
    } else {
        None
    }
}

/// Raise a `badarg` exception and return the sentinel term.
///
/// Call this when an argument has the wrong type or an invalid value.
/// Always return the result of this function directly from your NIF:
///
/// ```rust,ignore
/// return badarg(env);
/// ```
///
/// C equivalent: `enif_make_badarg(env)`
pub unsafe fn badarg(env: ErlNifEnv) -> ERL_NIF_TERM {
    enif_make_badarg(env)
}

/// Build an `{:ok, val}` tuple — the standard Erlang/Elixir success result.
///
/// C equivalent: `enif_make_tuple2(env, enif_make_atom(env, "ok"), val)`
pub unsafe fn ok_tuple(env: ErlNifEnv, val: ERL_NIF_TERM) -> ERL_NIF_TERM {
    let ok = atom(env, "ok");
    let arr = [ok, val];
    enif_make_tuple_from_array(env, arr.as_ptr(), 2)
}

/// Build an `{:error, reason}` tuple — the standard Erlang/Elixir error result.
///
/// `reason` becomes an atom, e.g. `{:error, :invalid_input}`.
/// C equivalent: `enif_make_tuple2(env, enif_make_atom(env, "error"), enif_make_atom(env, reason))`
pub unsafe fn error_tuple(env: ErlNifEnv, reason: &str) -> ERL_NIF_TERM {
    let error = atom(env, "error");
    let reason_atom = atom(env, reason);
    let arr = [error, reason_atom];
    enif_make_tuple_from_array(env, arr.as_ptr(), 2)
}

// ---------------------------------------------------------------------------
// Safe wrappers — Maps
// ---------------------------------------------------------------------------
//
// Building an Elixir map from Rust normally goes:
//
//     let m = enif_make_new_map(env);          // %{}
//     let mut m = m;
//     enif_make_map_put(env, m, k1, v1, &mut m);  // %{k1 => v1}
//     enif_make_map_put(env, m, k2, v2, &mut m);  // %{k1 => v1, k2 => v2}
//
// Maps are immutable; each `_put` returns a new term. The wrapper below
// hides the boilerplate.

/// Create an empty Elixir map term `%{}`.
pub unsafe fn make_map(env: ErlNifEnv) -> ERL_NIF_TERM {
    enif_make_new_map(env)
}

/// Functionally insert `(key, value)` into `map`, returning the new map.
///
/// Returns the input map unchanged if the BEAM rejects the put (which
/// happens only when the key already exists — call this on a fresh map
/// or use `map_update` for overwrites).
pub unsafe fn map_put(
    env: ErlNifEnv,
    map: ERL_NIF_TERM,
    key: ERL_NIF_TERM,
    value: ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let mut out: ERL_NIF_TERM = 0;
    if enif_make_map_put(env, map, key, value, &mut out) != 0 {
        out
    } else {
        map
    }
}

/// Build a map from a slice of `(key, value)` pairs.
///
/// Convenient for assembling the env map term from a Rust HashMap:
///
/// ```rust,ignore
/// let pairs: Vec<(ERL_NIF_TERM, ERL_NIF_TERM)> = my_hashmap.iter()
///     .map(|(k, v)| (str_to_binary(env, k), str_to_binary(env, v)))
///     .collect();
/// let m = make_map_from_pairs(env, &pairs);
/// ```
pub unsafe fn make_map_from_pairs(
    env: ErlNifEnv,
    pairs: &[(ERL_NIF_TERM, ERL_NIF_TERM)],
) -> ERL_NIF_TERM {
    let mut m = enif_make_new_map(env);
    for &(k, v) in pairs {
        m = map_put(env, m, k, v);
    }
    m
}

/// Look up `key` in `map`. Returns `None` if absent.
pub unsafe fn map_get(
    env: ErlNifEnv,
    map: ERL_NIF_TERM,
    key: ERL_NIF_TERM,
) -> Option<ERL_NIF_TERM> {
    let mut value: ERL_NIF_TERM = 0;
    if enif_get_map_value(env, map, key, &mut value) != 0 {
        Some(value)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Safe wrappers — Binaries (string ↔ binary conversion)
// ---------------------------------------------------------------------------
//
// In Elixir, binaries (`<<…>>`) are the standard string representation —
// not char-lists. NIFs that exchange strings with Elixir code should always
// use binaries: faster, no per-character cons cells, and printable as text
// when the bytes are valid UTF-8.

/// Build an Erlang binary term from a Rust `&str`.
///
/// Allocates a fresh binary, copies the bytes, and returns the term.
/// On Elixir side this becomes a regular UTF-8 string.
pub unsafe fn str_to_binary(env: ErlNifEnv, s: &str) -> ERL_NIF_TERM {
    let mut term: ERL_NIF_TERM = 0;
    let buf = enif_make_new_binary(env, s.len(), &mut term);
    if !buf.is_null() && !s.is_empty() {
        std::ptr::copy_nonoverlapping(s.as_ptr(), buf, s.len());
    }
    term
}

/// Extract the bytes of a binary term as a borrowed `&[u8]`.
///
/// Returns `None` if `term` is not a binary. The slice is valid only for
/// the lifetime of the calling NIF — copy out anything you need to keep.
pub unsafe fn binary_to_bytes<'a>(env: ErlNifEnv, term: ERL_NIF_TERM) -> Option<&'a [u8]> {
    let mut bin = ErlNifBinary {
        size: 0,
        data: ptr::null_mut(),
        _priv: [0u8; 32],
    };
    if enif_inspect_binary(env, term, &mut bin) == 0 {
        return None;
    }
    Some(std::slice::from_raw_parts(bin.data, bin.size))
}

/// Extract a binary term as a Rust `String` (UTF-8 lossy).
///
/// Returns `None` if `term` is not a binary. Lossy conversion replaces
/// invalid UTF-8 byte sequences with `U+FFFD REPLACEMENT CHARACTER`.
pub unsafe fn binary_to_string(env: ErlNifEnv, term: ERL_NIF_TERM) -> Option<String> {
    let bytes = binary_to_bytes(env, term)?;
    Some(String::from_utf8_lossy(bytes).into_owned())
}

// ---------------------------------------------------------------------------
// Safe wrappers — Pids and send
// ---------------------------------------------------------------------------

/// Get the calling process's pid.
///
/// Returns `None` only when called from a context with no current process
/// (e.g. a pure background thread that has never been associated with a
/// scheduler). From a regular NIF or dirty NIF, this always succeeds.
pub unsafe fn self_pid(env: ErlNifEnv) -> Option<ErlNifPid> {
    let mut pid = ErlNifPid { pid: 0 };
    let ret = enif_self(env, &mut pid);
    if ret.is_null() {
        None
    } else {
        Some(pid)
    }
}

/// Extract a pid from a term passed by Elixir.
pub unsafe fn get_pid(env: ErlNifEnv, term: ERL_NIF_TERM) -> Option<ErlNifPid> {
    let mut pid = ErlNifPid { pid: 0 };
    if enif_get_local_pid(env, term, &mut pid) != 0 {
        Some(pid)
    } else {
        None
    }
}

/// Send `msg` (built in `msg_env`) to `to_pid` from a non-scheduler thread.
///
/// `msg_env` MUST be a long-lived env created with `enif_alloc_env`.
/// The send transfers ownership of the env's terms; the env itself must
/// be freed by the caller with `enif_free_env` afterwards.
///
/// Returns `true` if the message was queued, `false` if the destination
/// process is dead.
pub unsafe fn send_from_thread(
    to_pid: &ErlNifPid,
    msg_env: ErlNifEnv,
    msg: ERL_NIF_TERM,
) -> bool {
    enif_send(ptr::null_mut(), to_pid, msg_env, msg) != 0
}

// ---------------------------------------------------------------------------
// nif_init! — the module entry point macro
// ---------------------------------------------------------------------------
//
// Every NIF library must export a function named `nif_init` that returns
// a pointer to an `ErlNifEntry`. The BEAM calls this during `:erlang.load_nif/2`.
//
// In C, this is done via the `ERL_NIF_INIT` macro. We replicate it here.
//
// Usage:
//
// ```rust,ignore
// nif_init!("my_module", [
//     ErlNifFunc { name: b"add\0".as_ptr() as *const _, arity: 2, fptr: nif_add, flags: 0 },
// ]);
// ```

/// Define the NIF library entry point.
///
/// Generates the `nif_init()` function that the BEAM calls when loading
/// your NIF library. Provide the Erlang module name and a list of `ErlNifFunc`
/// descriptors.
///
/// The `$module` argument must be a string literal like `"my_module"`.
/// The `$funcs` argument must be an expression of type `&'static [ErlNifFunc]`.
///
/// # Example
///
/// ```rust,ignore
/// static FUNCS: &[ErlNifFunc] = &[
///     ErlNifFunc {
///         name: b"add\0".as_ptr() as *const _,
///         arity: 2,
///         fptr: nif_add,
///         flags: 0,
///     },
/// ];
///
/// nif_init!("my_math", FUNCS);
/// ```
#[macro_export]
macro_rules! nif_init {
    ($module:expr, $funcs:expr) => {
        /// The NIF library entry point. Called by the BEAM when loading this NIF.
        ///
        /// # Safety
        /// The returned pointer must remain valid for the lifetime of the process.
        #[no_mangle]
        pub unsafe extern "C" fn nif_init() -> *const $crate::ErlNifEntry {
            static VM_VARIANT: &[u8] = b"beam.vanilla\0";
            static MIN_ERTS: &[u8] = b"erts-13.0\0";

            static MODULE_NAME: &[u8] = concat!($module, "\0").as_bytes();

            static ENTRY: $crate::ErlNifEntry = $crate::ErlNifEntry {
                major: $crate::ERL_NIF_MAJOR_VERSION,
                minor: $crate::ERL_NIF_MINOR_VERSION,
                name: MODULE_NAME.as_ptr() as *const ::std::ffi::c_char,
                num_of_funcs: $funcs.len() as ::std::ffi::c_int,
                funcs: $funcs.as_ptr(),
                load: None,
                reload: None,
                upgrade: None,
                unload: None,
                vm_variant: VM_VARIANT.as_ptr() as *const ::std::ffi::c_char,
                options: 0,
                sizeof_ErlNifResourceTypeInit: 0,
                min_erts: MIN_ERTS.as_ptr() as *const ::std::ffi::c_char,
            };

            &ENTRY
        }
    };
}
