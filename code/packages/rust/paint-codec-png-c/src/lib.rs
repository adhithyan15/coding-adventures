use paint_instructions::PixelContainer;
use std::panic::catch_unwind;

#[repr(C)]
pub struct paint_encoded_bytes_t {
    pub data: *mut u8,
    pub len: usize,
}

/// # Safety
///
/// The caller must ensure `rgba_bytes` points to at least `rgba_len` readable
/// bytes and `out_bytes` points to a valid, writable `paint_encoded_bytes_t`.
/// Null pointers are handled and cause a `0` return. On success, `out_bytes.data`
/// is heap-allocated and must be released with `paint_codec_png_free_bytes`.
#[no_mangle]
pub unsafe extern "C" fn paint_codec_png_encode_rgba8(
    width: u32,
    height: u32,
    rgba_bytes: *const u8,
    rgba_len: usize,
    out_bytes: *mut paint_encoded_bytes_t,
) -> u8 {
    if out_bytes.is_null() || rgba_bytes.is_null() {
        return 0;
    }

    let expected_len = width as usize * height as usize * 4;
    if rgba_len != expected_len {
        (*out_bytes).data = std::ptr::null_mut();
        (*out_bytes).len = 0;
        return 0;
    }

    let result = catch_unwind(|| {
        let bytes = unsafe { std::slice::from_raw_parts(rgba_bytes, rgba_len) };
        let pixels = PixelContainer::from_data(width, height, bytes.to_vec());
        paint_codec_png::encode_png(&pixels)
    });

    let Ok(png_bytes) = result else {
        (*out_bytes).data = std::ptr::null_mut();
        (*out_bytes).len = 0;
        return 0;
    };

    let mut data = png_bytes.into_boxed_slice();
    (*out_bytes).len = data.len();
    (*out_bytes).data = data.as_mut_ptr();
    std::mem::forget(data);
    1
}

/// # Safety
///
/// `data`/`len` must be exactly the pair produced by a prior successful
/// `paint_codec_png_encode_rgba8` call (i.e. a boxed slice of `len` bytes), and
/// must be freed at most once. Passing a null `data` or `len == 0` is a no-op.
#[no_mangle]
pub unsafe extern "C" fn paint_codec_png_free_bytes(data: *mut u8, len: usize) {
    if data.is_null() || len == 0 {
        return;
    }
    drop(Vec::from_raw_parts(data, len, len));
}
