// An Erlang NIF library built only to be loaded into a real BEAM by
// tests/smoke.escript.
//
// Why this exists
// ---------------
// erl-nif-bridge is a library of `extern "C"` declarations (and, on Windows, a
// table of function pointers).  A compiler cannot tell whether a declaration names
// a symbol the BEAM really exports, whether a constant matches `erl_nif.h`, or
// whether a pointer-sized type is right: all of those compile.  `enif_make_pid`
// was declared as an exported function and is a header macro, so a NIF that called
// it failed to load; only loading one into the BEAM shows that.  This library
// calls every function the bridge wraps and its resource, map, pid and message
// paths, and tests/smoke.escript checks the results from the Erlang side.
//
// Loading it
// ----------
// tests/nif_smoke.erl declares a stub for each function below and calls
// `erlang:load_nif/2`; the BEAM calls `nif_init`, reads the `ErlNifEntry`, and
// replaces each stub with the native function of the same name and arity.  The
// module name in the entry ("nif_smoke") must match the Erlang module.
//
// The entry is written out by hand rather than with `nif_init!` because it needs
// a `load` callback: the resource type must be registered while the library loads.

#![allow(non_snake_case)]

use erl_nif_bridge::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

/// The registered resource type, set once by `load`.
static COUNTER_TYPE: AtomicPtr<ErlNifResourceType> = AtomicPtr::new(ptr::null_mut());

/// How many Counter resources the BEAM has destroyed.
static DESTROYED: AtomicUsize = AtomicUsize::new(0);

/// The payload of a Counter resource: a Rust value whose lifetime the BEAM's
/// garbage collector manages.
struct Counter {
    value: i64,
}

/// The pid a spawned thread sends to.  `ErlNifPid` is plain data (one term), but
/// a raw `ERL_NIF_TERM` is not `Send` for the compiler's purposes, so it travels
/// as a `usize`.
struct SendPid(usize);

// -- integers and floats ----------------------------------------------------

/// `add(A, B)` -> `A + B` for 64-bit integers (`get_i64` / `make_i64`; on
/// Windows these use the `int64` table entries because C `long` is 32 bits).
unsafe extern "C" fn nif_add(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    match (get_i64(env, *argv), get_i64(env, *argv.add(1))) {
        (Some(a), Some(b)) => make_i64(env, a.wrapping_add(b)),
        _ => badarg(env),
    }
}

/// `int64_extremes()` -> `{Min, Max}`: values that need all 64 bits.
unsafe extern "C" fn nif_int64_extremes(
    env: ErlNifEnv,
    _argc: c_int,
    _argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let pair = [make_i64(env, i64::MIN), make_i64(env, i64::MAX)];
    enif_make_tuple_from_array(env, pair.as_ptr(), 2)
}

/// `half(X)` -> `X / 2` as a float, for an integer or a float `X` (`get_f64`
/// falls back to an integer; `make_f64`).
unsafe extern "C" fn nif_half(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    match get_f64(env, *argv) {
        Some(x) => make_f64(env, x / 2.0),
        None => badarg(env),
    }
}

// -- atoms, tuples, results -------------------------------------------------

/// `kind(Term)` -> the name of `Term`'s type, from the `enif_is_*` predicates.
unsafe extern "C" fn nif_kind(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let t = *argv;
    let name = if enif_is_atom(env, t) != 0 {
        "atom"
    } else if enif_is_binary(env, t) != 0 {
        "binary"
    } else if enif_is_map(env, t) != 0 {
        "map"
    } else if enif_is_tuple(env, t) != 0 {
        "tuple"
    } else if enif_is_list(env, t) != 0 {
        "list"
    } else if enif_is_number(env, t) != 0 {
        "number"
    } else {
        "other"
    };
    atom(env, name)
}

/// `atom_length(Atom)` -> its length in bytes (`enif_get_atom_length`), or badarg.
unsafe extern "C" fn nif_atom_length(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let mut len: u32 = 0;
    if enif_get_atom_length(env, *argv, &mut len, ERL_NIF_LATIN1) == 0 {
        return badarg(env);
    }
    make_i64(env, len as i64)
}

/// `swap({A, B})` -> `{B, A}` (`enif_get_tuple`, `enif_make_tuple_from_array`).
unsafe extern "C" fn nif_swap(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let mut arity: c_int = 0;
    let mut items: *const ERL_NIF_TERM = ptr::null();
    if enif_get_tuple(env, *argv, &mut arity, &mut items) == 0 || arity != 2 {
        return badarg(env);
    }
    let swapped = [*items.add(1), *items];
    enif_make_tuple_from_array(env, swapped.as_ptr(), 2)
}

/// `result(Good)` -> `{ok, 1}` or `{error, nope}` (`ok_tuple`, `error_tuple`).
unsafe extern "C" fn nif_result(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let mut good = [0u8; 8];
    let mut len: u32 = 0;
    if enif_get_atom_length(env, *argv, &mut len, ERL_NIF_LATIN1) == 0 || len > 7 {
        return badarg(env);
    }
    if enif_get_atom(
        env,
        *argv,
        good.as_mut_ptr() as *mut c_char,
        8,
        ERL_NIF_LATIN1,
    ) == 0
    {
        return badarg(env);
    }
    if &good[..len as usize] == b"true" {
        ok_tuple(env, make_i64(env, 1))
    } else {
        error_tuple(env, "nope")
    }
}

/// `variadic_tuple()` -> `{1, two}` through the variadic `enif_make_tuple`.
/// Windows has no variadic entries in the callback table, so the function does
/// not exist there (see the registration below).
#[cfg(not(windows))]
unsafe extern "C" fn nif_variadic_tuple(
    env: ErlNifEnv,
    _argc: c_int,
    _argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    enif_make_tuple(env, 2, make_i64(env, 1), atom(env, "two"))
}

// -- binaries ---------------------------------------------------------------

/// `upcase(Bin)` -> the binary in upper case (`binary_to_string`, `str_to_binary`).
unsafe extern "C" fn nif_upcase(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    match binary_to_string(env, *argv) {
        Some(s) => str_to_binary(env, &s.to_uppercase()),
        None => badarg(env),
    }
}

/// `checksum(Bin)` -> the sum of the bytes (`binary_to_bytes`, byte-exact for
/// embedded zeros).
unsafe extern "C" fn nif_checksum(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    match binary_to_bytes(env, *argv) {
        Some(bytes) => make_i64(env, bytes.iter().map(|&b| b as i64).sum()),
        None => badarg(env),
    }
}

/// `alloc_roundtrip()` -> `true` when a block from `enif_alloc` keeps what is
/// written to it before `enif_free`.
unsafe extern "C" fn nif_alloc_roundtrip(
    env: ErlNifEnv,
    _argc: c_int,
    _argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let block = enif_alloc(64) as *mut u8;
    if block.is_null() {
        return badarg(env);
    }
    for i in 0..64 {
        *block.add(i) = i as u8;
    }
    let intact = (0..64).all(|i| *block.add(i) == i as u8);
    enif_free(block as *mut c_void);
    atom(env, if intact { "true" } else { "false" })
}

// -- lists ------------------------------------------------------------------

/// `sum_floats(List)` -> the sum of a list of numbers (`get_f64_list`), or badarg.
unsafe extern "C" fn nif_sum_floats(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    match get_f64_list(env, *argv) {
        // `0.0 +`: an empty float sum is -0.0, and OTP 27 tells -0.0 from 0.0.
        Some(values) => make_f64(env, 0.0 + values.iter().sum::<f64>()),
        None => badarg(env),
    }
}

/// `range_floats(N)` -> `[1.0, ..., N]` (`make_f64_list`).
unsafe extern "C" fn nif_range_floats(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    match get_i64(env, *argv) {
        Some(n) if n >= 0 => {
            let values: Vec<f64> = (1..=n).map(|i| i as f64).collect();
            make_f64_list(env, &values)
        }
        _ => badarg(env),
    }
}

/// `list_of_atoms()` -> `[a, b, c]` (`make_list`).
unsafe extern "C" fn nif_list_of_atoms(
    env: ErlNifEnv,
    _argc: c_int,
    _argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    make_list(env, &[atom(env, "a"), atom(env, "b"), atom(env, "c")])
}

// -- maps -------------------------------------------------------------------

/// `make_map()` -> `#{a => 1, b => 2}` (`make_map_from_pairs`).
unsafe extern "C" fn nif_make_map(
    env: ErlNifEnv,
    _argc: c_int,
    _argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    make_map_from_pairs(
        env,
        &[
            (atom(env, "a"), make_i64(env, 1)),
            (atom(env, "b"), make_i64(env, 2)),
        ],
    )
}

/// `map_put(Map, Key, Value)` -> the updated map (`map_put`).
unsafe extern "C" fn nif_map_put(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    map_put(env, *argv, *argv.add(1), *argv.add(2))
}

/// `map_lookup(Map, Key)` -> `{ok, Value}` or `error` (`map_get`).
unsafe extern "C" fn nif_map_lookup(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    match map_get(env, *argv, *argv.add(1)) {
        Some(value) => ok_tuple(env, value),
        None => atom(env, "error"),
    }
}

/// `map_size(Map)` -> the number of entries (`enif_get_map_size`).
unsafe extern "C" fn nif_map_size(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let mut size: usize = 0;
    if enif_get_map_size(env, *argv, &mut size) == 0 {
        return badarg(env);
    }
    make_i64(env, size as i64)
}

/// `map_sum(Map)` -> the sum of the integer values, walking the map with the
/// iterator functions.
unsafe extern "C" fn nif_map_sum(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let mut iter = ErlNifMapIterator::zeroed();
    if enif_map_iterator_create(env, *argv, &mut iter, ERL_NIF_MAP_ITERATOR_FIRST) == 0 {
        return badarg(env);
    }
    let mut total: i64 = 0;
    loop {
        let mut key: ERL_NIF_TERM = 0;
        let mut value: ERL_NIF_TERM = 0;
        if enif_map_iterator_get_pair(env, &mut iter, &mut key, &mut value) == 0 {
            break;
        }
        total += get_i64(env, value).unwrap_or(0);
        enif_map_iterator_next(env, &mut iter);
    }
    enif_map_iterator_destroy(env, &mut iter);
    make_i64(env, total)
}

// -- resources --------------------------------------------------------------

/// The destructor the BEAM runs when the last reference to a Counter dies.
unsafe extern "C" fn counter_dtor(_env: ErlNifEnv, obj: *mut c_void) {
    ptr::drop_in_place(obj as *mut Counter);
    DESTROYED.fetch_add(1, Ordering::SeqCst);
}

/// `counter_new(Start)` -> a resource reference holding a Rust `Counter`
/// (`wrap_resource`).
unsafe extern "C" fn nif_counter_new(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    match get_i64(env, *argv) {
        Some(start) => wrap_resource(
            env,
            COUNTER_TYPE.load(Ordering::SeqCst),
            Counter { value: start },
        ),
        None => badarg(env),
    }
}

/// `counter_bump(Counter)` -> the new value (`unwrap_resource`); badarg when the
/// argument is not a Counter.
unsafe extern "C" fn nif_counter_bump(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    match unwrap_resource::<Counter>(env, *argv, COUNTER_TYPE.load(Ordering::SeqCst)) {
        Some(counter) => {
            (*counter).value += 1;
            make_i64(env, (*counter).value)
        }
        None => badarg(env),
    }
}

/// `counters_destroyed()` -> how many Counters the BEAM has destroyed so far.
unsafe extern "C" fn nif_counters_destroyed(
    env: ErlNifEnv,
    _argc: c_int,
    _argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    make_i64(env, DESTROYED.load(Ordering::SeqCst) as i64)
}

// -- processes and messages -------------------------------------------------

/// `self_pid()` -> the calling process's pid (`self_pid`, `enif_make_pid`).
unsafe extern "C" fn nif_self_pid(
    env: ErlNifEnv,
    _argc: c_int,
    _argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    match self_pid(env) {
        Some(pid) => enif_make_pid(env, &pid),
        None => badarg(env),
    }
}

/// `send_from_thread(Pid)` -> `ok` at once; a Rust thread then sends the atom
/// `hello_from_thread` to `Pid` from outside any scheduler (`get_pid`,
/// `enif_alloc_env`, `send_from_thread`, `enif_free_env`).
unsafe extern "C" fn nif_send_from_thread(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let Some(pid) = get_pid(env, *argv) else {
        return badarg(env);
    };
    let carried = SendPid(pid.pid);
    std::thread::spawn(move || {
        let carried = carried;
        let pid = ErlNifPid { pid: carried.0 };
        // SAFETY: the env is private to this thread; its message term is consumed
        // by the send and the env is then freed.
        unsafe {
            let msg_env = enif_alloc_env();
            let msg = atom(msg_env, "hello_from_thread");
            send_from_thread(&pid, msg_env, msg);
            enif_free_env(msg_env);
        }
    });
    atom(env, "ok")
}

// -- errors -----------------------------------------------------------------

/// `badarg_nif()` raises `badarg` through `enif_make_badarg`.
unsafe extern "C" fn nif_badarg(
    env: ErlNifEnv,
    _argc: c_int,
    _argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    badarg(env)
}

/// `raise(Reason)` raises `error:Reason` through `enif_raise_exception`.
unsafe extern "C" fn nif_raise(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    enif_raise_exception(env, *argv)
}

// -- the entry --------------------------------------------------------------

macro_rules! func {
    ($name:literal, $arity:literal, $f:ident) => {
        ErlNifFunc {
            name: concat!($name, "\0").as_ptr() as *const c_char,
            arity: $arity,
            fptr: $f,
            flags: 0,
        }
    };
}

/// `ErlNifFunc` holds raw pointers, so a static array of them needs a wrapper
/// that promises it is never mutated.
struct FuncTable<const N: usize>([ErlNifFunc; N]);
unsafe impl<const N: usize> Sync for FuncTable<N> {}

/// The functions every platform has; the compiler rejects the table below if this
/// count is wrong.
const COMMON_FUNCS: usize = 25;

/// Windows has one fewer function: its callback table has no variadic
/// `enif_make_tuple`.
#[cfg(not(windows))]
const VARIADIC_FUNCS: usize = 1;
#[cfg(windows)]
const VARIADIC_FUNCS: usize = 0;

static FUNCS: FuncTable<{ COMMON_FUNCS + VARIADIC_FUNCS }> = FuncTable([
    func!("add", 2, nif_add),
    func!("int64_extremes", 0, nif_int64_extremes),
    func!("half", 1, nif_half),
    func!("kind", 1, nif_kind),
    func!("atom_length", 1, nif_atom_length),
    func!("swap", 1, nif_swap),
    func!("result", 1, nif_result),
    #[cfg(not(windows))]
    func!("variadic_tuple", 0, nif_variadic_tuple),
    func!("upcase", 1, nif_upcase),
    func!("checksum", 1, nif_checksum),
    func!("alloc_roundtrip", 0, nif_alloc_roundtrip),
    func!("sum_floats", 1, nif_sum_floats),
    func!("range_floats", 1, nif_range_floats),
    func!("list_of_atoms", 0, nif_list_of_atoms),
    func!("make_map", 0, nif_make_map),
    func!("map_put", 3, nif_map_put),
    func!("map_lookup", 2, nif_map_lookup),
    func!("map_size", 1, nif_map_size),
    func!("map_sum", 1, nif_map_sum),
    func!("counter_new", 1, nif_counter_new),
    func!("counter_bump", 1, nif_counter_bump),
    func!("counters_destroyed", 0, nif_counters_destroyed),
    func!("self_pid", 0, nif_self_pid),
    func!("send_from_thread", 1, nif_send_from_thread),
    func!("badarg_nif", 0, nif_badarg),
    func!("raise", 1, nif_raise),
]);

struct EntryHolder(ErlNifEntry);
unsafe impl Sync for EntryHolder {}

/// Register the Counter resource type.  Called by the BEAM while the library
/// loads; a nonzero return fails the load.
unsafe extern "C" fn load(
    env: ErlNifEnv,
    _priv_data: *mut *mut c_void,
    _load_info: ERL_NIF_TERM,
) -> c_int {
    let mut tried: ErlNifResourceFlags = 0;
    let rtype = enif_open_resource_type(
        env,
        ptr::null(),
        c"nif_smoke_counter".as_ptr(),
        Some(counter_dtor),
        ERL_NIF_RT_CREATE,
        &mut tried,
    );
    if rtype.is_null() {
        return 1;
    }
    COUNTER_TYPE.store(rtype, Ordering::SeqCst);
    0
}

static ENTRY: EntryHolder = EntryHolder(ErlNifEntry {
    major: ERL_NIF_MAJOR_VERSION,
    minor: ERL_NIF_MINOR_VERSION,
    name: c"nif_smoke".as_ptr(),
    num_of_funcs: FUNCS.0.len() as c_int,
    funcs: FUNCS.0.as_ptr(),
    load: Some(load),
    reload: None,
    upgrade: None,
    unload: None,
    vm_variant: c"beam.vanilla".as_ptr(),
    options: 0,
    sizeof_ErlNifResourceTypeInit: 0,
    min_erts: c"erts-13.0".as_ptr(),
});

/// The entry point the BEAM looks up in the library.
///
/// # Safety
///
/// Called by the BEAM only, once, while loading the library.
#[cfg(not(windows))]
#[no_mangle]
pub unsafe extern "C" fn nif_init() -> *const ErlNifEntry {
    &ENTRY.0
}

/// The Windows form: the BEAM passes its table of `enif_*` function pointers
/// (`beam.smp.dll` exports no `enif_*` names), which must be stored before any NIF
/// runs.
///
/// # Safety
///
/// Called by the BEAM only, once, while loading the library, with its callback
/// table.
#[cfg(windows)]
#[no_mangle]
pub unsafe extern "C" fn nif_init(
    callbacks: *const win_dyn::TWinDynNifCallbacks,
) -> *const ErlNifEntry {
    win_dyn::win_init(callbacks);
    &ENTRY.0
}
