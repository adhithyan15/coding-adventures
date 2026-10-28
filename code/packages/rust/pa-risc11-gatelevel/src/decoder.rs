//! Independent fail-closed PA-RISC instruction decoder.

use crate::gates::{decode6, selected};
use logic_gates::gates::{and_gate, not_gate, or_gate};
use pa_risc11_simulator::PaRisc11Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Kind {
    Break,
    Andcm,
    And,
    Or,
    Xor,
    Sub,
    Add,
    Ldil,
    Addil,
    Ldo,
    Load {
        width: usize,
    },
    Store {
        width: usize,
    },
    Conditional {
        add: bool,
        immediate: bool,
        branch_false: bool,
    },
    Shd,
    Bl,
    Blr,
    Bv,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Decoded {
    pub kind: Kind,
    pub right: u8,
    pub left: u8,
    pub target: u8,
    pub condition: u8,
    pub nullify: bool,
}

fn any(lines: &[u8], indexes: &[usize]) -> u8 {
    indexes
        .iter()
        .fold(0, |value, index| or_gate(value, lines[*index]))
}

pub(crate) fn decode(word: u32, pc: u32, in_delay_slot: bool) -> Result<Decoded, PaRisc11Error> {
    let primary = ((word >> 26) & 0x3f) as u8;
    let lines = decode6(primary);
    let control = any(
        &lines,
        &[0x20, 0x21, 0x22, 0x23, 0x28, 0x29, 0x2a, 0x2b, 0x3a],
    );
    if and_gate(control, u8::from(in_delay_slot)) == 1 {
        return Err(PaRisc11Error::UnsupportedDelayedControl { pc });
    }

    let right = ((word >> 21) & 31) as u8;
    let left = ((word >> 16) & 31) as u8;
    let target = (word & 31) as u8;
    let condition = ((word >> 13) & 7) as u8;
    let nullify = ((word >> 1) & 1) == 1;
    let base = Decoded {
        kind: Kind::Break,
        right,
        left,
        target,
        condition,
        nullify,
    };

    if lines[0] == 1 {
        if selected(word, 0, 32) == 1 {
            return Ok(base);
        }
        return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
    }
    if lines[2] == 1 {
        let reserved = or_gate(
            any(&decode6(((word >> 13) & 7) as u8), &[1, 2, 3, 4, 5, 6, 7]),
            or_gate(((word >> 12) & 1) as u8, ((word >> 5) & 1) as u8),
        );
        if reserved == 1 {
            return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
        }
        let extension = decode6(((word >> 6) & 0x3f) as u8);
        let kind = if extension[0] == 1 {
            Kind::Andcm
        } else if extension[8] == 1 {
            Kind::And
        } else if extension[9] == 1 {
            Kind::Or
        } else if extension[10] == 1 {
            Kind::Xor
        } else if extension[16] == 1 {
            Kind::Sub
        } else if extension[24] == 1 {
            Kind::Add
        } else {
            return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
        };
        return Ok(Decoded { kind, ..base });
    }
    if lines[8] == 1 {
        return Ok(Decoded {
            kind: Kind::Ldil,
            ..base
        });
    }
    if lines[10] == 1 {
        return Ok(Decoded {
            kind: Kind::Addil,
            ..base
        });
    }
    if lines[13] == 1 {
        if selected((word >> 14) & 3, 0, 2) == 0 {
            return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
        }
        return Ok(Decoded {
            kind: Kind::Ldo,
            ..base
        });
    }
    for (line, width, load) in [
        (0x10, 1, true),
        (0x11, 2, true),
        (0x12, 4, true),
        (0x18, 1, false),
        (0x19, 2, false),
        (0x1a, 4, false),
    ] {
        if lines[line] == 1 {
            if selected((word >> 14) & 3, 0, 2) == 0 {
                return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
            }
            return Ok(Decoded {
                kind: if load {
                    Kind::Load { width }
                } else {
                    Kind::Store { width }
                },
                ..base
            });
        }
    }
    for line in [0x20, 0x21, 0x22, 0x23, 0x28, 0x29, 0x2a, 0x2b] {
        if lines[line] == 1 {
            return Ok(Decoded {
                kind: Kind::Conditional {
                    add: line >= 0x28,
                    immediate: line & 1 == 1,
                    branch_false: line & 2 == 2,
                },
                ..base
            });
        }
    }
    if lines[0x34] == 1 {
        let valid = and_gate(
            selected((word >> 13) & 7, 0, 3),
            selected((word >> 10) & 7, 2, 3),
        );
        if valid == 0 {
            return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
        }
        return Ok(Decoded {
            kind: Kind::Shd,
            ..base
        });
    }
    if lines[0x3a] == 1 {
        let extension = decode6(((word >> 13) & 7) as u8);
        if extension[0] == 1 {
            return Ok(Decoded {
                kind: Kind::Bl,
                ..base
            });
        }
        let reserved = or_gate(
            (word & 1) as u8,
            not_gate(selected((word >> 2) & 0x7ff, 0, 11)),
        );
        if reserved == 1 {
            return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
        }
        if extension[2] == 1 {
            return Ok(Decoded {
                kind: Kind::Blr,
                ..base
            });
        }
        if extension[6] == 1 {
            return Ok(Decoded {
                kind: Kind::Bv,
                ..base
            });
        }
        return Err(PaRisc11Error::UnsupportedInstruction { pc, word });
    }
    Err(PaRisc11Error::UnknownInstruction { pc, word })
}
