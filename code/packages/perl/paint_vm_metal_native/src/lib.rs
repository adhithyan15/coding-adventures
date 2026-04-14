#![allow(non_snake_case, non_camel_case_types)]

use paint_instructions::{PaintInstruction, PaintRect, PaintScene};
use perl_bridge::{
    die, newSViv, newSVpvn, newXS, sv_2nv, sv_2pv_flags, xs_boot_finish, xs_bootstrap, xsub_frame,
    xsub_return, CV, IV, SV,
};
use std::ffi::c_char;
use std::panic::catch_unwind;

unsafe fn set_return(base: *mut *mut SV, ax: i32, n: i32, sv: *mut SV) {
    *base.add((ax + n) as usize) = sv;
}

unsafe fn arg_f64(base: *mut *mut SV, ax: i32, n: i32) -> f64 {
    let sv = *base.add((ax + n) as usize);
    if sv.is_null() {
        die("numeric argument is null");
        return 0.0;
    }
    sv_2nv(sv)
}

unsafe fn arg_string(base: *mut *mut SV, ax: i32, n: i32) -> (String, usize) {
    let sv = *base.add((ax + n) as usize);
    if sv.is_null() {
        die("string argument is null");
        return (String::new(), 0);
    }
    let mut len: usize = 0;
    let ptr = sv_2pv_flags(sv, &mut len, 0);
    if ptr.is_null() {
        die("expected a string argument");
        return (String::new(), 0);
    }
    let bytes = std::slice::from_raw_parts(ptr as *const u8, len);
    (String::from_utf8_lossy(bytes).to_string(), len)
}

fn parse_rect_blob(blob: &str) -> Vec<PaintInstruction> {
    let mut rects = Vec::new();
    for line in blob.lines() {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(5, '\t');
        let x: f64 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
        let y: f64 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
        let width: f64 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
        let height: f64 = parts.next().unwrap_or("0").parse().unwrap_or(0.0);
        let fill = parts.next().unwrap_or("#000000");

        rects.push(PaintInstruction::Rect(PaintRect::filled(
            x, y, width, height, fill,
        )));
    }
    rects
}

extern "C" fn xs_render_rect_scene_native(_cv: *mut CV) {
    let result = catch_unwind(|| unsafe {
        let frame = xsub_frame();
        let base = frame.base;
        let ax = frame.ax;
        let items = frame.items;
        if items < 4 {
            die("render_rect_scene_native: expected width, height, background, rect_blob");
            return;
        }

        let width = arg_f64(base, ax, 0);
        let height = arg_f64(base, ax, 1);
        let (background, _) = arg_string(base, ax, 2);
        let (rect_blob, _) = arg_string(base, ax, 3);

        let mut scene = PaintScene::new(width, height);
        scene.background = background;
        scene.instructions = parse_rect_blob(&rect_blob);

        let pixels = catch_unwind(|| paint_metal::render(&scene))
            .unwrap_or_else(|_| die("Metal rendering failed"));

        set_return(base, ax, 0, newSViv(pixels.width as IV));
        set_return(base, ax, 1, newSViv(pixels.height as IV));
        set_return(
            base,
            ax,
            2,
            newSVpvn(pixels.data.as_ptr() as *const c_char, pixels.data.len()),
        );
        xsub_return(3, ax);
    });

    if result.is_err() {
        unsafe { die("paint VM operation panicked unexpectedly") };
    }
}

#[no_mangle]
pub unsafe extern "C" fn boot_CodingAdventures__PaintVmMetalNative(cv: *mut CV) {
    let file = b"PaintVmMetalNative.so\0".as_ptr() as *const c_char;
    let ax = xs_bootstrap(cv, file);
    newXS(
        b"CodingAdventures::PaintVmMetalNative::render_rect_scene_native\0".as_ptr()
            as *const c_char,
        xs_render_rect_scene_native,
        file,
    );
    xs_boot_finish(ax);
}
