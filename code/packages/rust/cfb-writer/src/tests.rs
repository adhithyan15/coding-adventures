//! Tests for `cfb-writer`. The centrepiece is the **round-trip**: we write CFB
//! bytes, reopen them with the sibling `cfb` reader, and assert byte-for-byte
//! equality of every stream. That is the proof our writer produces valid files.

use super::*;

// ---------------------------------------------------------------------------
// The required round-trip: small (mini-FAT) + large (regular FAT) together.
// ---------------------------------------------------------------------------

#[test]
fn round_trip_mixed_small_and_large() {
    let bytes = write_cfb(&[
        ("Workbook", &vec![0xABu8; 5000][..]), // large -> regular FAT
        ("SmallStream", &b"hello mini-stream"[..]), // small -> mini-FAT
        ("Another", &vec![0x01u8; 100][..]),   // small
    ]);

    let cf = cfb::CompoundFile::open(&bytes).expect("our own reader must open our output");
    let mut names = cf.stream_names();
    names.sort();
    assert!(names.iter().any(|n| n == "Workbook"));
    assert!(names.iter().any(|n| n == "SmallStream"));
    assert!(names.iter().any(|n| n == "Another"));

    assert_eq!(cf.read_stream("Workbook").unwrap(), vec![0xABu8; 5000]);
    assert_eq!(
        cf.read_stream("SmallStream").unwrap(),
        b"hello mini-stream".to_vec()
    );
    assert_eq!(cf.read_stream("Another").unwrap(), vec![0x01u8; 100]);
}

// ---------------------------------------------------------------------------
// Header sanity — the signature and version fields must be exactly right.
// ---------------------------------------------------------------------------

#[test]
fn header_signature_and_version_fields() {
    let bytes = write_cfb(&[("Only", &b"x"[..])]);
    assert_eq!(&bytes[0..8], &SIGNATURE);
    // major version 3, sector shift 0x0009, mini shift 0x0006, cutoff 4096.
    assert_eq!(u16::from_le_bytes([bytes[26], bytes[27]]), 0x0003);
    assert_eq!(u16::from_le_bytes([bytes[30], bytes[31]]), 0x0009);
    assert_eq!(u16::from_le_bytes([bytes[32], bytes[33]]), 0x0006);
    assert_eq!(u16::from_le_bytes([bytes[28], bytes[29]]), 0xFFFE);
    assert_eq!(
        u32::from_le_bytes([bytes[56], bytes[57], bytes[58], bytes[59]]),
        MINI_CUTOFF
    );
    // The total length is always a whole number of 512-byte sectors + header.
    assert_eq!((bytes.len() - HEADER_LEN) % SECTOR_SIZE, 0);
}

// ---------------------------------------------------------------------------
// Single stream (large).
// ---------------------------------------------------------------------------

#[test]
fn single_large_stream_round_trips() {
    let data = vec![0x42u8; 4096];
    let bytes = write_cfb(&[("Big", &data[..])]);
    let cf = cfb::CompoundFile::open(&bytes).unwrap();
    assert_eq!(cf.read_stream("Big").unwrap(), data);
}

// ---------------------------------------------------------------------------
// Single small stream (mini-FAT path in isolation).
// ---------------------------------------------------------------------------

#[test]
fn single_small_stream_round_trips() {
    let bytes = write_cfb(&[("Tiny", &b"abc"[..])]);
    let cf = cfb::CompoundFile::open(&bytes).unwrap();
    assert_eq!(cf.read_stream("Tiny").unwrap(), b"abc".to_vec());
}

// ---------------------------------------------------------------------------
// Empty stream (0 bytes) — must round-trip as an empty vector.
// ---------------------------------------------------------------------------

#[test]
fn empty_stream_round_trips() {
    let bytes = write_cfb(&[("Nothing", &[][..]), ("Something", &b"data"[..])]);
    let cf = cfb::CompoundFile::open(&bytes).unwrap();
    assert_eq!(cf.read_stream("Nothing").unwrap(), Vec::<u8>::new());
    assert_eq!(cf.read_stream("Something").unwrap(), b"data".to_vec());
}

// ---------------------------------------------------------------------------
// Empty stream set — a valid CFB with only a Root Entry, no streams.
// ---------------------------------------------------------------------------

#[test]
fn no_streams_produces_valid_empty_cfb() {
    let bytes = CfbWriter::new().finish();
    let cf = cfb::CompoundFile::open(&bytes).expect("empty CFB must be valid");
    assert!(cf.stream_names().is_empty());
    // The root storage should still be enumerable.
    assert!(cf
        .entries()
        .iter()
        .any(|e| e.kind == cfb::EntryKind::RootStorage));
}

// ---------------------------------------------------------------------------
// Boundary: a stream exactly at the 4096 cutoff goes to the REGULAR FAT
// (cutoff is `< 4096` -> mini, so 4096 is large).
// ---------------------------------------------------------------------------

#[test]
fn exactly_cutoff_bytes_is_large() {
    let data = vec![0x7Eu8; MINI_CUTOFF as usize]; // exactly 4096
    let bytes = write_cfb(&[("AtCutoff", &data[..])]);
    let cf = cfb::CompoundFile::open(&bytes).unwrap();
    assert_eq!(cf.read_stream("AtCutoff").unwrap(), data);
    // And one byte under the cutoff must round-trip via the mini path.
    let small = vec![0x7Eu8; (MINI_CUTOFF - 1) as usize];
    let bytes2 = write_cfb(&[("JustUnder", &small[..])]);
    let cf2 = cfb::CompoundFile::open(&bytes2).unwrap();
    assert_eq!(cf2.read_stream("JustUnder").unwrap(), small);
}

// ---------------------------------------------------------------------------
// Many small streams that overflow one mini-sector's worth, exercising a
// multi-mini-sector mini-stream and a multi-slot mini-FAT.
// ---------------------------------------------------------------------------

#[test]
fn many_small_streams_overflow_mini_sectors() {
    // Build 50 small streams of varying sizes, each < cutoff, whose total
    // exceeds a single 64-byte mini-sector many times over.
    let mut pairs: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..50u32 {
        let name = format!("s{i}");
        let len = (i as usize % 200) + 1; // 1..=200 bytes -> spans mini-sectors
        pairs.push((name, vec![(i & 0xFF) as u8; len]));
    }
    let refs: Vec<(&str, &[u8])> = pairs.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
    let bytes = write_cfb(&refs);
    let cf = cfb::CompoundFile::open(&bytes).unwrap();
    for (name, data) in &pairs {
        assert_eq!(
            cf.read_stream(name).unwrap(),
            *data,
            "mismatch on stream {name}"
        );
    }
    assert_eq!(cf.stream_names().len(), 50);
}

// ---------------------------------------------------------------------------
// A stream large enough to span multiple 512-byte sectors AND large enough to
// require more than one FAT sector (a single FAT sector maps 128 sectors =
// 64 KiB; we go well past that).
// ---------------------------------------------------------------------------

#[test]
fn large_stream_spans_multiple_fat_sectors() {
    // 300 KiB -> ~600 sectors -> needs >4 FAT sectors, exercising the
    // fixed-point FAT-sector count and multi-sector chains.
    let data: Vec<u8> = (0..300 * 1024u32).map(|i| (i & 0xFF) as u8).collect();
    let bytes = write_cfb(&[("Huge", &data[..])]);
    let cf = cfb::CompoundFile::open(&bytes).unwrap();
    assert_eq!(cf.read_stream("Huge").unwrap(), data);
    // Confirm the header actually records more than one FAT sector.
    let num_fat = u32::from_le_bytes([bytes[44], bytes[45], bytes[46], bytes[47]]);
    assert!(num_fat > 1, "expected multiple FAT sectors, got {num_fat}");
}

// ---------------------------------------------------------------------------
// Name-too-long handling: names over 31 UTF-16 units are truncated.
// ---------------------------------------------------------------------------

#[test]
fn overlong_name_is_truncated_to_31_units() {
    let long = "A".repeat(100); // 100 ASCII units
    let bytes = write_cfb(&[(long.as_str(), &b"payload"[..])]);
    let cf = cfb::CompoundFile::open(&bytes).unwrap();
    let names = cf.stream_names();
    assert_eq!(names.len(), 1);
    assert_eq!(names[0].chars().count(), MAX_NAME_UNITS);
    assert_eq!(names[0], "A".repeat(MAX_NAME_UNITS));
    // The data is still intact under the truncated name.
    assert_eq!(cf.read_stream(&names[0]).unwrap(), b"payload".to_vec());
}

#[test]
fn name_exactly_31_units_is_kept_whole() {
    let name = "B".repeat(MAX_NAME_UNITS);
    let bytes = write_cfb(&[(name.as_str(), &b"x"[..])]);
    let cf = cfb::CompoundFile::open(&bytes).unwrap();
    assert_eq!(cf.read_stream(&name).unwrap(), b"x".to_vec());
}

// ---------------------------------------------------------------------------
// Unicode / control-prefixed names (the classic \u{5}SummaryInformation).
// ---------------------------------------------------------------------------

#[test]
fn control_prefixed_and_unicode_names_round_trip() {
    let bytes = write_cfb(&[
        ("\u{5}SummaryInformation", &b"summary"[..]),
        ("café-Ω", &b"unicode"[..]),
    ]);
    let cf = cfb::CompoundFile::open(&bytes).unwrap();
    assert_eq!(
        cf.read_stream("\u{5}SummaryInformation").unwrap(),
        b"summary".to_vec()
    );
    assert_eq!(cf.read_stream("café-Ω").unwrap(), b"unicode".to_vec());
}

// ---------------------------------------------------------------------------
// The builder API (add_stream) and the free function agree.
// ---------------------------------------------------------------------------

#[test]
fn builder_and_free_function_agree() {
    let mut w = CfbWriter::new();
    w.add_stream("One", b"first");
    w.add_stream("Two", b"second");
    let a = w.finish();
    let b = write_cfb(&[("One", &b"first"[..]), ("Two", &b"second"[..])]);
    assert_eq!(a, b, "builder and convenience fn must be identical");
}

// ---------------------------------------------------------------------------
// Determinism: the same input always yields identical bytes (no timestamps).
// ---------------------------------------------------------------------------

#[test]
fn output_is_deterministic() {
    let mk = || write_cfb(&[("A", &vec![9u8; 5000][..]), ("B", &b"tiny"[..])]);
    assert_eq!(mk(), mk());
}

// ---------------------------------------------------------------------------
// A mix where the mini-stream itself spans more than one 512-byte sector
// (mini-stream > 512 bytes) — exercises multi-sector mini-stream chaining.
// ---------------------------------------------------------------------------

#[test]
fn mini_stream_spanning_multiple_sectors() {
    // 20 streams of ~200 bytes each -> mini-stream ~ several KiB -> several
    // 512-byte sectors, all chained through the regular FAT.
    let mut pairs: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..20u32 {
        pairs.push((format!("m{i}"), vec![(i as u8).wrapping_add(1); 200]));
    }
    let refs: Vec<(&str, &[u8])> = pairs.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
    let bytes = write_cfb(&refs);
    let cf = cfb::CompoundFile::open(&bytes).unwrap();
    for (name, data) in &pairs {
        assert_eq!(cf.read_stream(name).unwrap(), *data);
    }
}

// ---------------------------------------------------------------------------
// Case-insensitive lookup still works on our output (reader behaviour).
// ---------------------------------------------------------------------------

#[test]
fn reader_case_insensitive_lookup_on_our_output() {
    let bytes = write_cfb(&[("Workbook", &vec![0u8; 4096][..])]);
    let cf = cfb::CompoundFile::open(&bytes).unwrap();
    assert!(cf.read_stream("WORKBOOK").is_some());
    assert!(cf.read_stream("workbook").is_some());
}

// ---------------------------------------------------------------------------
// Unit tests for the internal helpers (pure functions, no I/O).
// ---------------------------------------------------------------------------

#[test]
fn div_round_up_edge_cases() {
    assert_eq!(div_round_up(0, 512), 0);
    assert_eq!(div_round_up(1, 512), 1);
    assert_eq!(div_round_up(512, 512), 1);
    assert_eq!(div_round_up(513, 512), 2);
    assert_eq!(div_round_up(64, 64), 1);
    assert_eq!(div_round_up(65, 64), 2);
}

#[test]
fn truncate_name_boundaries() {
    assert_eq!(truncate_name("short"), "short");
    let long = "z".repeat(40);
    assert_eq!(truncate_name(&long).chars().count(), MAX_NAME_UNITS);
    assert_eq!(truncate_name(&"y".repeat(MAX_NAME_UNITS)).chars().count(), MAX_NAME_UNITS);
}

#[test]
fn encode_fat_like_pads_with_freesect() {
    // Two entries in a 128-slot sector: the rest must be FREESECT.
    let bytes = encode_fat_like(&[ENDOFCHAIN, FATSECT], FAT_ENTRIES_PER_SECTOR);
    assert_eq!(bytes.len(), SECTOR_SIZE);
    assert_eq!(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]), ENDOFCHAIN);
    assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), FATSECT);
    // Slot 2 (byte 8) onwards is FREESECT.
    assert_eq!(u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]), FREESECT);
}

#[test]
fn encode_fat_like_empty_is_empty() {
    assert!(encode_fat_like(&[], FAT_ENTRIES_PER_SECTOR).is_empty());
}

#[test]
fn pad_to_sector_rounds_up() {
    let mut v = vec![1u8; 10];
    pad_to_sector(&mut v);
    assert_eq!(v.len(), SECTOR_SIZE);
    let mut w = vec![1u8; SECTOR_SIZE];
    pad_to_sector(&mut w);
    assert_eq!(w.len(), SECTOR_SIZE); // already aligned: unchanged
    let mut z: Vec<u8> = Vec::new();
    pad_to_sector(&mut z);
    assert!(z.is_empty());
}

#[test]
fn encode_directory_is_sector_aligned() {
    let dir = vec![DirEntryBuild {
        name: "Root Entry".into(),
        object_type: OBJ_ROOT,
        right: NOSTREAM,
        child: NOSTREAM,
        start_sector: ENDOFCHAIN,
        size: 0,
    }];
    let bytes = encode_directory(&dir);
    assert_eq!(bytes.len() % SECTOR_SIZE, 0);
    // First entry's object type is at offset 66.
    assert_eq!(bytes[66], OBJ_ROOT);
}
