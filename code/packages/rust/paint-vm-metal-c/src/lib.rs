use paint_instructions::{
    PaintBase, PaintInstruction, PaintRect, PaintScene, PaintText, TextAlign,
};
use std::panic::catch_unwind;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct paint_rgba8_color_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct paint_rect_instruction_t {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub fill: paint_rgba8_color_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct paint_text_instruction_t {
    pub x: f64,
    pub y: f64,
    pub text: *const u8,
    pub text_len: usize,
    pub font_ref: *const u8,
    pub font_ref_len: usize,
    pub font_size: f64,
    pub fill: paint_rgba8_color_t,
    pub text_align: u32,
}

#[repr(C)]
pub struct paint_rgba8_buffer_t {
    pub width: u32,
    pub height: u32,
    pub data: *mut u8,
    pub len: usize,
}

fn color_to_hex(color: paint_rgba8_color_t) -> String {
    if color.a == 255 {
        format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            color.r, color.g, color.b, color.a
        )
    }
}

unsafe fn utf8_from_parts(ptr: *const u8, len: usize) -> Option<String> {
    if len == 0 {
        return Some(String::new());
    }
    if ptr.is_null() {
        return None;
    }
    std::str::from_utf8(std::slice::from_raw_parts(ptr, len))
        .ok()
        .map(str::to_string)
}

fn text_align_from_u32(value: u32) -> Option<TextAlign> {
    match value {
        1 => Some(TextAlign::Center),
        2 => Some(TextAlign::Right),
        _ => Some(TextAlign::Left),
    }
}

unsafe fn build_scene(
    width: u32,
    height: u32,
    background: paint_rgba8_color_t,
    rects: *const paint_rect_instruction_t,
    rect_count: usize,
    texts: *const paint_text_instruction_t,
    text_count: usize,
) -> Option<PaintScene> {
    let rect_slice = if rect_count == 0 {
        &[][..]
    } else if rects.is_null() {
        return None;
    } else {
        std::slice::from_raw_parts(rects, rect_count)
    };

    let text_slice = if text_count == 0 {
        &[][..]
    } else if texts.is_null() {
        return None;
    } else {
        std::slice::from_raw_parts(texts, text_count)
    };

    let mut scene = PaintScene::new(width as f64, height as f64);
    scene.background = color_to_hex(background);
    scene.instructions.extend(rect_slice.iter().map(|rect| {
        PaintInstruction::Rect(PaintRect::filled(
            rect.x as f64,
            rect.y as f64,
            rect.width as f64,
            rect.height as f64,
            &color_to_hex(rect.fill),
        ))
    }));
    for text in text_slice {
        let text_value = utf8_from_parts(text.text, text.text_len)?;
        let font_ref = if text.font_ref_len == 0 {
            None
        } else {
            Some(utf8_from_parts(text.font_ref, text.font_ref_len)?)
        };
        scene.instructions.push(PaintInstruction::Text(PaintText {
            base: PaintBase::default(),
            x: text.x,
            y: text.y,
            text: text_value,
            font_ref,
            font_size: text.font_size,
            fill: Some(color_to_hex(text.fill)),
            text_align: text_align_from_u32(text.text_align),
        }));
    }

    Some(scene)
}

unsafe fn write_pixels_to_out_buffer(
    pixels: paint_instructions::PixelContainer,
    out_buffer: *mut paint_rgba8_buffer_t,
) {
    let mut data = pixels.data.into_boxed_slice();
    (*out_buffer).width = pixels.width;
    (*out_buffer).height = pixels.height;
    (*out_buffer).len = data.len();
    (*out_buffer).data = data.as_mut_ptr();
    std::mem::forget(data);
}

#[no_mangle]
pub unsafe extern "C" fn paint_vm_metal_render_rect_scene(
    width: u32,
    height: u32,
    background: paint_rgba8_color_t,
    rects: *const paint_rect_instruction_t,
    rect_count: usize,
    out_buffer: *mut paint_rgba8_buffer_t,
) -> u8 {
    if out_buffer.is_null() {
        return 0;
    }

    let result = catch_unwind(|| {
        let scene = unsafe {
            build_scene(
                width,
                height,
                background,
                rects,
                rect_count,
                std::ptr::null(),
                0,
            )?
        };
        Some(paint_metal::render(&scene))
    });

    let Some(pixels) = result.ok().flatten() else {
        (*out_buffer).width = 0;
        (*out_buffer).height = 0;
        (*out_buffer).data = std::ptr::null_mut();
        (*out_buffer).len = 0;
        return 0;
    };

    write_pixels_to_out_buffer(pixels, out_buffer);
    1
}

#[no_mangle]
pub unsafe extern "C" fn paint_vm_metal_render_scene(
    width: u32,
    height: u32,
    background: paint_rgba8_color_t,
    rects: *const paint_rect_instruction_t,
    rect_count: usize,
    texts: *const paint_text_instruction_t,
    text_count: usize,
    out_buffer: *mut paint_rgba8_buffer_t,
) -> u8 {
    if out_buffer.is_null() {
        return 0;
    }

    let result = catch_unwind(|| {
        let scene = unsafe {
            build_scene(
                width, height, background, rects, rect_count, texts, text_count,
            )?
        };
        Some(paint_metal::render(&scene))
    });

    let Some(pixels) = result.ok().flatten() else {
        (*out_buffer).width = 0;
        (*out_buffer).height = 0;
        (*out_buffer).data = std::ptr::null_mut();
        (*out_buffer).len = 0;
        return 0;
    };

    write_pixels_to_out_buffer(pixels, out_buffer);
    1
}

#[no_mangle]
pub unsafe extern "C" fn paint_vm_metal_free_buffer_data(data: *mut u8, len: usize) {
    if data.is_null() || len == 0 {
        return;
    }
    drop(Vec::from_raw_parts(data, len, len));
}
