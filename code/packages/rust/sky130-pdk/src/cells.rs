//! Sky130 standard-cell teaching subset.
//!
//! ~33 cells covering everything needed to take a 4-bit adder through the
//! full pipeline to tape-out. Cell name format:
//!   `sky130_fd_sc_hd__<function>_<drive>`
//!
//! "hd" = high-density; "fd" = foundry design; "sc" = standard cell.
//! Drive strength 1 is the minimum (smallest, most power-efficient);
//! higher drives (2, 4, 8) can source/sink more current for long wires.
//!
//! For the full cell list (hundreds of cells), a real Sky130 install is
//! needed. This subset is sufficient for the v0.1.0 teaching pipeline.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

/// Per-cell metadata from the PDK.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellInfo {
    /// Full qualified cell name, e.g. `"sky130_fd_sc_hd__inv_1"`.
    pub name: String,
    /// Boolean function in informal notation, e.g. `"Y = !A"`.
    pub function: String,
    /// Drive strength (1, 2, 4, 8, …). 0 means the cell has no drive
    /// (filler, tap, decap).
    pub drive_strength: u32,
    /// Cell height in routing tracks. sky130_fd_sc_hd is 9 tracks.
    pub height_tracks: u32,
}

impl CellInfo {
    fn new(name: &str, function: &str, drive: u32) -> Self {
        Self {
            name: name.to_string(),
            function: function.to_string(),
            drive_strength: drive,
            height_tracks: 9,
        }
    }
}

/// Teaching subset: ~33 cells.
pub static TEACHING_CELLS: LazyLock<HashMap<&'static str, CellInfo>> = LazyLock::new(|| {
    vec![
        ("sky130_fd_sc_hd__inv_1",     CellInfo::new("sky130_fd_sc_hd__inv_1",     "Y = !A",            1)),
        ("sky130_fd_sc_hd__inv_2",     CellInfo::new("sky130_fd_sc_hd__inv_2",     "Y = !A",            2)),
        ("sky130_fd_sc_hd__inv_4",     CellInfo::new("sky130_fd_sc_hd__inv_4",     "Y = !A",            4)),
        ("sky130_fd_sc_hd__inv_8",     CellInfo::new("sky130_fd_sc_hd__inv_8",     "Y = !A",            8)),
        ("sky130_fd_sc_hd__buf_1",     CellInfo::new("sky130_fd_sc_hd__buf_1",     "X = A",             1)),
        ("sky130_fd_sc_hd__buf_2",     CellInfo::new("sky130_fd_sc_hd__buf_2",     "X = A",             2)),
        ("sky130_fd_sc_hd__buf_4",     CellInfo::new("sky130_fd_sc_hd__buf_4",     "X = A",             4)),
        ("sky130_fd_sc_hd__buf_8",     CellInfo::new("sky130_fd_sc_hd__buf_8",     "X = A",             8)),
        ("sky130_fd_sc_hd__nand2_1",   CellInfo::new("sky130_fd_sc_hd__nand2_1",   "Y = !(A*B)",        1)),
        ("sky130_fd_sc_hd__nand2_2",   CellInfo::new("sky130_fd_sc_hd__nand2_2",   "Y = !(A*B)",        2)),
        ("sky130_fd_sc_hd__nand3_1",   CellInfo::new("sky130_fd_sc_hd__nand3_1",   "Y = !(A*B*C)",      1)),
        ("sky130_fd_sc_hd__nor2_1",    CellInfo::new("sky130_fd_sc_hd__nor2_1",    "Y = !(A+B)",        1)),
        ("sky130_fd_sc_hd__nor2_2",    CellInfo::new("sky130_fd_sc_hd__nor2_2",    "Y = !(A+B)",        2)),
        ("sky130_fd_sc_hd__nor3_1",    CellInfo::new("sky130_fd_sc_hd__nor3_1",    "Y = !(A+B+C)",      1)),
        ("sky130_fd_sc_hd__and2_1",    CellInfo::new("sky130_fd_sc_hd__and2_1",    "X = A*B",           1)),
        ("sky130_fd_sc_hd__and2_2",    CellInfo::new("sky130_fd_sc_hd__and2_2",    "X = A*B",           2)),
        ("sky130_fd_sc_hd__or2_1",     CellInfo::new("sky130_fd_sc_hd__or2_1",     "X = A+B",           1)),
        ("sky130_fd_sc_hd__or2_2",     CellInfo::new("sky130_fd_sc_hd__or2_2",     "X = A+B",           2)),
        ("sky130_fd_sc_hd__xor2_1",    CellInfo::new("sky130_fd_sc_hd__xor2_1",    "X = A^B",           1)),
        ("sky130_fd_sc_hd__xnor2_1",   CellInfo::new("sky130_fd_sc_hd__xnor2_1",  "Y = !(A^B)",        1)),
        ("sky130_fd_sc_hd__mux2_1",    CellInfo::new("sky130_fd_sc_hd__mux2_1",    "X = S?A1:A0",       1)),
        ("sky130_fd_sc_hd__aoi21_1",   CellInfo::new("sky130_fd_sc_hd__aoi21_1",   "Y = !(A1*A2 + B1)", 1)),
        ("sky130_fd_sc_hd__oai21_1",   CellInfo::new("sky130_fd_sc_hd__oai21_1",   "Y = !((A1+A2)*B1)", 1)),
        ("sky130_fd_sc_hd__dfxtp_1",   CellInfo::new("sky130_fd_sc_hd__dfxtp_1",   "Q = D@posedge CLK", 1)),
        ("sky130_fd_sc_hd__dfrtp_1",   CellInfo::new("sky130_fd_sc_hd__dfrtp_1",   "Q=D@CLK, async R",  1)),
        ("sky130_fd_sc_hd__dfstp_1",   CellInfo::new("sky130_fd_sc_hd__dfstp_1",   "Q=D@CLK, async S",  1)),
        ("sky130_fd_sc_hd__dfsrtp_1",  CellInfo::new("sky130_fd_sc_hd__dfsrtp_1",  "Q=D@CLK, R+S",      1)),
        ("sky130_fd_sc_hd__dlxtp_1",   CellInfo::new("sky130_fd_sc_hd__dlxtp_1",   "Q=D when GATE=1",   1)),
        ("sky130_fd_sc_hd__ebufn_1",   CellInfo::new("sky130_fd_sc_hd__ebufn_1",   "Z=A when TE_B=0",   1)),
        ("sky130_fd_sc_hd__conb_1",    CellInfo::new("sky130_fd_sc_hd__conb_1",    "LO=0; HI=1",        1)),
        ("sky130_fd_sc_hd__clkbuf_1",  CellInfo::new("sky130_fd_sc_hd__clkbuf_1",  "X=A (clk buf)",     1)),
        ("sky130_fd_sc_hd__clkbuf_4",  CellInfo::new("sky130_fd_sc_hd__clkbuf_4",  "X=A (clk buf)",     4)),
        ("sky130_fd_sc_hd__tap_1",     CellInfo::new("sky130_fd_sc_hd__tap_1",     "(well tap)",         0)),
        ("sky130_fd_sc_hd__decap_3",   CellInfo::new("sky130_fd_sc_hd__decap_3",   "(decap)",            3)),
        ("sky130_fd_sc_hd__fill_1",    CellInfo::new("sky130_fd_sc_hd__fill_1",    "(filler)",           0)),
    ]
    .into_iter()
    .collect()
});
