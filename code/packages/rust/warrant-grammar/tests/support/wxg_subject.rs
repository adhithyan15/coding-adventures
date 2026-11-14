// The honest in-test subject of tests/fixtures/wxg.warrant: one mutating export
// for every host form and post-state slot of VER04k section 4.  The exports are
// written from the definitions in the document, never from the generated code.

/// S1, `&mut [u8]`: every byte becomes zero.
pub fn wipe(buf: &mut [u8]) {
    for b in buf.iter_mut() {
        *b = 0;
    }
}

/// S1, `&mut [u8; 4]`.
pub fn wipe4(buf: &mut [u8; 4]) {
    for b in buf.iter_mut() {
        *b = 0;
    }
}

/// S1, `&mut u8`: add one, wrapping.
pub fn bump(x: &mut u8) {
    *x = x.wrapping_add(1);
}

/// S1, `&mut bool`.
pub fn toggle(flag: &mut bool) {
    *flag = !*flag;
}

/// S1, `&mut Cell` (a declared host struct).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub a: u8,
    pub on: bool,
}

pub fn set_on(c: &mut Cell) {
    c.on = true;
}

/// S3, `&mut Vec<u8>`: append one byte and say how many were appended.
pub fn push_byte(v: u8, out: &mut Vec<u8>) -> usize {
    out.push(v);
    1
}

/// S2 served by a host that returns nothing (VER04k section 4.6, M9): an empty
/// buffer is a panic, which the adapter normalises to `Err(TOO_SHORT)`.
pub fn clear_nonempty(buf: &mut [u8]) {
    assert!(!buf.is_empty(), "nothing to clear");
    wipe(buf);
}

/// S4 served by `Result<usize, ()>`: the opaque error is the one case TOO_SHORT.
pub fn encode(v: u8, out: &mut [u8]) -> Result<usize, ()> {
    if out.len() < 2 {
        return Err(());
    }
    out[0] = v;
    out[1] = !v;
    Ok(2)
}

/// S2 served by `Result<(), ()>`: an empty buffer is the one error.
pub fn clr(buf: &mut [u8]) -> Result<(), ()> {
    if buf.is_empty() {
        return Err(());
    }
    wipe(buf);
    Ok(())
}

/// S3 with two host-returned components.
pub fn pair(buf: &mut [u8]) -> (u8, u8) {
    wipe(buf);
    (1, 2)
}
