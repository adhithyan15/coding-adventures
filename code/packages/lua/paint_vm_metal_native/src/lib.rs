use std::ffi::{c_char, c_int};

use lua_bridge::{
    get_f64, get_str, lua_gettop, lua_newtable, lua_pop, lua_pushinteger,
    lua_pushlstring, lua_rawgeti, lua_rawlen, lua_setfield, lua_type,
    lua_Integer, lua_State, LUA_TTABLE, raise_error,
};
use paint_instructions::{PaintInstruction, PaintRect, PaintScene};

unsafe fn parse_rects(L: *mut lua_State, idx: c_int) -> Vec<PaintInstruction> {
    if lua_type(L, idx) != LUA_TTABLE {
        raise_error(L, "rects must be a table");
    }

    let len = lua_rawlen(L, idx);
    let mut rects = Vec::with_capacity(len as usize);
    for i in 1..=len {
        lua_rawgeti(L, idx, i);
        let rect_index = lua_gettop(L);
        if lua_type(L, rect_index) != LUA_TTABLE {
            raise_error(L, "each rect must be a table");
        }

        lua_rawgeti(L, rect_index, 1);
        let x = get_f64(L, -1).unwrap_or_else(|| raise_error(L, "rect x must be numeric"));
        lua_pop(L, 1);

        lua_rawgeti(L, rect_index, 2);
        let y = get_f64(L, -1).unwrap_or_else(|| raise_error(L, "rect y must be numeric"));
        lua_pop(L, 1);

        lua_rawgeti(L, rect_index, 3);
        let width = get_f64(L, -1).unwrap_or_else(|| raise_error(L, "rect width must be numeric"));
        lua_pop(L, 1);

        lua_rawgeti(L, rect_index, 4);
        let height = get_f64(L, -1).unwrap_or_else(|| raise_error(L, "rect height must be numeric"));
        lua_pop(L, 1);

        lua_rawgeti(L, rect_index, 5);
        let fill = get_str(L, -1).unwrap_or_else(|| raise_error(L, "rect fill must be a string"));
        lua_pop(L, 1);

        if width < 0.0 || height < 0.0 {
            raise_error(L, "rect width and height must be non-negative");
        }

        rects.push(PaintInstruction::Rect(PaintRect::filled(
            x, y, width, height, &fill,
        )));
        lua_pop(L, 1);
    }

    rects
}

unsafe extern "C" fn lua_render_rect_scene_native(L: *mut lua_State) -> c_int {
    let width = get_f64(L, 1).unwrap_or_else(|| raise_error(L, "width must be numeric"));
    let height = get_f64(L, 2).unwrap_or_else(|| raise_error(L, "height must be numeric"));
    let background = get_str(L, 3).unwrap_or_else(|| raise_error(L, "background must be a string"));
    let rects = parse_rects(L, 4);

    let mut scene = PaintScene::new(width, height);
    scene.background = background;
    scene.instructions = rects;

    let pixels = std::panic::catch_unwind(|| paint_metal::render(&scene))
        .unwrap_or_else(|_| raise_error(L, "Metal rendering failed"));

    lua_newtable(L);
    lua_pushinteger(L, pixels.width as lua_Integer);
    lua_setfield(L, -2, b"width\0".as_ptr() as *const c_char);
    lua_pushinteger(L, pixels.height as lua_Integer);
    lua_setfield(L, -2, b"height\0".as_ptr() as *const c_char);
    lua_pushlstring(L, pixels.data.as_ptr() as *const c_char, pixels.data.len());
    lua_setfield(L, -2, b"data\0".as_ptr() as *const c_char);
    1
}

struct FuncTable([lua_bridge::luaL_Reg; 2]);
unsafe impl Sync for FuncTable {}

static FUNCS: FuncTable = FuncTable([
    lua_bridge::luaL_Reg {
        name: b"render_rect_scene_native\0".as_ptr() as *const _,
        func: Some(lua_render_rect_scene_native),
    },
    lua_bridge::luaL_Reg {
        name: std::ptr::null(),
        func: None,
    },
]);

#[no_mangle]
pub unsafe extern "C" fn luaopen_paint_vm_metal_native(L: *mut lua_State) -> c_int {
    lua_bridge::register_lib(L, &FUNCS.0);
    1
}
