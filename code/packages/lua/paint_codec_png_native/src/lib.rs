use std::ffi::{c_char, c_int};

use lua_bridge::{get_f64, lua_pushlstring, lua_tolstring, lua_State, raise_error};
use paint_instructions::PixelContainer;

unsafe extern "C" fn lua_encode_rgba8_native(L: *mut lua_State) -> c_int {
    let width = get_f64(L, 1).unwrap_or_else(|| raise_error(L, "width must be numeric")) as u32;
    let height = get_f64(L, 2).unwrap_or_else(|| raise_error(L, "height must be numeric")) as u32;

    let mut len: usize = 0;
    let ptr = lua_tolstring(L, 3, &mut len);
    if ptr.is_null() {
        raise_error(L, "rgba bytes must be a string");
    }

    let bytes = std::slice::from_raw_parts(ptr as *const u8, len).to_vec();
    if bytes.len() != width as usize * height as usize * 4 {
        raise_error(L, "RGBA buffer length does not match width * height * 4");
    }

    let pixels = PixelContainer::from_data(width, height, bytes);
    let png = paint_codec_png::encode_png(&pixels);
    lua_pushlstring(L, png.as_ptr() as *const c_char, png.len());
    1
}

struct FuncTable([lua_bridge::luaL_Reg; 2]);
unsafe impl Sync for FuncTable {}

static FUNCS: FuncTable = FuncTable([
    lua_bridge::luaL_Reg {
        name: b"encode_rgba8_native\0".as_ptr() as *const _,
        func: Some(lua_encode_rgba8_native),
    },
    lua_bridge::luaL_Reg {
        name: std::ptr::null(),
        func: None,
    },
]);

#[no_mangle]
pub unsafe extern "C" fn luaopen_paint_codec_png_native(L: *mut lua_State) -> c_int {
    lua_bridge::register_lib(L, &FUNCS.0);
    1
}
