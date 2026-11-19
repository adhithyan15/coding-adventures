// vos-ready-contract/src/lib.rs, as written in VOS01 section 1.2.
#![no_std]
#![forbid(unsafe_code)]
#[derive(Copy, Clone, PartialEq, Eq)] pub enum Upd  { Changed, Unchanged, OutOfRange }
#[derive(Copy, Clone, PartialEq, Eq)] pub enum Pick { NoneReady, Poke(usize) }

pub trait ReadySet {
    type State;                                   // owned by one field of K
    const N: u8;                                  // configuration, printed in labels
    fn init() -> Self::State;
    fn insert(s: Self::State, p: u8) -> (Upd, Self::State);
    fn remove(s: Self::State, p: u8) -> (Upd, Self::State);
    fn pick(s: Self::State) -> (Pick, Self::State);
}
// One associated type per interface; the profile crate binds each to ONE implementation.
pub trait Profile { type Ready: ReadySet; /* one line per interface */ }
pub struct Kernel<P: Profile> { pub ready: <P::Ready as ReadySet>::State, /* one field per component */ }
