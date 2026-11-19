//! The Rust half of the toy contract's types (CK10 compares it with `warrant/contract.part`).
#![no_std]
#![forbid(unsafe_code)]

/// A two-flag state, the Warrant `record Pair`.
pub struct Pair {
    pub a: bool,
    pub b: bool,
}

/// `variant Cmd = Nop | Put(slot: U8, on: Bool) | Fill(data: Bytes<4>)`.
pub enum Cmd {
    Nop,
    Put { slot: u8, on: bool },
    Fill([u8; 4]),
}

/// `variant Reply = Done | Got(p: Pair)`: a payload that is a same-crate type.
pub enum Reply {
    Done,
    Got(Pair),
}
