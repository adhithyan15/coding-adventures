// A BROKEN in-test subject of tests/fixtures/wxg.warrant: every export has one
// of the bugs VER04k section 4.1 names (a mutation lost on a temporary copy, an
// off-by-one loop bound, an append that overwrites, a write that is skipped).
// The generated evidence must fail for every operation (tests/wxg_broken.rs).

/// CWE-193: the loop stops one byte early.
pub fn wipe(buf: &mut [u8]) {
    let n = buf.len().saturating_sub(1);
    for b in buf[..n].iter_mut() {
        *b = 0;
    }
}

/// The zeroize-a-copy bug: the mutation lands on a temporary.
pub fn wipe4(buf: &mut [u8; 4]) {
    let mut copy = *buf;
    for b in copy.iter_mut() {
        *b = 0;
    }
    std::hint::black_box(copy);
}

/// The write is skipped.
pub fn bump(x: &mut u8) {
    std::hint::black_box(x);
}

pub fn toggle(flag: &mut bool) {
    std::hint::black_box(flag);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub a: u8,
    pub on: bool,
}

/// A state-dependent bug: only a cell with a non-zero byte is switched on.
pub fn set_on(c: &mut Cell) {
    c.on = c.a != 0;
}

/// An append that overwrites existing data.
pub fn push_byte(v: u8, out: &mut Vec<u8>) -> usize {
    if !out.is_empty() {
        out[0] ^= 1;
    }
    out.push(v);
    1
}

/// A crashing subject: under `error=` every call is a panic, hence `Err`.
pub fn clear_nonempty(buf: &mut [u8]) {
    let _ = buf;
    panic!("always crashes");
}

/// The review's repro: it wipes the buffer and then ALWAYS returns `Err(())`.
pub fn clr(buf: &mut [u8]) -> Result<(), ()> {
    for b in buf.iter_mut() {
        *b = 0;
    }
    Err(())
}

/// Writes only the first byte but reports two.
pub fn encode(v: u8, out: &mut [u8]) -> Result<usize, ()> {
    if out.len() < 2 {
        return Err(());
    }
    out[0] = v;
    Ok(2)
}
