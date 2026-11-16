//! # VER08b runtime capability conformance for `swf-container`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![deny(unsafe_code)]

use std::path::PathBuf;

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use swf_container::{
    validate_ratio, validate_tag_code, validate_version, BitReader, SwfColorTransform,
    SwfDecompressorConfig, SwfHeader, SwfMatrix, SwfMovie, SwfRect, SwfSignature, SwfTagIter,
    SwfTagKind, ZwsConfig,
};

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

        // 1. Validation functions
        checks.push(check("validate_version", validate_version(10).is_ok()));
        checks.push(check("validate_tag_code", validate_tag_code(9).is_ok()));
        checks.push(check(
            "validate_ratio",
            validate_ratio(200_000, 10_000).is_ok(),
        ));

        // 2. Geometry and BitReader
        let rect = SwfRect::create_rect(0, 11000, 0, 8000);
        checks.push(check(
            "rect geometry",
            rect.rect_width_pixels() == 550.0 && rect.rect_height_pixels() == 400.0,
        ));

        let matrix = SwfMatrix::default_matrix();
        checks.push(check("matrix default", matrix.matrix_scale() == (1.0, 1.0)));

        let cxform = SwfColorTransform::default_cxform();
        let pixel = cxform.cxform_apply_rgba([255, 128, 64, 255]);
        checks.push(check("cxform identity", pixel == [255, 128, 64, 255]));

        let bit_bytes = [0b1011_0100, 0b1111_0000];
        let mut reader = BitReader::from_bits(&bit_bytes);
        let bit_read_ok = reader.read_ub(1) == Ok(1);
        checks.push(check("bit_reader read_ub", bit_read_ok));

        // 3. Tag streaming
        let tag_data = [(9u8 << 6) | 3, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00];
        let mut iter = SwfTagIter::from_payload(&tag_data);
        let first_tag = iter.next();
        checks.push(check(
            "tag_iter next",
            matches!(first_tag, Some(Ok(t)) if t.header.kind == SwfTagKind::SetBackgroundColor),
        ));

        // 4. Header & Movie parsing
        let mut fws_bytes = Vec::new();
        fws_bytes.extend_from_slice(b"FWS");
        fws_bytes.push(10);
        fws_bytes.extend_from_slice(&20u32.to_le_bytes()); // file length
                                                           // RECT: 5 bits nbits=1, 4x1 bit 0 -> 9 bits -> 2 bytes [0x08, 0x00]
        fws_bytes.extend_from_slice(&[0x08, 0x00]);
        fws_bytes.extend_from_slice(&0x0C00u16.to_le_bytes()); // 12 fps
        fws_bytes.extend_from_slice(&1u16.to_le_bytes()); // 1 frame
        fws_bytes.extend_from_slice(&[0x00, 0x00]); // End tag

        let header_res = SwfHeader::parse_header(&fws_bytes);
        checks.push(check("parse_header", header_res.is_ok()));

        let config = SwfDecompressorConfig::default_decompressor_config();
        let movie_res = SwfMovie::from_swf_bytes(&fws_bytes, &config);
        checks.push(check("movie from_swf_bytes", movie_res.is_ok()));

        // 5. Signature & ZWS Config
        let sig_res = SwfSignature::from_signature_bytes(*b"FWS");
        checks.push(check("signature check", sig_res == Ok(SwfSignature::Fws)));

        let mut zws_header = Vec::new();
        zws_header.extend_from_slice(b"ZWS");
        zws_header.push(13);
        zws_header.extend_from_slice(&1000u32.to_le_bytes());
        zws_header.extend_from_slice(&500u32.to_le_bytes());
        zws_header.push(93); // prop byte
        zws_header.extend_from_slice(&0x0010_0000u32.to_le_bytes());
        let zws_res = ZwsConfig::parse_zws_header(&zws_header);
        checks.push(check("zws_config parse", zws_res.is_ok()));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/swf-container",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
