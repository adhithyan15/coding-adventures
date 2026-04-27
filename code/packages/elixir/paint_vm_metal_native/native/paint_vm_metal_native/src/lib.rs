#![allow(non_snake_case)]

use erl_nif_bridge::{
    badarg, enif_get_list_cell, enif_get_list_length, enif_get_tuple,
    enif_inspect_binary, enif_make_new_binary, enif_make_tuple_from_array,
    error_tuple, get_f64, make_i64, ok_tuple, ErlNifBinary, ErlNifEnv,
    ErlNifFunc, ERL_NIF_DIRTY_JOB_CPU_BOUND, ERL_NIF_TERM,
};
use paint_instructions::{PaintInstruction, PaintRect, PaintScene};
use std::ffi::c_int;
use std::ptr;
use std::slice;

unsafe fn get_utf8_binary(env: ErlNifEnv, term: ERL_NIF_TERM) -> Option<String> {
    let mut bin = std::mem::MaybeUninit::<ErlNifBinary>::zeroed().assume_init();
    if enif_inspect_binary(env, term, &mut bin) == 0 {
        return None;
    }

    let bytes = slice::from_raw_parts(bin.data, bin.size);
    std::str::from_utf8(bytes).ok().map(|value| value.to_string())
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

unsafe fn get_rect(env: ErlNifEnv, term: ERL_NIF_TERM) -> Option<PaintInstruction> {
    let mut arity: c_int = 0;
    let mut elements: *const ERL_NIF_TERM = ptr::null();
    if enif_get_tuple(env, term, &mut arity, &mut elements) == 0 || arity != 5 {
        return None;
    }

    let x = get_f64(env, *elements.add(0))?;
    let y = get_f64(env, *elements.add(1))?;
    let width = get_f64(env, *elements.add(2))?;
    let height = get_f64(env, *elements.add(3))?;
    let fill = get_utf8_binary(env, *elements.add(4))?;

    if width < 0.0 || height < 0.0 {
        return None;
    }

    Some(PaintInstruction::Rect(PaintRect::filled(x, y, width, height, &fill)))
}

unsafe fn get_rect_list(env: ErlNifEnv, term: ERL_NIF_TERM) -> Option<Vec<PaintInstruction>> {
    let mut len: u32 = 0;
    if enif_get_list_length(env, term, &mut len) == 0 {
        return None;
    }

    let mut current = term;
    let mut rects = Vec::with_capacity(len as usize);

    loop {
        let mut head: ERL_NIF_TERM = 0;
        let mut tail: ERL_NIF_TERM = 0;
        if enif_get_list_cell(env, current, &mut head, &mut tail) == 0 {
            break;
        }

        rects.push(get_rect(env, head)?);
        current = tail;
    }

    Some(rects)
}

pub unsafe extern "C" fn nif_render_rect_scene(
    env: ErlNifEnv,
    _argc: c_int,
    argv: *const ERL_NIF_TERM,
) -> ERL_NIF_TERM {
    let width = match get_f64(env, *argv.add(0)) {
        Some(value) if value >= 0.0 => value,
        _ => return badarg(env),
    };

    let height = match get_f64(env, *argv.add(1)) {
        Some(value) if value >= 0.0 => value,
        _ => return badarg(env),
    };

    let background = match get_utf8_binary(env, *argv.add(2)) {
        Some(value) => value,
        None => return badarg(env),
    };

    let rects = match get_rect_list(env, *argv.add(3)) {
        Some(value) => value,
        None => return badarg(env),
    };

    let mut scene = PaintScene::new(width, height);
    scene.background = background;
    scene.instructions = rects;

    let pixels = match std::panic::catch_unwind(|| paint_metal::render(&scene)) {
        Ok(value) => value,
        Err(_) => return error_tuple(env, "render_failed"),
    };

    let data_term = match make_binary(env, &pixels.data) {
        Some(value) => value,
        None => return error_tuple(env, "alloc_failed"),
    };

    let payload = [
        make_i64(env, pixels.width as i64),
        make_i64(env, pixels.height as i64),
        data_term,
    ];
    let pixel_tuple = enif_make_tuple_from_array(env, payload.as_ptr(), 3);
    ok_tuple(env, pixel_tuple)
}

struct FuncTable([ErlNifFunc; 1]);
unsafe impl Sync for FuncTable {}

static FUNCS: FuncTable = FuncTable([ErlNifFunc {
    name: b"render_rect_scene_native\0".as_ptr() as *const _,
    arity: 4,
    fptr: nif_render_rect_scene,
    flags: ERL_NIF_DIRTY_JOB_CPU_BOUND,
}]);

struct NifEntry(erl_nif_bridge::ErlNifEntry);
unsafe impl Sync for NifEntry {}

static MODULE_NAME_BYTES: &[u8] = b"Elixir.CodingAdventures.PaintVmMetalNative\0";
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
