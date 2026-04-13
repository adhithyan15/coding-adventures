#![allow(non_snake_case)]

use erl_nif_bridge::{
    badarg, enif_inspect_binary, enif_make_new_binary, get_i64, ok_tuple,
    error_tuple, ErlNifBinary, ErlNifEnv, ErlNifFunc, ERL_NIF_DIRTY_JOB_CPU_BOUND,
    ERL_NIF_TERM,
};
use paint_instructions::PixelContainer;
use std::ffi::c_int;
use std::slice;

unsafe fn get_binary_bytes(env: ErlNifEnv, term: ERL_NIF_TERM) -> Option<Vec<u8>> {
    let mut bin = std::mem::MaybeUninit::<ErlNifBinary>::zeroed().assume_init();
    if enif_inspect_binary(env, term, &mut bin) == 0 {
        return None;
    }

    Some(slice::from_raw_parts(bin.data, bin.size).to_vec())
}

unsafe fn make_binary(env: ErlNifEnv, bytes: &[u8]) -> Option<ERL_NIF_TERM> {
    let mut term: ERL_NIF_TERM = 0;
    let ptr = enif_make_new_binary(env, bytes.len(), &mut term);
    if ptr.is_null() {
        return None;
    }

    if !bytes.is_empty() {
        ptr.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
    }

    Some(term)
}

pub unsafe extern "C" fn nif_encode_rgba8(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let width = match get_i64(env, *argv.add(0)) {
        Some(value) if value >= 0 => value as u32,
        _ => return badarg(env),
    };

    let height = match get_i64(env, *argv.add(1)) {
        Some(value) if value >= 0 => value as u32,
        _ => return badarg(env),
    };

    let bytes = match get_binary_bytes(env, *argv.add(2)) {
        Some(value) => value,
        None => return badarg(env),
    };

    let expected_len = width as usize * height as usize * 4;
    if bytes.len() != expected_len {
        return error_tuple(env, "invalid_pixel_data");
    }

    let pixels = PixelContainer::from_data(width, height, bytes);
    let png = paint_codec_png::encode_png(&pixels);

    match make_binary(env, &png) {
        Some(value) => ok_tuple(env, value),
        None => error_tuple(env, "alloc_failed"),
    }
}

struct FuncTable([ErlNifFunc; 1]);
unsafe impl Sync for FuncTable {}

static FUNCS: FuncTable = FuncTable([ErlNifFunc {
    name: b"encode_rgba8_native\0".as_ptr() as *const _,
    arity: 3,
    fptr: nif_encode_rgba8,
    flags: ERL_NIF_DIRTY_JOB_CPU_BOUND,
}]);

struct NifEntry(erl_nif_bridge::ErlNifEntry);
unsafe impl Sync for NifEntry {}

static MODULE_NAME_BYTES: &[u8] = b"Elixir.CodingAdventures.PaintCodecPngNative\0";
static VM_VARIANT_BYTES: &[u8] = b"beam.vanilla\0";
static MIN_ERTS_BYTES: &[u8] = b"erts-13.0\0";

static NIF_ENTRY: NifEntry = NifEntry(erl_nif_bridge::ErlNifEntry {
    major: erl_nif_bridge::ERL_NIF_MAJOR_VERSION,
    minor: erl_nif_bridge::ERL_NIF_MINOR_VERSION,
    name: MODULE_NAME_BYTES.as_ptr() as *const std::ffi::c_char,
    num_of_funcs: 1,
    funcs: FUNCS.0.as_ptr(),
    load: None,
    reload: None,
    upgrade: None,
    unload: None,
    vm_variant: VM_VARIANT_BYTES.as_ptr() as *const std::ffi::c_char,
    options: 0,
    sizeof_ErlNifResourceTypeInit: 0,
    min_erts: MIN_ERTS_BYTES.as_ptr() as *const std::ffi::c_char,
});

#[no_mangle]
pub unsafe extern "C" fn nif_init() -> *const erl_nif_bridge::ErlNifEntry {
    &NIF_ENTRY.0
}
