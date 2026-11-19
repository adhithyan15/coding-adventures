// The subject of tests/wxf_array.rs (VOS-D025): eight cells, each 0 or 1, in a byte array.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cells {
    pub cells: [u8; 8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Total {
    pub n: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Idle,
    Run(u8),
}

pub fn tally(s: Cells) -> Total {
    Total { n: s.cells.iter().filter(|&&c| c != 0).count() as u8 }
}

pub fn lowest(s: Cells) -> Pick {
    match s.cells.iter().position(|&c| c != 0) {
        Some(i) => Pick::Run(i as u8),
        None => Pick::Idle,
    }
}

pub fn highest(s: Cells) -> Pick {
    match s.cells.iter().rposition(|&c| c != 0) {
        Some(i) => Pick::Run(i as u8),
        None => Pick::Idle,
    }
}
