//! # VER08b runtime capability conformance for `palette-indexed`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![deny(unsafe_code)]

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use palette_indexed::{
    blit_indexed_to_container, blit_scanline_to_container, crossfade_palette_into,
    expand_row_to_rgba_into, fade_color, fade_palette_into, pack_row_into, unpack_row_into,
    BitDepth, ChannelBits, IndexedCanvas, Palette, PixelContainer, BLIT_SPRITE,
};
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        let mut checks = Vec::new();

        let indices = [0u8, 1, 2, 3, 3, 2, 1, 0];
        let mut packed = [0u8; 4];
        let packed_res = pack_row_into(BitDepth::Bpp2, &indices, &mut packed);
        checks.push(check("pack_row_into succeeds", packed_res == Ok(2)));

        let mut back = [0u8; 8];
        let unpack_res = unpack_row_into(BitDepth::Bpp2, &packed, 8, &mut back);
        checks.push(check(
            "unpack_row_into round-trips",
            unpack_res.is_ok() && back == indices,
        ));

        let palette = Palette::from_colors(
            BitDepth::Bpp2,
            &[
                [0, 0, 0, 255],
                [85, 85, 85, 255],
                [170, 170, 170, 255],
                [255, 255, 255, 255],
            ],
        )
        .expect("palette");

        let mut line = [0u8; 32];
        let expand_res =
            expand_row_to_rgba_into(BitDepth::Bpp2, &packed, 8, &palette, true, &mut line);
        checks.push(check(
            "expand_row_to_rgba_into succeeds",
            expand_res == Ok(32),
        ));

        let fade = fade_color(
            [255, 255, 255, 255],
            [0, 0, 0, 255],
            1,
            2,
            ChannelBits::Bits5,
        );
        checks.push(check("fade_color succeeds", fade.is_ok()));

        let mut faded = Palette::blank(BitDepth::Bpp2);
        let fade_res = fade_palette_into(
            &palette,
            &mut faded,
            [0, 0, 0, 255],
            2,
            4,
            ChannelBits::Bits3,
        );
        checks.push(check("fade_palette_into succeeds", fade_res.is_ok()));

        let mut mixed = Palette::blank(BitDepth::Bpp2);
        let cross_res =
            crossfade_palette_into(&palette, &faded, &mut mixed, 1, 2, ChannelBits::Bits8);
        checks.push(check("crossfade_palette_into succeeds", cross_res.is_ok()));

        let mut fb = [0u8; 16];
        if let Ok(mut canvas) = IndexedCanvas::attach(BitDepth::Bpp2, 8, 4, 2, &mut fb) {
            let _ = canvas.put_index(1, 1, 2);
            let img = canvas.as_image();
            let mut dst = PixelContainer::new(8, 4);
            let blit = blit_indexed_to_container(&img, &palette, BLIT_SPRITE, &mut dst, 0, 0);
            checks.push(check("blit_indexed_to_container succeeds", blit.is_ok()));
            let scan = blit_scanline_to_container(&img, 1, &palette, BLIT_SPRITE, &mut dst, 0, 1);
            checks.push(check("blit_scanline_to_container succeeds", scan.is_ok()));
        } else {
            checks.push(check("canvas attach succeeds", false));
        }

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/palette-indexed",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
