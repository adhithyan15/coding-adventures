use std::ffi::{c_char, c_long, c_void};

use paint_instructions::{PaintInstruction, PaintRect, PaintScene};
use ruby_bridge::VALUE;

extern "C" {
    fn rb_str_new(ptr: *const c_char, len: c_long) -> VALUE;
}

fn make_binary_string(bytes: &[u8]) -> VALUE {
    unsafe { rb_str_new(bytes.as_ptr() as *const c_char, bytes.len() as c_long) }
}

fn get_coding_adventures_module() -> VALUE {
    ruby_bridge::define_module("CodingAdventures")
}

fn get_module() -> VALUE {
    ruby_bridge::define_module_under(get_coding_adventures_module(), "PaintVmMetalNative")
}

fn parse_rect(rect_value: VALUE) -> PaintInstruction {
    let x = ruby_bridge::f64_from_rb(ruby_bridge::array_entry(rect_value, 0));
    let y = ruby_bridge::f64_from_rb(ruby_bridge::array_entry(rect_value, 1));
    let width = ruby_bridge::f64_from_rb(ruby_bridge::array_entry(rect_value, 2));
    let height = ruby_bridge::f64_from_rb(ruby_bridge::array_entry(rect_value, 3));
    let fill = ruby_bridge::str_from_rb(ruby_bridge::array_entry(rect_value, 4))
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("rect fill must be a String"));

    if width < 0.0 || height < 0.0 {
        ruby_bridge::raise_arg_error("rect width and height must be non-negative");
    }

    PaintInstruction::Rect(PaintRect::filled(x, y, width, height, &fill))
}

fn parse_rects(rects_value: VALUE) -> Vec<PaintInstruction> {
    let len = ruby_bridge::array_len(rects_value);
    let mut rects = Vec::with_capacity(len);

    for index in 0..len {
        let rect_value = ruby_bridge::array_entry(rects_value, index);
        rects.push(parse_rect(rect_value));
    }

    rects
}

extern "C" fn render_rect_scene_native(
    _module: VALUE,
    width_value: VALUE,
    height_value: VALUE,
    background_value: VALUE,
    rects_value: VALUE,
) -> VALUE {
    let width = ruby_bridge::f64_from_rb(width_value);
    let height = ruby_bridge::f64_from_rb(height_value);
    let background = ruby_bridge::str_from_rb(background_value)
        .unwrap_or_else(|| ruby_bridge::raise_arg_error("background must be a String"));

    let mut scene = PaintScene::new(width, height);
    scene.background = background;
    scene.instructions = parse_rects(rects_value);

    let pixels = std::panic::catch_unwind(|| paint_metal::render(&scene))
        .unwrap_or_else(|_| ruby_bridge::raise_runtime_error("Metal rendering failed"));

    let payload = ruby_bridge::array_new();
    ruby_bridge::array_push(payload, ruby_bridge::usize_to_rb(pixels.width as usize));
    ruby_bridge::array_push(payload, ruby_bridge::usize_to_rb(pixels.height as usize));
    ruby_bridge::array_push(payload, make_binary_string(&pixels.data));
    payload
}

#[no_mangle]
pub extern "C" fn Init_paint_vm_metal_native() {
    let module = get_module();
    ruby_bridge::define_module_function_raw(
        module,
        "render_rect_scene_native",
        render_rect_scene_native as *const c_void,
        4,
    );
}
