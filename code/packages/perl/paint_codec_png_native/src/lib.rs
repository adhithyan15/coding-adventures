#![allow(non_snake_case, non_camel_case_types)]

use perl_bridge::{
    die, newSVpvn, sv_2iv, sv_2pv_flags, xs_boot_finish, xs_bootstrap, CV, SV,
};
use paint_instructions::PixelContainer;
use std::ffi::c_char;
use std::panic::catch_unwind;

extern "C" {
    #[link_name = "Perl_newXS"]
    fn newXS(
        name: *const c_char,
        subaddr: unsafe extern "C" fn(*mut CV),
        filename: *const c_char,
    ) -> *mut CV;
}

extern "C" {
    static mut PL_markstack_ptr: *mut i32;
    static mut PL_stack_sp: *mut *mut SV;
    static mut PL_stack_base: *mut *mut SV;
}

unsafe fn xsub_args() -> (*mut *mut SV, i32, i32) {
    let mark = *PL_markstack_ptr;
    let ax = mark.saturating_add(1);
    let base = PL_stack_base;
    let sp = PL_stack_sp;

    const MAX_SANE_AX: i32 = 4096;
    if ax > MAX_SANE_AX || ax < 0 {
        return (base, 0, 0);
    }

    let base_ax = base.add(ax as usize);
    let items = if sp >= base_ax {
        ((sp as usize - base_ax as usize) / std::mem::size_of::<*mut SV>()) as i32 + 1
    } else {
        0
    };
    (base, ax, items)
}

unsafe fn xsub_return(n: i32, ax: i32) {
    PL_stack_sp = PL_stack_base.add((ax + n - 1) as usize);
    PL_markstack_ptr = PL_markstack_ptr.sub(1);
}

unsafe fn set_return(base: *mut *mut SV, ax: i32, n: i32, sv: *mut SV) {
    *base.add((ax + n) as usize) = sv;
}

unsafe fn arg_u32(base: *mut *mut SV, ax: i32, n: i32) -> u32 {
    let sv = *base.add((ax + n) as usize);
    if sv.is_null() {
        die("numeric argument is null");
        return 0;
    }
    sv_2iv(sv) as u32
}

unsafe fn arg_bytes(base: *mut *mut SV, ax: i32, n: i32) -> Vec<u8> {
    let sv = *base.add((ax + n) as usize);
    if sv.is_null() {
        die("byte-string argument is null");
        return Vec::new();
    }
    let mut len: usize = 0;
    let ptr = sv_2pv_flags(sv, &mut len, 0);
    if ptr.is_null() {
        die("expected a byte string");
        return Vec::new();
    }
    std::slice::from_raw_parts(ptr as *const u8, len).to_vec()
}

extern "C" fn xs_encode_rgba8_native(_cv: *mut CV) {
    let result = catch_unwind(|| unsafe {
        let (base, ax, items) = xsub_args();
        if items < 3 {
            die("encode_rgba8_native: expected width, height, rgba bytes");
            return;
        }

        let width = arg_u32(base, ax, 0);
        let height = arg_u32(base, ax, 1);
        let bytes = arg_bytes(base, ax, 2);

        if bytes.len() != width as usize * height as usize * 4 {
            die("RGBA buffer length does not match width * height * 4");
        }

        let pixels = PixelContainer::from_data(width, height, bytes);
        let png = paint_codec_png::encode_png(&pixels);
        set_return(base, ax, 0, newSVpvn(png.as_ptr() as *const c_char, png.len()));
        xsub_return(1, ax);
    });

    if result.is_err() {
        unsafe { die("PNG codec operation panicked unexpectedly") };
    }
}

#[no_mangle]
pub unsafe extern "C" fn boot_CodingAdventures__PaintCodecPngNative(cv: *mut CV) {
    let file = b"PaintCodecPngNative.so\0".as_ptr() as *const c_char;
    let ax = xs_bootstrap(cv, file);
    newXS(
        b"CodingAdventures::PaintCodecPngNative::encode_rgba8_native\0".as_ptr()
            as *const c_char,
        xs_encode_rgba8_native,
        file,
    );
    xs_boot_finish(ax);
}
