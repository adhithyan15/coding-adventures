use paint_instructions::{PaintInstruction, PaintRect, PaintScene};
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
        let rect_slice = if rect_count == 0 {
            &[][..]
        } else if rects.is_null() {
            return None;
        } else {
            unsafe { std::slice::from_raw_parts(rects, rect_count) }
        };

        let mut scene = PaintScene::new(width as f64, height as f64);
        scene.background = color_to_hex(background);
        scene.instructions = rect_slice
            .iter()
            .map(|rect| {
                PaintInstruction::Rect(PaintRect::filled(
                    rect.x as f64,
                    rect.y as f64,
                    rect.width as f64,
                    rect.height as f64,
                    &color_to_hex(rect.fill),
                ))
            })
            .collect();

        Some(paint_metal::render(&scene))
    });

    let Some(pixels) = result.ok().flatten() else {
        (*out_buffer).width = 0;
        (*out_buffer).height = 0;
        (*out_buffer).data = std::ptr::null_mut();
        (*out_buffer).len = 0;
        return 0;
    };

    let mut data = pixels.data.into_boxed_slice();
    (*out_buffer).width = pixels.width;
    (*out_buffer).height = pixels.height;
    (*out_buffer).len = data.len();
    (*out_buffer).data = data.as_mut_ptr();
    std::mem::forget(data);
    1
}

#[no_mangle]
pub unsafe extern "C" fn paint_vm_metal_free_buffer_data(data: *mut u8, len: usize) {
    if data.is_null() || len == 0 {
        return;
    }
    drop(Vec::from_raw_parts(data, len, len));
}
