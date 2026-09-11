use barcode_2d::{layout, render_scene, Barcode2DLayoutConfig};
use image_geometric_transforms::{perspective_warp, Interpolation, OutOfBounds};
use pixel_container::PixelContainer;
use qr_code::{encode, EccLevel};
use qr_locate::{locate_and_decode, QrLocateError};

fn to_f32(m: &matrix::Matrix) -> [[f32; 3]; 3] {
    let mut out = [[0f32; 3]; 3];
    for (r, row) in m.data.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            out[r][c] = v as f32;
        }
    }
    out
}

/// The forward transform (clean image space -> photo space): rotate
/// around the clean image's own center, then translate to the photo
/// canvas's center. Built as an ordinary 3x3 affine matrix in `(x, y)`
/// order to match `perspective_warp`'s own convention (see
/// `qr-locate.md` §3).
fn build_forward(dest_x: f64, dest_y: f64, angle_deg: f64, src_cx: f64, src_cy: f64) -> matrix::Matrix {
    let theta = angle_deg.to_radians();
    let (s, c) = theta.sin_cos();
    let rotate = matrix::Matrix::new_2d(vec![
        vec![c, -s, 0.0],
        vec![s, c, 0.0],
        vec![0.0, 0.0, 1.0],
    ]);
    let to_origin = matrix::Matrix::new_2d(vec![
        vec![1.0, 0.0, -src_cx],
        vec![0.0, 1.0, -src_cy],
        vec![0.0, 0.0, 1.0],
    ]);
    let to_dest = matrix::Matrix::new_2d(vec![
        vec![1.0, 0.0, dest_x],
        vec![0.0, 1.0, dest_y],
        vec![0.0, 0.0, 1.0],
    ]);
    to_dest
        .dot(&rotate)
        .expect("3x3 dot 3x3")
        .dot(&to_origin)
        .expect("3x3 dot 3x3")
}

/// Render a real QR code, then synthesize a "photo" of it: rotate by
/// `angle_deg` and place it centered in a `photo_size` x `photo_size`
/// canvas via `perspective_warp` itself. `perspective_warp`'s `h` is
/// the *inverse* of the forward transform -- `matrix::Matrix::invert`
/// (a dev-dependency only) computes that inverse directly, rather than
/// this test deriving it by hand. `OutOfBounds::Replicate` (not
/// `Zero`) gives the area outside the rotated code a plausible uniform
/// light background for free: the clean render's quiet-zone border is
/// already white, so edge-replication just extends it.
fn synthesize_photo(clean: &PixelContainer, angle_deg: f64, photo_size: u32) -> PixelContainer {
    let src_cx = clean.width as f64 / 2.0;
    let src_cy = clean.height as f64 / 2.0;
    let dest = photo_size as f64 / 2.0;
    let forward = build_forward(dest, dest, angle_deg, src_cx, src_cy);
    let h64 = forward
        .invert()
        .expect("a rotation + translation is always invertible");
    let h32 = to_f32(&h64);
    perspective_warp(
        clean,
        h32,
        photo_size,
        photo_size,
        Interpolation::Bilinear,
        OutOfBounds::Replicate,
    )
}

fn render_clean_qr(payload: &str) -> PixelContainer {
    let grid = encode(payload, EccLevel::M).expect("encode should succeed for a short ASCII payload");
    let config = Barcode2DLayoutConfig::default();
    let scene = layout(&grid, &config).expect("layout should succeed");
    render_scene(&scene).expect("render should succeed")
}

fn photo_for(payload: &str, angle_deg: f64) -> PixelContainer {
    let clean = render_clean_qr(payload);
    let photo_size = ((clean.width.max(clean.height) as f64) * 1.6) as u32;
    synthesize_photo(&clean, angle_deg, photo_size)
}

// ---------------------------------------------------------------------
// Synthetic end-to-end: photo in, original payload out
// ---------------------------------------------------------------------

#[test]
fn locate_and_decode_recovers_payload_unrotated() {
    let payload = "HELLO WORLD";
    let photo = photo_for(payload, 0.0);
    let decoded = locate_and_decode(&photo).expect("should locate and decode");
    assert_eq!(decoded, payload.as_bytes());
}

#[test]
fn locate_and_decode_recovers_payload_at_90_degrees() {
    let payload = "ROTATE90";
    let photo = photo_for(payload, 90.0);
    let decoded = locate_and_decode(&photo).expect("should locate and decode");
    assert_eq!(decoded, payload.as_bytes());
}

#[test]
fn locate_and_decode_recovers_payload_at_an_arbitrary_angle() {
    // Exercises both the chirality-retry logic (§4.5) and non-axis-
    // aligned geometry for real, not just as a compiled-but-untested
    // code path.
    let payload = "SKEW17DEG";
    let photo = photo_for(payload, 17.0);
    let decoded = locate_and_decode(&photo).expect("should locate and decode");
    assert_eq!(decoded, payload.as_bytes());
}

#[test]
fn locate_and_decode_recovers_payload_at_180_degrees() {
    let payload = "UPSIDEDOWN";
    let photo = photo_for(payload, 180.0);
    let decoded = locate_and_decode(&photo).expect("should locate and decode");
    assert_eq!(decoded, payload.as_bytes());
}

// ---------------------------------------------------------------------
// Degenerate / failure input: documented errors, never a panic
// ---------------------------------------------------------------------

#[test]
fn locate_and_decode_blank_image_fails_cleanly() {
    let mut img = PixelContainer::new(200, 200);
    img.fill(255, 255, 255, 255);
    assert_eq!(
        locate_and_decode(&img),
        Err(QrLocateError::NoFinderPatternTriangleFound)
    );
}

#[test]
fn locate_and_decode_all_dark_image_fails_cleanly() {
    let mut img = PixelContainer::new(200, 200);
    img.fill(0, 0, 0, 255);
    // No panic, and no spurious success -- an all-dark image has no
    // real finder-pattern structure to find.
    assert!(locate_and_decode(&img).is_err());
}

#[test]
fn locate_and_decode_tiny_image_does_not_panic() {
    for (w, h) in [(0, 0), (1, 1), (3, 3), (10, 10)] {
        let img = PixelContainer::new(w, h);
        let _ = locate_and_decode(&img);
    }
}

#[test]
fn locate_and_decode_photo_like_noise_without_a_qr_code_fails_cleanly() {
    // A checkerboard-ish pattern with real dark/light structure but no
    // finder patterns -- distinct from the pure-noise/blank cases above.
    let mut img = PixelContainer::new(150, 150);
    img.fill(255, 255, 255, 255);
    for y in 0..150u32 {
        for x in 0..150u32 {
            if (x / 11 + y / 13) % 2 == 0 {
                img.set_pixel(x, y, 30, 30, 30, 255);
            }
        }
    }
    let _ = locate_and_decode(&img); // must not panic; outcome not asserted
}

// ---------------------------------------------------------------------
// Bounded cost on dense, pathological input (MAX_RAW_CANDIDATES /
// MAX_REFINED_CANDIDATES exist to keep this from hanging or blowing up)
// ---------------------------------------------------------------------

#[test]
fn locate_and_decode_dense_striped_bitmap_completes_without_hanging_or_panicking() {
    // Vertical 3-dark/3-light stripes across the whole width -- close
    // to the finder pattern's own 1:1:3:1:1 ratio family on nearly
    // every scanline, the kind of density MAX_RAW_CANDIDATES and
    // MAX_REFINED_CANDIDATES exist to bound the cost of. This test's
    // purpose is proving completion and no panic, not a specific
    // Ok/Err outcome.
    let size = 300u32;
    let mut img = PixelContainer::new(size, size);
    img.fill(255, 255, 255, 255);
    for y in 0..size {
        for x in 0..size {
            if (x / 3) % 2 == 0 {
                img.set_pixel(x, y, 0, 0, 0, 255);
            }
        }
    }
    let _ = locate_and_decode(&img);
}
