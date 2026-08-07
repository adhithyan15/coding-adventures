//! Low-level GDSII binary stream encoding.
//!
//! Coordinates in the GDSII file are integers in database units. The `UNITS`
//! record tells readers: 1 dbu = `user_unit` micrometres. Sky130 convention:
//! `user_unit = 0.001` µm = 1 nm, so 1 µm = 1000 dbu.

// ---------------------------------------------------------------------------
// Record type constants  (high byte = record type, low byte = data type)
// ---------------------------------------------------------------------------

const HEADER:    u16 = 0x0002; // integer
const BGNLIB:    u16 = 0x0102; // integer (timestamp pair)
const LIBNAME:   u16 = 0x0206; // string
const UNITS:     u16 = 0x0305; // two GDS reals
const ENDLIB:    u16 = 0x0400; // no data
const BGNSTR:    u16 = 0x0502; // integer (timestamp pair)
const STRNAME:   u16 = 0x0606; // string
const ENDSTR:    u16 = 0x0700; // no data
const BOUNDARY:  u16 = 0x0800; // no data (start of BOUNDARY element)
const PATH:      u16 = 0x0900; // no data
const SREF:      u16 = 0x0A00; // no data
const TEXT:      u16 = 0x0C00; // no data
const LAYER:     u16 = 0x0D02; // integer
const DATATYPE:  u16 = 0x0E02; // integer
const WIDTH:     u16 = 0x0F03; // integer
const XY:        u16 = 0x1003; // integer pairs
const ENDEL:     u16 = 0x1100; // no data
const SNAME:     u16 = 0x1206; // string
const TEXTTYPE:  u16 = 0x1602; // integer
const STRING:    u16 = 0x1906; // string

// ---------------------------------------------------------------------------
// GDSII real encoding
// ---------------------------------------------------------------------------

/// Convert an `f64` to the 8-byte GDSII fixed-point real format.
///
/// The GDSII real uses base-16 with a 7-bit excess-64 exponent:
/// `value = mantissa × 16^(exponent − 64)` where `1/16 ≤ mantissa < 1`.
/// The sign bit occupies the MSB of the first byte.
pub fn double_to_gds_real(value: f64) -> [u8; 8] {
    if value == 0.0 { return [0u8; 8]; }

    let (sign, mut v) = if value < 0.0 { (1u8, -value) } else { (0u8, value) };

    let mut exponent: i32 = 64;
    while v >= 1.0     { v /= 16.0; exponent += 1; }
    while v < 1.0/16.0 { v *= 16.0; exponent -= 1; }

    // 56-bit mantissa.
    let mantissa = (v * ((1u64 << 56) as f64)) as u64;
    let mantissa = mantissa.min((1u64 << 56) - 1);

    let sign_exp = (sign << 7) | (exponent as u8 & 0x7F);
    let mut out = [0u8; 8];
    out[0] = sign_exp;
    let mbytes = mantissa.to_be_bytes();
    out[1..8].copy_from_slice(&mbytes[1..8]);
    out
}

// ---------------------------------------------------------------------------
// Record builder
// ---------------------------------------------------------------------------

fn record(rec_type: u16, payload: &[u8]) -> Vec<u8> {
    let length = 4 + payload.len();
    assert!(length <= 0xFFFF, "GDSII record too long: {length}");
    let rt = (rec_type >> 8) as u8;
    let dt = rec_type as u8;
    let mut out = Vec::with_capacity(4 + payload.len());
    out.extend_from_slice(&(length as u16).to_be_bytes());
    out.push(rt);
    out.push(dt);
    out.extend_from_slice(payload);
    out
}

fn rec_no_data(rec_type: u16) -> Vec<u8> { record(rec_type, &[]) }

fn rec_int2(rec_type: u16, value: i16) -> Vec<u8> {
    record(rec_type, &value.to_be_bytes())
}

fn rec_int4(rec_type: u16, values: &[i32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(4 * values.len());
    for &v in values { payload.extend_from_slice(&v.to_be_bytes()); }
    record(rec_type, &payload)
}

fn rec_string(rec_type: u16, s: &str) -> Vec<u8> {
    // GDSII strings must be ASCII, padded to even length with a NUL.
    let mut bytes: Vec<u8> = s.bytes().collect();
    if !bytes.len().is_multiple_of(2) { bytes.push(0); }
    record(rec_type, &bytes)
}

fn rec_units(user_unit: f64, db_unit: f64) -> Vec<u8> {
    let mut payload = [0u8; 16];
    let r1 = double_to_gds_real(user_unit);
    let r2 = double_to_gds_real(db_unit);
    payload[0..8].copy_from_slice(&r1);
    payload[8..16].copy_from_slice(&r2);
    record(UNITS, &payload)
}

// Zero-valued timestamp (6 i16 fields: year, month, day, hour, min, sec).
fn rec_timestamp() -> Vec<u8> {
    record(BGNLIB, &[0u8; 24])
}

fn rec_bgnstr() -> Vec<u8> {
    record(BGNSTR, &[0u8; 24])
}

// ---------------------------------------------------------------------------
// High-level data structures
// ---------------------------------------------------------------------------

/// A polygon boundary in one cell.
#[derive(Debug, Clone)]
pub struct GdsBoundary {
    /// GDS layer number.
    pub layer: i16,
    /// GDS datatype.
    pub datatype: i16,
    /// Closed polygon vertices (last point = first point). Integer dbu.
    pub xy: Vec<(i32, i32)>,
}

/// A path (wire) in one cell.
#[derive(Debug, Clone)]
pub struct GdsPath {
    pub layer: i16,
    pub datatype: i16,
    pub width: i32,
    pub xy: Vec<(i32, i32)>,
}

/// A structure reference (instance).
#[derive(Debug, Clone)]
pub struct GdsSref {
    pub sname: String,
    pub x: i32,
    pub y: i32,
}

/// A text label.
#[derive(Debug, Clone)]
pub struct GdsText {
    pub layer: i16,
    pub texttype: i16,
    pub text: String,
    pub x: i32,
    pub y: i32,
}

/// One GDSII structure (cell).
#[derive(Debug, Clone, Default)]
pub struct GdsCell {
    pub name: String,
    pub boundaries: Vec<GdsBoundary>,
    pub paths: Vec<GdsPath>,
    pub srefs: Vec<GdsSref>,
    pub texts: Vec<GdsText>,
}

impl GdsCell {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), ..Default::default() }
    }
}

/// Top-level GDSII library writer.
pub struct GdsWriter {
    pub library_name: String,
    /// 1 user unit in metres = `user_unit` µm × 1e-6.
    /// Convention: 0.001 → 1 dbu = 1 nm.
    pub user_unit: f64,
    /// 1 dbu in metres.
    pub db_unit: f64,
    pub cells: Vec<GdsCell>,
}

impl GdsWriter {
    pub fn new(library_name: impl Into<String>) -> Self {
        Self {
            library_name: library_name.into(),
            // Sky130 convention: 1 µm = 1000 dbu; 1 dbu = 1 nm = 1e-9 m.
            user_unit: 1e-3,   // 0.001 µm per dbu
            db_unit: 1e-9,     // 1 dbu = 1 nm = 1e-9 m
            cells: vec![],
        }
    }

    /// Encode the full library as a GDSII binary byte stream.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();

        // Library header.
        out.extend(rec_int2(HEADER, 600)); // version 600
        out.extend(rec_timestamp());
        out.extend(rec_string(LIBNAME, &self.library_name));
        out.extend(rec_units(self.user_unit, self.db_unit));

        // Each cell.
        for cell in &self.cells {
            out.extend(encode_cell(cell));
        }

        out.extend(rec_no_data(ENDLIB));
        out
    }
}

fn encode_cell(cell: &GdsCell) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend(rec_bgnstr());
    out.extend(rec_string(STRNAME, &cell.name));

    for b in &cell.boundaries {
        out.extend(rec_no_data(BOUNDARY));
        out.extend(rec_int2(LAYER, b.layer));
        out.extend(rec_int2(DATATYPE, b.datatype));
        let xy: Vec<i32> = b.xy.iter().flat_map(|&(x,y)| [x, y]).collect();
        out.extend(rec_int4(XY, &xy));
        out.extend(rec_no_data(ENDEL));
    }

    for p in &cell.paths {
        out.extend(rec_no_data(PATH));
        out.extend(rec_int2(LAYER, p.layer));
        out.extend(rec_int2(DATATYPE, p.datatype));
        out.extend(rec_int4(WIDTH, &[p.width]));
        let xy: Vec<i32> = p.xy.iter().flat_map(|&(x,y)| [x, y]).collect();
        out.extend(rec_int4(XY, &xy));
        out.extend(rec_no_data(ENDEL));
    }

    for s in &cell.srefs {
        out.extend(rec_no_data(SREF));
        out.extend(rec_string(SNAME, &s.sname));
        out.extend(rec_int4(XY, &[s.x, s.y]));
        out.extend(rec_no_data(ENDEL));
    }

    for t in &cell.texts {
        out.extend(rec_no_data(TEXT));
        out.extend(rec_int2(LAYER, t.layer));
        out.extend(rec_int2(TEXTTYPE, t.texttype));
        out.extend(rec_int4(XY, &[t.x, t.y]));
        out.extend(rec_string(STRING, &t.text));
        out.extend(rec_no_data(ENDEL));
    }

    out.extend(rec_no_data(ENDSTR));
    out
}

/// Convert a floating-point µm coordinate to database units (integer).
/// Sky130: 1 µm = 1000 dbu (1 dbu = 1 nm).
pub fn um_to_dbu(um: f64) -> i32 {
    (um * 1000.0).round() as i32
}
