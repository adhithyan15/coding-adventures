// vos-ready-linear/src/lib.rs: VOS01 section 5.4, the linear scan (safety tier only).
// insert and remove are "as for the bitmap" in the spec; written out here with the one
// loop form SH6 allows.
#![no_std]
#![forbid(unsafe_code)]
use vos_ready_contract::{Pick, ReadySet, Upd};

#[derive(Copy, Clone, PartialEq, Eq)] pub struct Linear { cells: [u8; 8] }   // inv: every byte is 0 or 1
pub struct LinearReady;
const LEN: usize = 8;                                // == ReadySet::N, checked at compile time
const _: () = assert!(LEN == LinearReady::N as usize);

impl ReadySet for LinearReady {
    type State = Linear;
    const N: u8 = 8;
    fn init() -> Linear { Linear { cells: [0; 8] } }
    fn insert(s: Linear, p: u8) -> (Upd, Linear) {
        if p >= Self::N { return (Upd::OutOfRange, s); }
        let mut cells = s.cells;
        let mut changed = false;
        for i in 0..LEN {
            if i as u8 == p && cells[i] == 0 { cells[i] = 1; changed = true; }
        }
        (if changed { Upd::Changed } else { Upd::Unchanged }, Linear { cells })
    }
    fn remove(s: Linear, p: u8) -> (Upd, Linear) {
        if p >= Self::N { return (Upd::OutOfRange, s); }
        let mut cells = s.cells;
        let mut changed = false;
        for i in 0..LEN {
            if i as u8 == p && cells[i] != 0 { cells[i] = 0; changed = true; }
        }
        (if changed { Upd::Changed } else { Upd::Unchanged }, Linear { cells })
    }
    fn pick(s: Linear) -> (Pick, Linear) {
        let (mut out, mut found) = (Pick::NoneReady, false);
        for i in 0..LEN {                            // SH6: constant bound, the array's own length
            if !found && s.cells[i] != 0 { out = Pick::Member(i as u8); found = true; }
        }
        (out, s)
    }
}
