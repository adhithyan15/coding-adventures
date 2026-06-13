//! Built-in cell type registry.
//!
//! The GENERIC level of an HNL uses these cell types as abstract primitives.
//! Tech-mapping replaces them with real library cells (Sky130 stdcells).
//!
//! All 1-bit in / 1-bit out except where noted in `pin_widths`.

use std::collections::HashMap;

/// Signature of a built-in cell type.
#[derive(Debug, Clone, PartialEq)]
pub struct CellTypeSig {
    pub name: &'static str,
    pub inputs: &'static [&'static str],
    pub outputs: &'static [&'static str],
    /// Overrides for pins wider than 1 bit. Missing entries default to 1.
    pub pin_widths: HashMap<&'static str, u32>,
}

impl CellTypeSig {
    fn new(
        name: &'static str,
        inputs: &'static [&'static str],
        outputs: &'static [&'static str],
    ) -> Self {
        Self { name, inputs, outputs, pin_widths: HashMap::new() }
    }

    pub fn width(&self, pin: &str) -> u32 {
        self.pin_widths.get(pin).copied().unwrap_or(1)
    }

    pub fn has_pin(&self, pin: &str) -> bool {
        self.inputs.contains(&pin) || self.outputs.contains(&pin)
    }
}

/// All built-in generic cell types used by the synthesis pass.
pub static BUILTIN_CELL_TYPES: std::sync::LazyLock<HashMap<&'static str, CellTypeSig>> =
    std::sync::LazyLock::new(|| {
        let cells: Vec<CellTypeSig> = vec![
            CellTypeSig::new("BUF",     &["A"],             &["Y"]),
            CellTypeSig::new("NOT",     &["A"],             &["Y"]),
            CellTypeSig::new("AND2",    &["A","B"],         &["Y"]),
            CellTypeSig::new("AND3",    &["A","B","C"],     &["Y"]),
            CellTypeSig::new("AND4",    &["A","B","C","D"], &["Y"]),
            CellTypeSig::new("OR2",     &["A","B"],         &["Y"]),
            CellTypeSig::new("OR3",     &["A","B","C"],     &["Y"]),
            CellTypeSig::new("OR4",     &["A","B","C","D"], &["Y"]),
            CellTypeSig::new("NAND2",   &["A","B"],         &["Y"]),
            CellTypeSig::new("NAND3",   &["A","B","C"],     &["Y"]),
            CellTypeSig::new("NAND4",   &["A","B","C","D"], &["Y"]),
            CellTypeSig::new("NOR2",    &["A","B"],         &["Y"]),
            CellTypeSig::new("NOR3",    &["A","B","C"],     &["Y"]),
            CellTypeSig::new("NOR4",    &["A","B","C","D"], &["Y"]),
            CellTypeSig::new("XOR2",    &["A","B"],         &["Y"]),
            CellTypeSig::new("XOR3",    &["A","B","C"],     &["Y"]),
            CellTypeSig::new("XNOR2",   &["A","B"],         &["Y"]),
            CellTypeSig::new("XNOR3",   &["A","B","C"],     &["Y"]),
            CellTypeSig::new("MUX2",    &["A","B","S"],     &["Y"]),
            CellTypeSig::new("DFF",     &["D","CLK"],       &["Q"]),
            CellTypeSig::new("DFF_R",   &["D","CLK","R"],   &["Q"]),
            CellTypeSig::new("DFF_S",   &["D","CLK","S"],   &["Q"]),
            CellTypeSig::new("DFF_RS",  &["D","CLK","R","S"], &["Q"]),
            CellTypeSig::new("DLATCH",  &["D","EN"],        &["Q"]),
            CellTypeSig::new("TBUF",    &["A","OE"],        &["Y"]),
            CellTypeSig::new("CONST_0", &[],                &["Y"]),
            CellTypeSig::new("CONST_1", &[],                &["Y"]),
        ];
        cells.into_iter().map(|c| (c.name, c)).collect()
    });
