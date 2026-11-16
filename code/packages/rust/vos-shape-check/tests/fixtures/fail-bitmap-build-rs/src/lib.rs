// BROKEN VARIANT of the toy. vos-ready-bitmap/src/lib.rs: VOS01 section 5.3 (the production code) with the
// ghost face of VOS02 section 4.2.  The "..." elisions of the specs are filled in.
#![no_std]
#![forbid(unsafe_code)]
use vos_ready_contract::{Pick, ReadySet, Upd};

#[derive(Copy, Clone, PartialEq, Eq)] pub struct Bitmap { mask: u8 }   // private fields: rule 2
pub struct BitmapReady;
const _: () = assert!(BitmapReady::N == 8);          // N is the mask width; compile-time check

// Leading zeros as a three-stage binary search: branch-free, division-free, visible cost.
// u8::leading_zeros is avoided: on RV32I without bit-manipulation it may become a call into a
// compiler runtime routine outside the contract (`unbound-item`, VER29 section 8.3).
const fn clz8(x: u8) -> u8 {
    let e4 = (x >> 4 == 0) as u8;  let x = x.wrapping_shl((e4 << 2) as u32);
    let e2 = (x >> 6 == 0) as u8;  let x = x.wrapping_shl((e2 << 1) as u32);
    let e1 = (x >> 7 == 0) as u8;  let x = x.wrapping_shl(e1 as u32);
    (e4 << 2).wrapping_add(e2 << 1).wrapping_add(e1).wrapping_add((x == 0) as u8)
}
impl ReadySet for BitmapReady {
    type State = Bitmap;
    const N: u8 = 8;
    fn init() -> Bitmap { Bitmap { mask: 0 } }
    fn insert(s: Bitmap, p: u8) -> (Upd, Bitmap) {
        if p >= Self::N { return (Upd::OutOfRange, s); }
        let bit = 1u8.wrapping_shl(p as u32);
        if s.mask & bit != 0 { (Upd::Unchanged, s) } else { (Upd::Changed, Bitmap { mask: s.mask | bit }) }
    }
    fn remove(s: Bitmap, p: u8) -> (Upd, Bitmap) {
        if p >= Self::N { return (Upd::OutOfRange, s); }
        let bit = 1u8.wrapping_shl(p as u32);
        if s.mask & bit == 0 { (Upd::Unchanged, s) } else { (Upd::Changed, Bitmap { mask: s.mask & !bit }) }
    }
    fn pick(s: Bitmap) -> (Pick, Bitmap) {
        if s.mask == 0 { (Pick::NoneReady, s) } else { (Pick::Member(7u8.wrapping_sub(clz8(s.mask))), s) }
    }
}

#[cfg(feature = "ghost")]                       // SH13: evidence build only
pub mod ghost {
    use super::*;
    pub struct Image { pub mask: u8 }            // public fields, equal to State's (SH17)
    pub fn vos_ghost_init() -> Image {
        let Bitmap { mask } = <BitmapReady as ReadySet>::init();
        Image { mask }
    }
    pub fn vos_ghost_insert(s: Image, p: u8) -> (Upd, Image) {
        let (o, Bitmap { mask }) = <BitmapReady as ReadySet>::insert(Bitmap { mask: s.mask }, p);
        (o, Image { mask })                      // exhaustive destructure, one UFCS call, restructure
    }
    pub fn vos_ghost_remove(s: Image, p: u8) -> (Upd, Image) {
        let (o, Bitmap { mask }) = <BitmapReady as ReadySet>::remove(Bitmap { mask: s.mask }, p);
        (o, Image { mask })
    }
    pub fn vos_ghost_pick(s: Image) -> (Pick, Image) {
        let (o, Bitmap { mask }) = <BitmapReady as ReadySet>::pick(Bitmap { mask: s.mask });
        (o, Image { mask })
    }
    // The abstraction and the configuration are exports, not wrappers: no UFCS call.
    pub fn vos_ghost_alpha(s: Image) -> u8 { s.mask }
    pub fn vos_ghost_cfg_n() -> u8 { <BitmapReady as ReadySet>::N }
}
#[cfg(feature = "ghost")]
pub use ghost::{vos_ghost_alpha, vos_ghost_cfg_n, vos_ghost_init, vos_ghost_insert, vos_ghost_pick, vos_ghost_remove};
