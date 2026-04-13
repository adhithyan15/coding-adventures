use std::ffi::{c_char, c_long, c_void};

use paint_instructions::PixelContainer;
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
    ruby_bridge::define_module_under(get_coding_adventures_module(), "PaintCodecPngNative")
}

fn parse_bytes(bytes_value: VALUE) -> Vec<u8> {
    let len = ruby_bridge::array_len(bytes_value);
    let mut bytes = Vec::with_capacity(len);
    for index in 0..len {
      let value = ruby_bridge::array_entry(bytes_value, index);
      let number = unsafe { ruby_bridge::rb_num2long(value) };
      if !(0..=255).contains(&number) {
        ruby_bridge::raise_arg_error("PNG codec expects an array of RGBA bytes");
      }
      bytes.push(number as u8);
    }
    bytes
}

extern "C" fn encode_rgba8_native(
    _module: VALUE,
    width_value: VALUE,
    height_value: VALUE,
    bytes_value: VALUE,
) -> VALUE {
    let width = unsafe { ruby_bridge::rb_num2long(width_value) } as u32;
    let height = unsafe { ruby_bridge::rb_num2long(height_value) } as u32;
    let bytes = parse_bytes(bytes_value);

    if bytes.len() != width as usize * height as usize * 4 {
        ruby_bridge::raise_arg_error("RGBA buffer length does not match width * height * 4");
    }

    let pixels = PixelContainer::from_data(width, height, bytes);
    make_binary_string(&paint_codec_png::encode_png(&pixels))
}

#[no_mangle]
pub extern "C" fn Init_paint_codec_png_native() {
    let module = get_module();
    ruby_bridge::define_module_function_raw(
        module,
        "encode_rgba8_native",
        encode_rgba8_native as *const c_void,
        3,
    );
}
