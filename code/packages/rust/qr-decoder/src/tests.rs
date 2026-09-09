//! # qr-decoder Test Suite
//!
//! Implements MA04-qr-decoder.md §7's full test strategy. Because
//! `qr_code::encode` already exists and is independently, thoroughly
//! tested, this suite's primary verification strategy is round-tripping
//! against it — encode with the real encoder, decode with this crate,
//! compare against the original input. Corruption tests flip real grid
//! *modules* (not internal data structures) via [`corrupt_codeword`], which
//! works regardless of which mask the encoder happened to choose: flipping
//! a physical module always flips the corresponding *unmasked* logical bit
//! exactly once, so XOR-ing specific bits of a target codeword is
//! mask-invariant.
//!
//! Section numbers below refer to MA04-qr-decoder.md §7's numbered list.

use super::*;
use qr_code::{encode, EccLevel};

/// `ModuleGrid` (defined in `barcode-2d`) does not derive `Clone`, so tests
/// that need to corrupt a copy of an encoder-produced grid build one by
/// hand field-by-field instead.
fn clone_grid(grid: &ModuleGrid) -> ModuleGrid {
    ModuleGrid {
        rows: grid.rows,
        cols: grid.cols,
        modules: grid.modules.clone(),
        module_shape: grid.module_shape.clone(),
    }
}

// =============================================================================
// Test input generators — deterministic, no external randomness needed.
// =============================================================================

/// `len` bytes of cycling printable ASCII (`!`..`~`, 94 values), guaranteed
/// to trigger Byte mode (the set includes characters outside qr-code's
/// 45-character alphanumeric alphabet, e.g. lowercase letters and `!`).
/// All bytes are single-byte-UTF8-safe ASCII, so `str::len()` (bytes) and
/// `chars().count()` agree — important since Byte mode's char-count field
/// is a *byte* length.
fn byte_mode_input(len: usize) -> String {
    (0..len).map(|i| (b'!' + (i % 94) as u8) as char).collect()
}

/// `len` decimal digits, cycling 0-9.
fn numeric_input(len: usize) -> String {
    (0..len).map(|i| char::from_digit((i % 10) as u32, 10).unwrap()).collect()
}

/// `len` characters cycling through qr-code's 45-character alphanumeric set.
fn alphanumeric_input(len: usize) -> String {
    const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ $%*+-./:";
    (0..len).map(|i| CHARS[i % CHARS.len()] as char).collect()
}

/// The largest byte-mode message length that `select_version` will place at
/// exactly `version` for `ecc`, accounting for the mode-indicator (4 bits)
/// and character-count (8 bits for v<=9, 16 otherwise) header overhead —
/// `qr_code::num_data_codewords(version, ecc)` alone is the *codeword*
/// capacity, not the raw message-byte capacity; the header eats into it.
/// Getting this wrong makes `encode` silently pick a *larger* version than
/// intended, which was the root cause of several early test failures here
/// (an oversized "capacity" input for v1-H, for instance, actually got
/// encoded at v2).
fn byte_capacity(version: usize, ecc: EccLevel) -> usize {
    let capacity_codewords = qr_code::num_data_codewords(version, ecc);
    let header_bits = 4 + if version <= 9 { 8 } else { 16 };
    let available_bits = capacity_codewords * 8 - header_bits;
    available_bits / 8
}

// =============================================================================
// Corruption helpers — flip grid modules at the positions corresponding to
// a specific *interleaved* codeword index, so tests can corrupt "codeword
// k" or "block b's j-th data/ecc byte" without needing access to qr-code's
// private block-construction internals.
// =============================================================================

/// Flip whichever of interleaved-codeword `codeword_index`'s 8 grid modules
/// correspond to the `1` bits of `xor_mask`. Mask-invariant (see module
/// doc): flipping a physical module always flips the same logical
/// (unmasked) bit regardless of which of the 8 mask patterns the encoder
/// picked, so this reliably reproduces `codeword[k] ^= xor_mask` on the
/// decoded byte.
fn corrupt_codeword(grid: &mut ModuleGrid, version: usize, codeword_index: usize, xor_mask: u8) {
    let order = qr_code::data_module_order(version);
    for j in 0u32..8 {
        if (xor_mask >> (7 - j)) & 1 == 1 {
            let (r, c) = order[codeword_index * 8 + j as usize];
            grid.modules[r][c] = !grid.modules[r][c];
        }
    }
}

/// Compute the interleaved-codeword index for a given block's `byte_idx`-th
/// data byte (`in_ecc = false`) or ECC byte (`in_ecc = true`). Mirrors the
/// exact same block-size arithmetic and round-robin traversal `decode`'s
/// own de-interleave step (and, by construction, qr-code's own
/// `compute_blocks`/`interleave_blocks`) uses — this is test-setup
/// plumbing to locate where to corrupt, not a duplicate implementation
/// being tested (decode's own de-interleave correctness is independently
/// proven by every round-trip test above, since a bug there would corrupt
/// *every* multi-block round trip, not just these targeted tests).
fn interleaved_index(version: usize, ecc: EccLevel, block: usize, in_ecc: bool, byte_idx: usize) -> usize {
    let total_data = qr_code::num_data_codewords(version, ecc);
    let total_blocks = qr_code::num_blocks(ecc, version);
    let ecc_len = qr_code::ecc_codewords_per_block(ecc, version);
    let short_len = total_data / total_blocks;
    let num_long = total_data % total_blocks;
    let g1_count = total_blocks - num_long;
    let block_data_len = |b: usize| if b < g1_count { short_len } else { short_len + 1 };
    let max_data = if num_long > 0 { short_len + 1 } else { short_len };

    let mut cursor = 0usize;
    if !in_ecc {
        for i in 0..max_data {
            for b in 0..total_blocks {
                if i < block_data_len(b) {
                    if b == block && i == byte_idx {
                        return cursor;
                    }
                    cursor += 1;
                }
            }
        }
        panic!("data byte_idx {byte_idx} out of range for block {block} at v{version}/{ecc:?}");
    } else {
        for i in 0..max_data {
            for b in 0..total_blocks {
                if i < block_data_len(b) {
                    cursor += 1;
                }
            }
        }
        for i in 0..ecc_len {
            for b in 0..total_blocks {
                if b == block && i == byte_idx {
                    return cursor;
                }
                cursor += 1;
            }
        }
        panic!("ecc byte_idx {byte_idx} out of range for block {block} at v{version}/{ecc:?}");
    }
}

// =============================================================================
// 1. Exhaustive round-trip (spec §7 item 1, §8 gate 3)
// =============================================================================

/// Byte mode, every version 1-40 x every ECC level: 160 cases, each sized
/// to the *exact* byte capacity of its (version, ecc) pair so `encode`'s
/// own `select_version` is forced to pick that exact version — verified by
/// asserting `grid.rows == qr_code::symbol_size(version)` before decoding.
/// This is the single strongest correctness signal in this suite: every
/// stage (format read, version read for v>=7, unmask, traversal,
/// de-interleave, RS decode, mode decode) must be simultaneously correct
/// for even one of these 160 cases to pass.
#[test]
fn exhaustive_byte_mode_round_trip_all_versions_all_ecc() {
    for version in 1..=40usize {
        for ecc in [EccLevel::L, EccLevel::M, EccLevel::Q, EccLevel::H] {
            let input = byte_mode_input(byte_capacity(version, ecc));
            let grid = encode(&input, ecc)
                .unwrap_or_else(|e| panic!("encode failed at v{version}/{ecc:?}: {e}"));
            assert_eq!(
                grid.rows as usize,
                qr_code::symbol_size(version),
                "input sized for v{version}/{ecc:?} selected a different version (rows={})",
                grid.rows
            );
            let decoded = decode(&grid)
                .unwrap_or_else(|e| panic!("decode failed at v{version}/{ecc:?}: {e:?}"));
            assert_eq!(
                decoded,
                input.as_bytes(),
                "round-trip mismatch at v{version}/{ecc:?}"
            );
        }
    }
}

/// Numeric mode across a spread of versions (low/mid/high) x every ECC
/// level. Unlike byte mode, numeric's 3-digits-per-10-bits packing makes
/// exact-version targeting fragile to hand-compute, so this samples input
/// lengths designed to land in different version tiers rather than
/// asserting an exact version per case — still a real, meaningful
/// round-trip through every pipeline stage, just without the "hit exactly
/// this version" guarantee the byte-mode sweep above provides exhaustively.
#[test]
fn numeric_mode_round_trip_representative_versions() {
    let mut versions_seen = std::collections::HashSet::new();
    for len in [1usize, 5, 15, 41, 77, 187, 500, 1200, 2500, 4500, 7089] {
        for ecc in [EccLevel::L, EccLevel::M, EccLevel::Q, EccLevel::H] {
            let input = numeric_input(len);
            let Ok(grid) = encode(&input, ecc) else { continue };
            let version = (grid.rows as usize - 17) / 4;
            versions_seen.insert(version);
            let decoded = decode(&grid)
                .unwrap_or_else(|e| panic!("decode failed for numeric len={len}/{ecc:?}: {e:?}"));
            assert_eq!(decoded, input.as_bytes(), "numeric round-trip mismatch len={len}/{ecc:?}");
        }
    }
    assert!(versions_seen.contains(&1), "numeric sweep never hit version 1");
    assert!(
        *versions_seen.iter().max().unwrap() >= 35,
        "numeric sweep never reached a high version, got max {:?}",
        versions_seen.iter().max()
    );
}

/// Alphanumeric mode, same sampling strategy as the numeric sweep above.
#[test]
fn alphanumeric_mode_round_trip_representative_versions() {
    let mut versions_seen = std::collections::HashSet::new();
    for len in [1usize, 4, 12, 33, 67, 154, 400, 900, 1800, 3200, 4296] {
        for ecc in [EccLevel::L, EccLevel::M, EccLevel::Q, EccLevel::H] {
            let input = alphanumeric_input(len);
            let Ok(grid) = encode(&input, ecc) else { continue };
            let version = (grid.rows as usize - 17) / 4;
            versions_seen.insert(version);
            let decoded = decode(&grid)
                .unwrap_or_else(|e| panic!("decode failed for alphanumeric len={len}/{ecc:?}: {e:?}"));
            assert_eq!(decoded, input.as_bytes(), "alphanumeric round-trip mismatch len={len}/{ecc:?}");
        }
    }
    assert!(versions_seen.contains(&1), "alphanumeric sweep never hit version 1");
    assert!(
        *versions_seen.iter().max().unwrap() >= 35,
        "alphanumeric sweep never reached a high version, got max {:?}",
        versions_seen.iter().max()
    );
}

/// The actual driving use case: an `otpauth://totp/...` URI. Contains
/// lowercase letters and `?`/`=`/`&`, none in the 45-char alphanumeric set,
/// so qr-code's own mode-selection heuristic always picks Byte mode for it
/// (MA04-qr-decoder.md §4.4).
#[test]
fn otpauth_uri_round_trip_all_ecc_levels() {
    let uri = "otpauth://totp/Example:alice@google.com?secret=JBSWY3DPEHPK3PXP&issuer=Example";
    for ecc in [EccLevel::L, EccLevel::M, EccLevel::Q, EccLevel::H] {
        let grid = encode(uri, ecc).unwrap();
        let decoded = decode(&grid).unwrap();
        assert_eq!(decoded, uri.as_bytes());
    }
}

#[test]
fn empty_string_round_trip() {
    let grid = encode("", EccLevel::M).unwrap();
    let decoded = decode(&grid).unwrap();
    assert_eq!(decoded, b"");
}

#[test]
fn single_character_round_trips_every_mode() {
    for (input, ecc) in [("5", EccLevel::L), ("A", EccLevel::M), ("z", EccLevel::Q)] {
        let grid = encode(input, ecc).unwrap();
        assert_eq!(decode(&grid).unwrap(), input.as_bytes());
    }
}

// =============================================================================
// 2. Corruption-and-recovery within RS capacity (spec §7 item 2, §8 gate 4)
// =============================================================================

/// Single-block version (v1 always has exactly one RS block): corrupt
/// exactly `t = ecc_len/2` bytes within that block's data extent, assert
/// the original message is still recovered. This is the test class that
/// specifically proves `decode_with_base`'s b=0 wiring is correct — a
/// wrong root convention can still round-trip perfectly on undamaged input
/// (zero errors exercises none of the syndrome/Forney machinery), so only
/// induced-error recovery actually proves it.
#[test]
fn corruption_recovery_single_block_v1() {
    let ecc = EccLevel::H; // v1-H: ecc_len=17, t=8 -- plenty of room to corrupt
    let version = 1usize;
    let input = byte_mode_input(byte_capacity(version, ecc));
    let grid = encode(&input, ecc).unwrap();
    assert_eq!(grid.rows, 21, "expected version 1");
    assert_eq!(qr_code::num_blocks(ecc, version), 1, "expected exactly one block");

    let ecc_len = qr_code::ecc_codewords_per_block(ecc, version);
    let t = ecc_len / 2;

    let mut corrupted_grid = clone_grid(&grid);
    for k in 0..t {
        let idx = interleaved_index(version, ecc, 0, false, k);
        corrupt_codeword(&mut corrupted_grid, version, idx, 0x5A ^ (k as u8));
    }
    let decoded = decode(&corrupted_grid).unwrap();
    assert_eq!(decoded, input.as_bytes());
}

/// Multi-block version: corrupt `t` bytes within a single specific block
/// (not spread across blocks, which could exceed any one block's own
/// capacity even while total corruption is modest), assert recovery.
#[test]
fn corruption_recovery_single_block_within_multiblock_version() {
    let ecc = EccLevel::Q;
    let version = 5usize;
    assert!(qr_code::num_blocks(ecc, version) > 1, "expected multiple blocks at v5/Q");

    let input = byte_mode_input(byte_capacity(version, ecc));
    let grid = encode(&input, ecc).unwrap();
    assert_eq!(grid.rows as usize, qr_code::symbol_size(version));

    let target_block = 2usize;
    let ecc_len = qr_code::ecc_codewords_per_block(ecc, version);
    let t = ecc_len / 2;

    let mut corrupted_grid = clone_grid(&grid);
    for k in 0..t {
        let idx = interleaved_index(version, ecc, target_block, false, k);
        corrupt_codeword(&mut corrupted_grid, version, idx, 0x33 ^ ((k as u8) << 1));
    }
    let decoded = decode(&corrupted_grid).unwrap();
    assert_eq!(decoded, input.as_bytes());
}

/// Corrupt ECC bytes (not data bytes) within capacity — errors in the
/// check bytes must be just as correctable as errors in the data bytes.
#[test]
fn corruption_recovery_in_ecc_bytes() {
    let ecc = EccLevel::M;
    let version = 3usize;
    let input = byte_mode_input(byte_capacity(version, ecc));
    let grid = encode(&input, ecc).unwrap();
    assert_eq!(grid.rows as usize, qr_code::symbol_size(version));

    let ecc_len = qr_code::ecc_codewords_per_block(ecc, version);
    let t = ecc_len / 2;

    let mut corrupted_grid = clone_grid(&grid);
    for k in 0..t {
        let idx = interleaved_index(version, ecc, 0, true, k);
        corrupt_codeword(&mut corrupted_grid, version, idx, 0x77);
    }
    let decoded = decode(&corrupted_grid).unwrap();
    assert_eq!(decoded, input.as_bytes());
}

// =============================================================================
// 3. Beyond-capacity corruption (spec §7 item 3, §8 gate 4)
// =============================================================================

#[test]
fn beyond_capacity_corruption_rejected_not_silently_wrong() {
    let ecc = EccLevel::H;
    let version = 1usize;
    let input = byte_mode_input(byte_capacity(version, ecc));
    let grid = encode(&input, ecc).unwrap();
    assert_eq!(grid.rows as usize, qr_code::symbol_size(version));
    let ecc_len = qr_code::ecc_codewords_per_block(ecc, version);
    let t = ecc_len / 2;

    // Corrupt t+1 distinct data bytes -- one more than this block's
    // correction capacity.
    let mut corrupted_grid = clone_grid(&grid);
    let data_len = qr_code::num_data_codewords(version, ecc);
    let corrupt_count = (t + 1).min(data_len);
    for k in 0..corrupt_count {
        let idx = interleaved_index(version, ecc, 0, false, k);
        corrupt_codeword(&mut corrupted_grid, version, idx, 0xE7 ^ (k as u8));
    }
    let result = decode(&corrupted_grid);
    assert_eq!(
        result,
        Err(QrDecodeError::UnrecoverableBlock(0)),
        "expected UnrecoverableBlock(0) for {corrupt_count}-byte corruption (t={t}), got {result:?}"
    );
}

// =============================================================================
// 4. Format-info Copy-2 fallback (spec §7 item 4, §8 gate 5)
// =============================================================================

#[test]
fn format_info_copy2_fallback_when_copy1_corrupted() {
    let grid = encode("HELLO WORLD", EccLevel::M).unwrap();
    let mut corrupted = clone_grid(&grid);
    // Corrupt a handful of Copy 1 format-info modules directly -- bypassing
    // the encoder entirely, per spec §7 item 4 ("directly in a
    // ModuleGrid"). Deliberately flips only 3 of the 15 bits, not all of
    // them: this (15,5) BCH code is linear, and flipping *every* bit of a
    // valid codeword happens to land on another valid (but different)
    // codeword (confirmed empirically -- see qr-code's own
    // read_format_info_rejects_corrupted_word test comment) -- which would
    // make Copy 1 falsely validate with the WRONG (ecc, mask) instead of
    // failing, defeating the point of this test. 3 bit flips reliably
    // exceeds this code's 3-bit correction distance without landing on
    // another codeword.
    for &(r, c) in FORMAT_COPY_1[0..3].iter() {
        corrupted.modules[r][c] = !corrupted.modules[r][c];
    }
    let decoded = decode(&corrupted).expect("expected Copy 2 fallback to succeed");
    assert_eq!(decoded, b"HELLO WORLD");
}

#[test]
fn format_info_unreadable_when_both_copies_corrupted() {
    let grid = encode("HELLO WORLD", EccLevel::M).unwrap();
    let sz = grid.rows as usize;
    let mut corrupted = clone_grid(&grid);
    // Same rationale as the Copy-2-fallback test above: flip a handful of
    // bits per copy (not all 15), to genuinely break BCH validation on
    // both rather than coincidentally landing on another valid codeword.
    for &(r, c) in FORMAT_COPY_1[0..3].iter() {
        corrupted.modules[r][c] = !corrupted.modules[r][c];
    }
    for &(r, c) in format_copy_2(sz)[0..3].iter() {
        corrupted.modules[r][c] = !corrupted.modules[r][c];
    }
    assert_eq!(decode(&corrupted), Err(QrDecodeError::FormatInfoUnreadable));
}

// =============================================================================
// 5. Version-info mismatch (spec §7 item 5, §8 gate 5)
// =============================================================================

#[test]
fn version_info_unreadable_when_both_copies_corrupted() {
    let input: String = "A".repeat(85); // forces v7+
    let grid = encode(&input, EccLevel::H).unwrap();
    let version = (grid.rows as usize - 17) / 4;
    assert!(version >= 7, "expected a version >= 7 grid, got v{version}");
    let sz = grid.rows as usize;

    let mut corrupted = clone_grid(&grid);
    for i in 0u32..18 {
        let a = 5 - (i / 3) as usize;
        let b = sz - 9 - (i % 3) as usize;
        corrupted.modules[a][b] = !corrupted.modules[a][b]; // Copy 1
        corrupted.modules[b][a] = !corrupted.modules[b][a]; // Copy 2
    }
    assert_eq!(decode(&corrupted), Err(QrDecodeError::VersionInfoUnreadable));
}

#[test]
fn version_info_disagreeing_with_grid_size_rejected() {
    // Both version-info copies BCH-valid but encoding a version that
    // disagrees with the grid's actual (size-derived) version -- a real,
    // if redundant, consistency check ISO provides for exactly this
    // purpose (MA04-qr-decoder.md §4.2 step 4).
    let input: String = "A".repeat(85);
    let grid = encode(&input, EccLevel::H).unwrap();
    let actual_version = (grid.rows as usize - 17) / 4;
    let wrong_version = if actual_version == 7 { 10 } else { 7 };

    // Compute a valid BCH word for `wrong_version` using the same generator
    // qr-code's own compute_version_bits uses (0x1F25), independently here
    // (test-local, not calling qr-code's private function) so this test
    // doesn't depend on that function's visibility.
    let mut rem = (wrong_version as u32) << 12;
    for i in (12u32..=17).rev() {
        if (rem >> i) & 1 == 1 {
            rem ^= 0x1f25 << (i - 12);
        }
    }
    let bits18 = ((wrong_version as u32) << 12) | (rem & 0xfff);

    let sz = grid.rows as usize;
    let mut corrupted = clone_grid(&grid);
    for i in 0u32..18 {
        let a = 5 - (i / 3) as usize;
        let b = sz - 9 - (i % 3) as usize;
        let dark = (bits18 >> i) & 1 == 1;
        corrupted.modules[a][b] = dark;
        corrupted.modules[b][a] = dark;
    }
    assert_eq!(decode(&corrupted), Err(QrDecodeError::VersionInfoUnreadable));
}

// =============================================================================
// 6. InvalidSize (spec §7 item 6, §8 gate 5)
// =============================================================================

#[test]
fn invalid_size_non_square_grid() {
    let grid = ModuleGrid {
        rows: 21,
        cols: 25,
        modules: vec![vec![false; 25]; 21],
        module_shape: barcode_2d::ModuleShape::Square,
    };
    assert_eq!(decode(&grid), Err(QrDecodeError::InvalidSize));
}

#[test]
fn invalid_size_square_but_not_4v_plus_17() {
    // 22 is square but (22-17) is not a multiple of 4.
    let grid = ModuleGrid {
        rows: 22,
        cols: 22,
        modules: vec![vec![false; 22]; 22],
        module_shape: barcode_2d::ModuleShape::Square,
    };
    assert_eq!(decode(&grid), Err(QrDecodeError::InvalidSize));
}

#[test]
fn invalid_size_too_small() {
    let grid = ModuleGrid {
        rows: 5,
        cols: 5,
        modules: vec![vec![false; 5]; 5],
        module_shape: barcode_2d::ModuleShape::Square,
    };
    assert_eq!(decode(&grid), Err(QrDecodeError::InvalidSize));
}

#[test]
fn invalid_size_too_large_beyond_v40() {
    let sz = qr_code::symbol_size(40) + 4; // one version step beyond v40
    let grid = ModuleGrid {
        rows: sz as u32,
        cols: sz as u32,
        modules: vec![vec![false; sz]; sz],
        module_shape: barcode_2d::ModuleShape::Square,
    };
    assert_eq!(decode(&grid), Err(QrDecodeError::InvalidSize));
}

#[test]
fn invalid_size_ragged_modules_vec_does_not_panic() {
    // A hand-built ModuleGrid whose `modules` field doesn't actually match
    // its declared rows/cols -- gate 6 requires no panic here, just a clean
    // error, since ModuleGrid is publicly hand-constructible.
    let grid = ModuleGrid {
        rows: 21,
        cols: 21,
        modules: vec![vec![false; 21]; 3], // only 3 rows, not 21
        module_shape: barcode_2d::ModuleShape::Square,
    };
    assert_eq!(decode(&grid), Err(QrDecodeError::InvalidSize));
}

// =============================================================================
// 9. Direct encoder/decoder wire-format cross-check (spec §7 item 9)
// =============================================================================
//
// qr-code's real RS encoder (build_generator/rs_encode) is private, so this
// can't call it directly from another crate. Instead this achieves the same
// goal -- proving reed_solomon::decode_with_base(..., 0) actually agrees
// with qr-code's own (independently written) b=0 RS encoder on wire format
// -- indirectly but just as rigorously: encode a real grid with
// qr_code::encode, corrupt specific codeword bytes at the *module* level
// (not by hand-constructing a codeword), and decode via the full
// qr_decoder::decode pipeline, which internally calls
// reed_solomon::decode_with_base on exactly the bytes qr-code's real
// encoder produced. This is a stronger cross-check than calling the private
// functions directly would be, since it also exercises the surrounding
// traversal/de-interleave logic against the same real encoder output.

#[test]
fn direct_cross_check_qr_code_encoder_vs_reed_solomon_decoder() {
    for (version, ecc) in [(1usize, EccLevel::L), (2, EccLevel::M), (5, EccLevel::Q), (10, EccLevel::H)] {
        let input = byte_mode_input(byte_capacity(version, ecc));
        let grid = encode(&input, ecc).unwrap();
        assert_eq!(grid.rows as usize, qr_code::symbol_size(version));

        let ecc_len = qr_code::ecc_codewords_per_block(ecc, version);
        let t = ecc_len / 2;
        let mut corrupted = clone_grid(&grid);
        for k in 0..t {
            let idx = interleaved_index(version, ecc, 0, false, k);
            corrupt_codeword(&mut corrupted, version, idx, 0x9C ^ (k as u8 * 7));
        }
        let decoded = decode(&corrupted)
            .unwrap_or_else(|e| panic!("cross-check failed at v{version}/{ecc:?}: {e:?}"));
        assert_eq!(decoded, input.as_bytes());
    }
}

// =============================================================================
// 5 (§8 gate 5). Every QrDecodeError variant gets its own dedicated test —
// most are covered above under their spec-§7 item; UnsupportedMode and
// Truncated need direct hand-built-grid tests since qr-code's own encoder
// never produces Kanji/ECI mode or a truncated stream.
// =============================================================================

/// Build a minimal, otherwise-valid v1-M grid whose data segment's mode
/// indicator is overwritten to a Kanji/ECI value, to exercise
/// `UnsupportedMode` directly (qr-code's own encoder never emits these).
#[test]
fn unsupported_mode_kanji_indicator_rejected() {
    // Corrupting a real encoded grid's mode-indicator bits doesn't work as
    // a test strategy here: those bits live inside an RS-protected
    // codeword, and a small corruption (within the block's correction
    // capacity) gets silently *repaired* by Reed-Solomon before mode
    // parsing ever sees it -- confirmed empirically (flipping codeword 0's
    // mode nibble alone is just 1 byte of damage, trivially within any
    // real block's t>=1 correction capacity, so decode still succeeds with
    // the ORIGINAL mode). qr-code's own encoder can never legitimately
    // produce a Kanji/ECI mode indicator in the first place, so there's no
    // way to reach UnsupportedMode via the public encode+decode round trip
    // at all -- this directly unit-tests the private parse_data_segment
    // function instead, which is the only way to exercise this path.
    let data = [0b1000_0000u8]; // mode=Kanji(1000), rest irrelevant
    let result = parse_data_segment(&data, 1);
    assert!(
        matches!(result, Err(QrDecodeError::UnsupportedMode(0b1000))),
        "expected UnsupportedMode(0b1000), got {result:?}"
    );
}

#[test]
fn unsupported_mode_eci_indicator_rejected() {
    let data = [0b0111_0000u8]; // mode=ECI(0111)
    let result = parse_data_segment(&data, 1);
    assert!(
        matches!(result, Err(QrDecodeError::UnsupportedMode(0b0111))),
        "expected UnsupportedMode(0b0111), got {result:?}"
    );
}

/// A declared byte-mode character count that exceeds what the data stream
/// actually supplies must fail with `Truncated`, not panic or return
/// partial/garbage data. Same rationale as the mode tests above for going
/// straight to `parse_data_segment`: corrupting a real RS-protected grid's
/// character-count field within any single block's correction capacity
/// gets silently repaired before parsing ever sees the corruption.
#[test]
fn truncated_when_declared_length_exceeds_available_data() {
    // Mode=Byte (0100), then an 8-bit count of 0xFF (255 bytes claimed),
    // but only 2 actual data bytes follow -- nowhere near enough.
    let data = [0b0100_1111u8, 0b1111_0000u8, 0xAA, 0xBB];
    let result = parse_data_segment(&data, 1);
    assert!(
        matches!(result, Err(QrDecodeError::Truncated)),
        "expected Truncated for an over-claimed byte count, got {result:?}"
    );
}

#[test]
fn truncated_when_mode_indicator_itself_is_cut_off() {
    // Zero bytes at all -- not even a full 4-bit mode indicator available.
    let result = parse_data_segment(&[], 1);
    assert_eq!(result, Err(QrDecodeError::Truncated));
}

// =============================================================================
// Miscellaneous robustness
// =============================================================================

#[test]
fn decode_is_deterministic() {
    let grid = encode("determinism check", EccLevel::M).unwrap();
    assert_eq!(decode(&grid).unwrap(), decode(&grid).unwrap());
}

#[test]
fn qr_decode_error_display_is_non_empty_for_every_variant() {
    let variants = [
        QrDecodeError::InvalidSize,
        QrDecodeError::FormatInfoUnreadable,
        QrDecodeError::VersionInfoUnreadable,
        QrDecodeError::UnrecoverableBlock(3),
        QrDecodeError::UnsupportedMode(0b1000),
        QrDecodeError::Truncated,
    ];
    for v in variants {
        assert!(!v.to_string().is_empty());
    }
}

#[test]
fn unrecoverable_block_reports_correct_index_in_multiblock_grid() {
    let ecc = EccLevel::Q;
    let version = 5usize;
    assert!(qr_code::num_blocks(ecc, version) > 2);
    let input = byte_mode_input(byte_capacity(version, ecc));
    let grid = encode(&input, ecc).unwrap();
    assert_eq!(grid.rows as usize, qr_code::symbol_size(version));

    let ecc_len = qr_code::ecc_codewords_per_block(ecc, version);
    let t = ecc_len / 2;
    let target_block = 1usize;

    let mut corrupted = clone_grid(&grid);
    // t+1 errors in block 1 -- exceeds capacity there specifically.
    for k in 0..=t {
        let idx = interleaved_index(version, ecc, target_block, false, k);
        corrupt_codeword(&mut corrupted, version, idx, 0xC3 ^ (k as u8));
    }
    assert_eq!(decode(&corrupted), Err(QrDecodeError::UnrecoverableBlock(target_block)));
}

/// Bit-reader edge case: an all-false (empty/blank) grid at the smallest
/// valid size must not panic, and should decode as the "0000 = empty
/// message" case (format info will actually be unreadable for an
/// all-false grid, since format info's fixed XOR mask 0x5412 makes an
/// all-false raw read fail BCH validation -- verifying that produces a
/// clean error, not a panic, is the actual point of this test).
#[test]
fn all_false_grid_does_not_panic() {
    let sz = qr_code::symbol_size(1);
    let grid = ModuleGrid {
        rows: sz as u32,
        cols: sz as u32,
        modules: vec![vec![false; sz]; sz],
        module_shape: barcode_2d::ModuleShape::Square,
    };
    // Must return some Err, not panic -- the specific variant isn't the
    // point (format info on an all-false grid is essentially guaranteed
    // to fail BCH validation).
    let _ = decode(&grid);
}
