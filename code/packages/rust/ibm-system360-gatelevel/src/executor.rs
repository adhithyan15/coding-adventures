//! Independent gate-network architectural transition engine.

use crate::{gates, hfp::GateHfp, StepTrace, System360Error, System360State};
use logic_gates::gates::{and_gate, or_gate};

const ADDRESS_MASK: u32 = 0x00ff_ffff;

/// A gate-selected architectural operation.
///
/// This enum is deliberately not constructed from a host `match` over the
/// fetched opcode.  [`DecodeAuthority::new`] chooses exactly one table entry
/// with the settled one-hot decoder wires; host control only sequences the
/// operation that wire selected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Operation {
    Balr,
    Bctr,
    Bcr,
    Svc,
    Lpr,
    Lnr,
    Ltr,
    Lcr,
    Nr,
    Clr,
    Or,
    Xr,
    Lr,
    Cr,
    Ar,
    Sr,
    Mr,
    Dr,
    Alr,
    Slr,
    FloatRr(FloatRrOperation),
    Sth,
    La,
    Stc,
    Ic,
    Bal,
    Bct,
    Bc,
    Lh,
    Ch,
    Ah,
    Sh,
    Mh,
    St,
    N,
    Cl,
    O,
    X,
    L,
    C,
    A,
    S,
    M,
    D,
    Al,
    Sl,
    FloatRx(FloatRxOperation),
    Bxh,
    Bxle,
    Srl,
    Sll,
    Sra,
    Sla,
    Srdl,
    Sldl,
    Srda,
    Slda,
    Stm,
    Lm,
    Tm,
    Mvi,
    Ni,
    Cli,
    Oi,
    Xi,
    Mvc,
    Nc,
    Clc,
    Oc,
    Xc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FloatRrOperation {
    Op20,
    Op21,
    Op22,
    Op23,
    Op24,
    Op28,
    Op29,
    Op2a,
    Op2b,
    Op2c,
    Op2d,
    Op30,
    Op31,
    Op32,
    Op33,
    Op34,
    Op38,
    Op39,
    Op3a,
    Op3b,
    Op3c,
    Op3d,
}

impl FloatRrOperation {
    pub(crate) fn is_long(self) -> bool {
        matches!(
            self,
            Self::Op20
                | Self::Op21
                | Self::Op22
                | Self::Op23
                | Self::Op24
                | Self::Op28
                | Self::Op29
                | Self::Op2a
                | Self::Op2b
                | Self::Op2c
                | Self::Op2d
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FloatRxOperation {
    Op60,
    Op68,
    Op69,
    Op6a,
    Op6b,
    Op6c,
    Op6d,
    Op70,
    Op78,
    Op79,
    Op7a,
    Op7b,
    Op7c,
    Op7d,
}

impl FloatRxOperation {
    pub(crate) fn is_long(self) -> bool {
        matches!(
            self,
            Self::Op60
                | Self::Op68
                | Self::Op69
                | Self::Op6a
                | Self::Op6b
                | Self::Op6c
                | Self::Op6d
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Format {
    Rr,
    Rx,
    Rs,
    Si,
    Ss,
}

#[derive(Clone, Copy)]
struct Candidate {
    line: usize,
    format: Format,
    operation: Operation,
}

macro_rules! candidate {
    ($line:literal, $format:ident, $operation:expr) => {
        Candidate {
            line: $line,
            format: Format::$format,
            operation: $operation,
        }
    };
}

const CANDIDATES: &[Candidate] = &[
    candidate!(0x05, Rr, Operation::Balr),
    candidate!(0x06, Rr, Operation::Bctr),
    candidate!(0x07, Rr, Operation::Bcr),
    candidate!(0x0a, Rr, Operation::Svc),
    candidate!(0x10, Rr, Operation::Lpr),
    candidate!(0x11, Rr, Operation::Lnr),
    candidate!(0x12, Rr, Operation::Ltr),
    candidate!(0x13, Rr, Operation::Lcr),
    candidate!(0x14, Rr, Operation::Nr),
    candidate!(0x15, Rr, Operation::Clr),
    candidate!(0x16, Rr, Operation::Or),
    candidate!(0x17, Rr, Operation::Xr),
    candidate!(0x18, Rr, Operation::Lr),
    candidate!(0x19, Rr, Operation::Cr),
    candidate!(0x1a, Rr, Operation::Ar),
    candidate!(0x1b, Rr, Operation::Sr),
    candidate!(0x1c, Rr, Operation::Mr),
    candidate!(0x1d, Rr, Operation::Dr),
    candidate!(0x1e, Rr, Operation::Alr),
    candidate!(0x1f, Rr, Operation::Slr),
    candidate!(0x20, Rr, Operation::FloatRr(FloatRrOperation::Op20)),
    candidate!(0x21, Rr, Operation::FloatRr(FloatRrOperation::Op21)),
    candidate!(0x22, Rr, Operation::FloatRr(FloatRrOperation::Op22)),
    candidate!(0x23, Rr, Operation::FloatRr(FloatRrOperation::Op23)),
    candidate!(0x24, Rr, Operation::FloatRr(FloatRrOperation::Op24)),
    candidate!(0x28, Rr, Operation::FloatRr(FloatRrOperation::Op28)),
    candidate!(0x29, Rr, Operation::FloatRr(FloatRrOperation::Op29)),
    candidate!(0x2a, Rr, Operation::FloatRr(FloatRrOperation::Op2a)),
    candidate!(0x2b, Rr, Operation::FloatRr(FloatRrOperation::Op2b)),
    candidate!(0x2c, Rr, Operation::FloatRr(FloatRrOperation::Op2c)),
    candidate!(0x2d, Rr, Operation::FloatRr(FloatRrOperation::Op2d)),
    candidate!(0x30, Rr, Operation::FloatRr(FloatRrOperation::Op30)),
    candidate!(0x31, Rr, Operation::FloatRr(FloatRrOperation::Op31)),
    candidate!(0x32, Rr, Operation::FloatRr(FloatRrOperation::Op32)),
    candidate!(0x33, Rr, Operation::FloatRr(FloatRrOperation::Op33)),
    candidate!(0x34, Rr, Operation::FloatRr(FloatRrOperation::Op34)),
    candidate!(0x38, Rr, Operation::FloatRr(FloatRrOperation::Op38)),
    candidate!(0x39, Rr, Operation::FloatRr(FloatRrOperation::Op39)),
    candidate!(0x3a, Rr, Operation::FloatRr(FloatRrOperation::Op3a)),
    candidate!(0x3b, Rr, Operation::FloatRr(FloatRrOperation::Op3b)),
    candidate!(0x3c, Rr, Operation::FloatRr(FloatRrOperation::Op3c)),
    candidate!(0x3d, Rr, Operation::FloatRr(FloatRrOperation::Op3d)),
    candidate!(0x40, Rx, Operation::Sth),
    candidate!(0x41, Rx, Operation::La),
    candidate!(0x42, Rx, Operation::Stc),
    candidate!(0x43, Rx, Operation::Ic),
    candidate!(0x45, Rx, Operation::Bal),
    candidate!(0x46, Rx, Operation::Bct),
    candidate!(0x47, Rx, Operation::Bc),
    candidate!(0x48, Rx, Operation::Lh),
    candidate!(0x49, Rx, Operation::Ch),
    candidate!(0x4a, Rx, Operation::Ah),
    candidate!(0x4b, Rx, Operation::Sh),
    candidate!(0x4c, Rx, Operation::Mh),
    candidate!(0x50, Rx, Operation::St),
    candidate!(0x54, Rx, Operation::N),
    candidate!(0x55, Rx, Operation::Cl),
    candidate!(0x56, Rx, Operation::O),
    candidate!(0x57, Rx, Operation::X),
    candidate!(0x58, Rx, Operation::L),
    candidate!(0x59, Rx, Operation::C),
    candidate!(0x5a, Rx, Operation::A),
    candidate!(0x5b, Rx, Operation::S),
    candidate!(0x5c, Rx, Operation::M),
    candidate!(0x5d, Rx, Operation::D),
    candidate!(0x5e, Rx, Operation::Al),
    candidate!(0x5f, Rx, Operation::Sl),
    candidate!(0x60, Rx, Operation::FloatRx(FloatRxOperation::Op60)),
    candidate!(0x68, Rx, Operation::FloatRx(FloatRxOperation::Op68)),
    candidate!(0x69, Rx, Operation::FloatRx(FloatRxOperation::Op69)),
    candidate!(0x6a, Rx, Operation::FloatRx(FloatRxOperation::Op6a)),
    candidate!(0x6b, Rx, Operation::FloatRx(FloatRxOperation::Op6b)),
    candidate!(0x6c, Rx, Operation::FloatRx(FloatRxOperation::Op6c)),
    candidate!(0x6d, Rx, Operation::FloatRx(FloatRxOperation::Op6d)),
    candidate!(0x70, Rx, Operation::FloatRx(FloatRxOperation::Op70)),
    candidate!(0x78, Rx, Operation::FloatRx(FloatRxOperation::Op78)),
    candidate!(0x79, Rx, Operation::FloatRx(FloatRxOperation::Op79)),
    candidate!(0x7a, Rx, Operation::FloatRx(FloatRxOperation::Op7a)),
    candidate!(0x7b, Rx, Operation::FloatRx(FloatRxOperation::Op7b)),
    candidate!(0x7c, Rx, Operation::FloatRx(FloatRxOperation::Op7c)),
    candidate!(0x7d, Rx, Operation::FloatRx(FloatRxOperation::Op7d)),
    candidate!(0x86, Rs, Operation::Bxh),
    candidate!(0x87, Rs, Operation::Bxle),
    candidate!(0x88, Rs, Operation::Srl),
    candidate!(0x89, Rs, Operation::Sll),
    candidate!(0x8a, Rs, Operation::Sra),
    candidate!(0x8b, Rs, Operation::Sla),
    candidate!(0x8c, Rs, Operation::Srdl),
    candidate!(0x8d, Rs, Operation::Sldl),
    candidate!(0x8e, Rs, Operation::Srda),
    candidate!(0x8f, Rs, Operation::Slda),
    candidate!(0x90, Rs, Operation::Stm),
    candidate!(0x91, Si, Operation::Tm),
    candidate!(0x92, Si, Operation::Mvi),
    candidate!(0x94, Si, Operation::Ni),
    candidate!(0x95, Si, Operation::Cli),
    candidate!(0x96, Si, Operation::Oi),
    candidate!(0x97, Si, Operation::Xi),
    candidate!(0x98, Rs, Operation::Lm),
    candidate!(0xd2, Ss, Operation::Mvc),
    candidate!(0xd4, Ss, Operation::Nc),
    candidate!(0xd5, Ss, Operation::Clc),
    candidate!(0xd6, Ss, Operation::Oc),
    candidate!(0xd7, Ss, Operation::Xc),
];

/// Settled decode wires and the unique candidate they authorize.
pub(crate) struct DecodeAuthority {
    lines: [u8; 256],
    format: Option<Format>,
    operation: Option<Operation>,
}

impl DecodeAuthority {
    pub(crate) fn new(opcode: u8) -> Result<Self, System360Error> {
        let lines = gates::decode_byte(opcode);
        let (decoded_any, decoded_multiple) = gate_cardinality(&lines);
        if decoded_any != 1 || decoded_multiple != 0 {
            return gate_error("gate opcode decoder is not exactly one-hot");
        }
        let rr = or_gate(
            [lines[0x05], lines[0x06], lines[0x07], lines[0x0a]]
                .into_iter()
                .fold(0, or_gate),
            reduce_lines(&lines, 0x10, 0x3f),
        );
        let rx = reduce_lines(&lines, 0x40, 0x7f);
        let four = reduce_lines(&lines, 0x86, 0x98);
        let ss = [
            lines[0xd2],
            lines[0xd4],
            lines[0xd5],
            lines[0xd6],
            lines[0xd7],
        ]
        .into_iter()
        .fold(0, or_gate);
        let formats = [rr, rx, four, ss];
        let (format_any, format_multiple) = gate_cardinality(&formats);
        if format_multiple != 0 {
            return gate_error("gate format decoder selected multiple transport shapes");
        }
        let format = if rr == 1 {
            Some(Format::Rr)
        } else if rx == 1 {
            Some(Format::Rx)
        } else if four == 1 {
            None
        } else if ss == 1 {
            Some(Format::Ss)
        } else {
            None
        };

        let mut selected = None;
        let mut legal_any = 0;
        let mut legal_multiple = 0;
        let mut rs = 0;
        let mut si = 0;
        for candidate in CANDIDATES {
            let wire = lines[candidate.line];
            legal_multiple = or_gate(legal_multiple, and_gate(legal_any, wire));
            legal_any = or_gate(legal_any, wire);
            if candidate.format == Format::Rs {
                rs = or_gate(rs, wire);
            } else if candidate.format == Format::Si {
                si = or_gate(si, wire);
            }
            if wire == 1 {
                selected = Some(candidate.operation);
            }
        }
        if legal_multiple != 0 || and_gate(rs, si) != 0 {
            return gate_error("gate legal decoder selected multiple operations");
        }
        let format = if four == 1 {
            if rs == 1 {
                Some(Format::Rs)
            } else if si == 1 {
                Some(Format::Si)
            } else {
                // Reserved four-byte encodings keep their transport width so
                // truncation retains architectural precedence over opcode
                // rejection.  They have no executable format candidate.
                None
            }
        } else {
            format
        };
        debug_assert_eq!(
            format_any,
            u8::from(rr == 1 || rx == 1 || four == 1 || ss == 1)
        );
        let authority = Self {
            lines,
            format,
            operation: selected,
        };
        if !authority.format_matches_operation() {
            return gate_error("gate format and operation selectors disagree");
        }
        Ok(authority)
    }

    pub(crate) fn transport_width(&self) -> Option<usize> {
        if self.lines[0x05] == 1
            || self.lines[0x06] == 1
            || self.lines[0x07] == 1
            || self.lines[0x0a] == 1
            || reduce_lines(&self.lines, 0x10, 0x3f) == 1
        {
            Some(2)
        } else if reduce_lines(&self.lines, 0x40, 0x7f) == 1
            || reduce_lines(&self.lines, 0x86, 0x98) == 1
        {
            Some(4)
        } else if [
            self.lines[0xd2],
            self.lines[0xd4],
            self.lines[0xd5],
            self.lines[0xd6],
            self.lines[0xd7],
        ]
        .into_iter()
        .fold(0, or_gate)
            == 1
        {
            Some(6)
        } else {
            None
        }
    }

    pub(crate) fn operation(&self) -> Option<Operation> {
        self.operation
    }

    pub(crate) fn line(&self, line: usize) -> u8 {
        self.lines[line]
    }

    pub(crate) fn format_matches_operation(&self) -> bool {
        self.operation.is_none()
            || CANDIDATES
                .iter()
                .find(|candidate| Some(candidate.operation) == self.operation)
                .is_some_and(|candidate| Some(candidate.format) == self.format)
    }
}

fn reduce_lines(lines: &[u8; 256], first: usize, last: usize) -> u8 {
    (first..=last).fold(0, |selected, line| or_gate(selected, lines[line]))
}

fn gate_cardinality(selects: &[u8]) -> (u8, u8) {
    selects
        .iter()
        .copied()
        .fold((0, 0), |(any, multiple), select| {
            (
                or_gate(any, select),
                or_gate(multiple, and_gate(any, select)),
            )
        })
}

pub(crate) fn execute(
    before: &System360State,
    trace: &StepTrace,
    authority: &DecodeAuthority,
) -> Result<System360State, System360Error> {
    let operation = authority.operation().ok_or(System360Error::Specification {
        reason: "gate executor received no authorized operation",
    })?;
    if authority.transport_width() != Some(trace.raw.len()) || !authority.format_matches_operation()
    {
        return gate_error("gate format selection disagrees with the instruction transport");
    }
    let mut next = before.clone();
    next.pc = gates::add_24(before.pc, u32::from(trace.width));
    match operation {
        Operation::Balr
        | Operation::Bctr
        | Operation::Bcr
        | Operation::Svc
        | Operation::Lpr
        | Operation::Lnr
        | Operation::Ltr
        | Operation::Lcr
        | Operation::Nr
        | Operation::Clr
        | Operation::Or
        | Operation::Xr
        | Operation::Lr
        | Operation::Cr
        | Operation::Ar
        | Operation::Sr
        | Operation::Mr
        | Operation::Dr
        | Operation::Alr
        | Operation::Slr
        | Operation::FloatRr(_) => execute_rr(before, &mut next, &trace.raw, operation)?,
        Operation::Sth
        | Operation::La
        | Operation::Stc
        | Operation::Ic
        | Operation::Bal
        | Operation::Bct
        | Operation::Bc
        | Operation::Lh
        | Operation::Ch
        | Operation::Ah
        | Operation::Sh
        | Operation::Mh
        | Operation::St
        | Operation::N
        | Operation::Cl
        | Operation::O
        | Operation::X
        | Operation::L
        | Operation::C
        | Operation::A
        | Operation::S
        | Operation::M
        | Operation::D
        | Operation::Al
        | Operation::Sl
        | Operation::FloatRx(_) => execute_rx(before, &mut next, &trace.raw, operation)?,
        Operation::Bxh
        | Operation::Bxle
        | Operation::Srl
        | Operation::Sll
        | Operation::Sra
        | Operation::Sla
        | Operation::Srdl
        | Operation::Sldl
        | Operation::Srda
        | Operation::Slda
        | Operation::Stm
        | Operation::Lm => execute_rs(before, &mut next, &trace.raw, operation, authority)?,
        Operation::Tm
        | Operation::Mvi
        | Operation::Ni
        | Operation::Cli
        | Operation::Oi
        | Operation::Xi => execute_si(before, &mut next, &trace.raw, operation)?,
        Operation::Mvc | Operation::Nc | Operation::Clc | Operation::Oc | Operation::Xc => {
            execute_ss(before, &mut next, &trace.raw, operation)?
        }
    }
    Ok(next)
}

fn execute_rr(
    before: &System360State,
    next: &mut System360State,
    raw: &[u8],
    operation: Operation,
) -> Result<(), System360Error> {
    let r1 = usize::from(raw[1] >> 4);
    let r2 = usize::from(raw[1] & 15);
    let a = before.gprs[r1];
    let b = before.gprs[r2];
    match operation {
        Operation::Svc => next.halted = true,
        Operation::Lpr => {
            next.gprs[r1] = if b & 0x8000_0000 == 0 {
                b
            } else {
                gates::negate_32(b).0
            };
            next.condition_code = gates::signed_condition_code(next.gprs[r1]);
        }
        Operation::Lnr => {
            next.gprs[r1] = if b & 0x8000_0000 == 0 && b != 0 {
                gates::negate_32(b).0
            } else {
                b
            };
            next.condition_code = gates::signed_condition_code(next.gprs[r1]);
        }
        Operation::Ltr | Operation::Lr => {
            next.gprs[r1] = b;
            if operation == Operation::Ltr {
                next.condition_code = gates::signed_condition_code(b);
            }
        }
        Operation::Lcr => {
            let (value, overflow) = gates::negate_32(b);
            require_zero(
                overflow,
                "gate complement network overflowed an oracle-valid order",
            )?;
            next.gprs[r1] = value;
            next.condition_code = gates::signed_condition_code(value);
        }
        Operation::Nr | Operation::Or | Operation::Xr => {
            let value = match operation {
                Operation::Nr => gates::bit_and_32(a, b),
                Operation::Or => gates::bit_or_32(a, b),
                _ => gates::bit_xor_32(a, b),
            };
            next.gprs[r1] = value;
            next.condition_code = gates::logical_condition_code(value);
        }
        Operation::Clr => next.condition_code = gates::unsigned_compare_condition_code(a, b),
        Operation::Cr => next.condition_code = gates::signed_compare_condition_code(a, b),
        Operation::Ar | Operation::Sr => {
            let (value, _, overflow) = if operation == Operation::Ar {
                gates::add_32(a, b, 0)
            } else {
                gates::add_32(a, gates::bit_not_32(b), 1)
            };
            require_zero(overflow, "gate signed ALU overflowed an oracle-valid order")?;
            next.gprs[r1] = value;
            next.condition_code = gates::signed_condition_code(value);
        }
        Operation::Alr | Operation::Slr => {
            let (value, carry, _) = if operation == Operation::Alr {
                gates::add_32(a, b, 0)
            } else {
                gates::add_32(a, gates::bit_not_32(b), 1)
            };
            next.gprs[r1] = value;
            next.condition_code = (carry << 1) | gates::logical_condition_code(value);
        }
        Operation::Mr => {
            let product = gates::multiply_signed_32(before.gprs[r1 + 1], b);
            next.gprs[r1] = (product >> 32) as u32;
            next.gprs[r1 + 1] = product as u32;
        }
        Operation::Dr => {
            let dividend = (u64::from(a) << 32) | u64::from(before.gprs[r1 + 1]);
            let (remainder, quotient) = gates::divide_signed_64_by_32(dividend, b).ok_or(
                System360Error::Specification {
                    reason: "gate divider rejected an oracle-valid order",
                },
            )?;
            next.gprs[r1] = remainder;
            next.gprs[r1 + 1] = quotient;
        }
        Operation::Balr => {
            next.gprs[r1] = next.pc;
            next.pc = gates::select_u32(
                next.pc,
                b & ADDRESS_MASK,
                gates::register_field_nonzero(r2 as u8),
            );
        }
        Operation::Bctr => {
            let value = gates::add_32(a, u32::MAX, 0).0;
            next.gprs[r1] = value;
            next.pc = gates::select_u32(
                next.pc,
                b & ADDRESS_MASK,
                gates::count_branch_selected(value, r2 as u8),
            );
        }
        Operation::Bcr => {
            next.pc = gates::select_u32(
                next.pc,
                b & ADDRESS_MASK,
                gates::branch_selected(r1 as u8, before.condition_code),
            );
        }
        Operation::FloatRr(float_operation) => {
            execute_float_rr(before, next, float_operation, r1, r2)?;
        }
        _ => return gate_error("gate RR decoder accepted an unsupported opcode"),
    }
    Ok(())
}

fn execute_rx(
    before: &System360State,
    next: &mut System360State,
    raw: &[u8],
    operation: Operation,
) -> Result<(), System360Error> {
    let register = usize::from(raw[1] >> 4);
    let address = effective_address_rx(raw, before) as usize;
    let a = before.gprs[register];
    if let Operation::FloatRx(float_operation) = operation {
        return execute_float_rx(before, next, float_operation, register, address);
    }
    let word = || read_be32(&before.memory, address);
    let half = || i32::from(read_be16(&before.memory, address) as i16) as u32;
    match operation {
        Operation::Sth => write_bytes(&mut next.memory, address, &(a as u16).to_be_bytes()),
        Operation::Stc => next.memory[address] = a as u8,
        Operation::St => write_bytes(&mut next.memory, address, &a.to_be_bytes()),
        Operation::La => next.gprs[register] = address as u32,
        Operation::Ic => {
            next.gprs[register] = (a & 0xffff_ff00) | u32::from(before.memory[address]);
        }
        Operation::Lh => next.gprs[register] = half(),
        Operation::L => next.gprs[register] = word(),
        Operation::Ah | Operation::Sh | Operation::A | Operation::S => {
            let b = if matches!(operation, Operation::Ah | Operation::Sh) {
                half()
            } else {
                word()
            };
            let (value, _, overflow) = if matches!(operation, Operation::Ah | Operation::A) {
                gates::add_32(a, b, 0)
            } else {
                gates::add_32(a, gates::bit_not_32(b), 1)
            };
            require_zero(overflow, "gate RX ALU overflowed an oracle-valid order")?;
            next.gprs[register] = value;
            next.condition_code = gates::signed_condition_code(value);
        }
        Operation::Ch => next.condition_code = gates::signed_compare_condition_code(a, half()),
        Operation::C => next.condition_code = gates::signed_compare_condition_code(a, word()),
        Operation::Cl => next.condition_code = gates::unsigned_compare_condition_code(a, word()),
        Operation::N | Operation::O | Operation::X => {
            let value = match operation {
                Operation::N => gates::bit_and_32(a, word()),
                Operation::O => gates::bit_or_32(a, word()),
                _ => gates::bit_xor_32(a, word()),
            };
            next.gprs[register] = value;
            next.condition_code = gates::logical_condition_code(value);
        }
        Operation::Al | Operation::Sl => {
            let b = word();
            let (value, carry, _) = if operation == Operation::Al {
                gates::add_32(a, b, 0)
            } else {
                gates::add_32(a, gates::bit_not_32(b), 1)
            };
            next.gprs[register] = value;
            next.condition_code = (carry << 1) | gates::logical_condition_code(value);
        }
        Operation::M => {
            let product = gates::multiply_signed_32(before.gprs[register + 1], word());
            next.gprs[register] = (product >> 32) as u32;
            next.gprs[register + 1] = product as u32;
        }
        Operation::Mh => next.gprs[register] = gates::multiply_signed_32(a, half()) as u32,
        Operation::D => {
            let dividend = (u64::from(a) << 32) | u64::from(before.gprs[register + 1]);
            let (remainder, quotient) = gates::divide_signed_64_by_32(dividend, word()).ok_or(
                System360Error::Specification {
                    reason: "gate RX divider rejected an oracle-valid order",
                },
            )?;
            next.gprs[register] = remainder;
            next.gprs[register + 1] = quotient;
        }
        Operation::Bal => {
            next.gprs[register] = next.pc;
            next.pc = address as u32;
        }
        Operation::Bct => {
            let decremented = gates::add_32(a, u32::MAX, 0).0;
            next.gprs[register] = decremented;
            next.pc = gates::select_u32(next.pc, address as u32, gates::nonzero_32(decremented));
        }
        Operation::Bc => {
            next.pc = gates::select_u32(
                next.pc,
                address as u32,
                gates::branch_selected(register as u8, before.condition_code),
            );
        }
        _ => return gate_error("gate RX decoder accepted an unsupported opcode"),
    }
    Ok(())
}

fn execute_rs(
    before: &System360State,
    next: &mut System360State,
    raw: &[u8],
    operation: Operation,
    authority: &DecodeAuthority,
) -> Result<(), System360Error> {
    let r1 = usize::from(raw[1] >> 4);
    let r3 = usize::from(raw[1] & 15);
    let address = effective_address_base(raw, 2, before) as usize;
    let count = (address as u32) & 63;
    let source = before.gprs[r1];
    match operation {
        Operation::Stm => {
            for offset in 0..register_span(r1, r3) {
                write_bytes(
                    &mut next.memory,
                    address + offset * 4,
                    &before.gprs[(r1 + offset) & 15].to_be_bytes(),
                );
            }
        }
        Operation::Lm => {
            for offset in 0..register_span(r1, r3) {
                next.gprs[(r1 + offset) & 15] = read_be32(&before.memory, address + offset * 4);
            }
        }
        Operation::Srl => next.gprs[r1] = gates::shift_right_32(source, count),
        Operation::Sll => next.gprs[r1] = gates::shift_left_32(source, count),
        Operation::Sra => {
            let value = gates::arithmetic_right_32(source, count);
            next.gprs[r1] = value;
            next.condition_code = gates::signed_condition_code(value);
        }
        Operation::Sla => {
            let value =
                (source & 0x8000_0000) | (gates::shift_left_32(source, count) & 0x7fff_ffff);
            next.gprs[r1] = value;
            next.condition_code = gates::signed_condition_code(value);
        }
        Operation::Srdl | Operation::Sldl | Operation::Srda | Operation::Slda => {
            let pair = (u64::from(source) << 32) | u64::from(before.gprs[r1 + 1]);
            let value = match operation {
                Operation::Srdl => gates::shift_right_64(pair, count),
                Operation::Sldl => gates::shift_left_64(pair, count),
                Operation::Srda => gates::arithmetic_right_64(pair, count),
                _ => {
                    (pair & 0x8000_0000_0000_0000)
                        | (gates::shift_left_64(pair, count) & 0x7fff_ffff_ffff_ffff)
                }
            };
            next.gprs[r1] = (value >> 32) as u32;
            next.gprs[r1 + 1] = value as u32;
            if matches!(operation, Operation::Srda | Operation::Slda) {
                next.condition_code = gates::signed_condition_code_64(value);
            }
        }
        Operation::Bxh | Operation::Bxle => {
            let compare = if r3.is_multiple_of(2) { r3 + 1 } else { r3 };
            let incremented = gates::add_32(source, before.gprs[r3], 0).0;
            next.gprs[r1] = incremented;
            let comparison =
                gates::signed_compare_condition_code(incremented, before.gprs[compare]);
            next.pc = gates::select_u32(
                next.pc,
                address as u32,
                gates::index_branch_selected(
                    authority.line(0x86),
                    authority.line(0x87),
                    comparison,
                ),
            );
        }
        _ => return gate_error("gate RS decoder accepted an unsupported opcode"),
    }
    Ok(())
}

fn execute_si(
    before: &System360State,
    next: &mut System360State,
    raw: &[u8],
    operation: Operation,
) -> Result<(), System360Error> {
    let immediate = raw[1];
    let address = effective_address_base(raw, 2, before) as usize;
    let a = u32::from(before.memory[address]);
    let b = u32::from(immediate);
    match operation {
        Operation::Tm => {
            let value = gates::bit_and_32(a, b) as u8;
            next.condition_code = if value == 0 {
                0
            } else if value == immediate {
                3
            } else {
                1
            };
        }
        Operation::Mvi => next.memory[address] = immediate,
        Operation::Ni | Operation::Oi | Operation::Xi => {
            let value = match operation {
                Operation::Ni => gates::bit_and_32(a, b),
                Operation::Oi => gates::bit_or_32(a, b),
                _ => gates::bit_xor_32(a, b),
            } as u8;
            next.memory[address] = value;
            next.condition_code = gates::logical_condition_code(u32::from(value));
        }
        Operation::Cli => next.condition_code = gates::unsigned_compare_condition_code(a, b),
        _ => return gate_error("gate SI decoder accepted an unsupported opcode"),
    }
    Ok(())
}

fn execute_ss(
    before: &System360State,
    next: &mut System360State,
    raw: &[u8],
    operation: Operation,
) -> Result<(), System360Error> {
    let length = usize::from(raw[1]) + 1;
    let destination = effective_address_base(raw, 2, before) as usize;
    let source = effective_address_base(raw, 4, before) as usize;
    match operation {
        Operation::Mvc => {
            for offset in 0..length {
                next.memory[destination + offset] = next.memory[source + offset];
            }
        }
        Operation::Clc => {
            next.condition_code = compare_bytes(
                &before.memory[destination..destination + length],
                &before.memory[source..source + length],
            );
        }
        Operation::Nc | Operation::Oc | Operation::Xc => {
            let mut any = 0_u8;
            for offset in 0..length {
                let a = u32::from(next.memory[destination + offset]);
                let b = u32::from(next.memory[source + offset]);
                let value = match operation {
                    Operation::Nc => gates::bit_and_32(a, b),
                    Operation::Oc => gates::bit_or_32(a, b),
                    _ => gates::bit_xor_32(a, b),
                } as u8;
                next.memory[destination + offset] = value;
                any |= value;
            }
            next.condition_code = gates::logical_condition_code(u32::from(any));
        }
        _ => return gate_error("gate SS decoder accepted an unsupported opcode"),
    }
    Ok(())
}

fn execute_float_rr(
    before: &System360State,
    next: &mut System360State,
    operation: FloatRrOperation,
    r1: usize,
    r2: usize,
) -> Result<(), System360Error> {
    let slot1 = r1 / 2;
    let slot2 = r2 / 2;
    let long = operation.is_long();
    let digits = if long { 14 } else { 6 };
    let raw_a = before.fprs[slot1];
    let raw_b = before.fprs[slot2];
    let a = GateHfp::decode(raw_a, long);
    let b = GateHfp::decode(raw_b, long);
    match operation {
        FloatRrOperation::Op28 | FloatRrOperation::Op38 => store_float(
            next,
            slot1,
            gates::hfp_with_sign(raw_b, long, raw_b >> 63 != 0),
            long,
        ),
        FloatRrOperation::Op22 | FloatRrOperation::Op32 => {
            let value = gates::hfp_with_sign(raw_b, long, raw_b >> 63 != 0);
            store_float(next, slot1, value, long);
            next.condition_code = gates::hfp_condition_code(value, long);
        }
        FloatRrOperation::Op23 | FloatRrOperation::Op33 => {
            let value = gates::hfp_with_sign(raw_b, long, raw_b >> 63 == 0);
            store_float(next, slot1, value, long);
            next.condition_code = gates::hfp_condition_code(value, long);
        }
        FloatRrOperation::Op20 | FloatRrOperation::Op30 => {
            let value = gates::hfp_with_sign(raw_b, long, false);
            store_float(next, slot1, value, long);
            next.condition_code = gates::hfp_condition_code(value, long);
        }
        FloatRrOperation::Op21 | FloatRrOperation::Op31 => {
            let value =
                gates::hfp_with_sign(raw_b, long, gates::hfp_fraction_nonzero(raw_b, long) == 1);
            store_float(next, slot1, value, long);
            next.condition_code = gates::hfp_condition_code(value, long);
        }
        FloatRrOperation::Op2a
        | FloatRrOperation::Op3a
        | FloatRrOperation::Op2b
        | FloatRrOperation::Op3b
        | FloatRrOperation::Op24
        | FloatRrOperation::Op34
        | FloatRrOperation::Op2c
        | FloatRrOperation::Op3c
        | FloatRrOperation::Op2d
        | FloatRrOperation::Op3d => {
            let value = match operation {
                FloatRrOperation::Op2a | FloatRrOperation::Op3a => a.add(b, digits),
                FloatRrOperation::Op2b | FloatRrOperation::Op3b => {
                    a.add(b.with_complemented_sign(), digits)
                }
                FloatRrOperation::Op24 | FloatRrOperation::Op34 => a.halve(digits),
                FloatRrOperation::Op2c | FloatRrOperation::Op3c => a.multiply(b, digits),
                _ => a.divide(b, digits),
            }
            .and_then(|value| value.encode(long))
            .ok_or(System360Error::Specification {
                reason: "gate HFP network rejected an oracle-valid RR order",
            })?;
            store_float(next, slot1, value, long);
            if matches!(
                operation,
                FloatRrOperation::Op2a
                    | FloatRrOperation::Op3a
                    | FloatRrOperation::Op2b
                    | FloatRrOperation::Op3b
            ) {
                next.condition_code = gates::hfp_condition_code(value, long);
            }
        }
        FloatRrOperation::Op29 | FloatRrOperation::Op39 => {
            next.condition_code = a.compare(b);
        }
    }
    Ok(())
}

fn execute_float_rx(
    before: &System360State,
    next: &mut System360State,
    operation: FloatRxOperation,
    register: usize,
    address: usize,
) -> Result<(), System360Error> {
    let slot = register / 2;
    let long = operation.is_long();
    let digits = if long { 14 } else { 6 };
    let memory_raw = if long {
        read_be64(&before.memory, address)
    } else {
        u64::from(read_be32(&before.memory, address)) << 32
    };
    let a = GateHfp::decode(before.fprs[slot], long);
    let b = GateHfp::decode(memory_raw, long);
    match operation {
        FloatRxOperation::Op68 | FloatRxOperation::Op78 => store_float(
            next,
            slot,
            gates::hfp_with_sign(memory_raw, long, memory_raw >> 63 != 0),
            long,
        ),
        FloatRxOperation::Op60 => {
            write_bytes(&mut next.memory, address, &before.fprs[slot].to_be_bytes());
        }
        FloatRxOperation::Op70 => write_bytes(
            &mut next.memory,
            address,
            &((before.fprs[slot] >> 32) as u32).to_be_bytes(),
        ),
        FloatRxOperation::Op6a
        | FloatRxOperation::Op7a
        | FloatRxOperation::Op6b
        | FloatRxOperation::Op7b
        | FloatRxOperation::Op6c
        | FloatRxOperation::Op7c
        | FloatRxOperation::Op6d
        | FloatRxOperation::Op7d => {
            let value = match operation {
                FloatRxOperation::Op6a | FloatRxOperation::Op7a => a.add(b, digits),
                FloatRxOperation::Op6b | FloatRxOperation::Op7b => {
                    a.add(b.with_complemented_sign(), digits)
                }
                FloatRxOperation::Op6c | FloatRxOperation::Op7c => a.multiply(b, digits),
                _ => a.divide(b, digits),
            }
            .and_then(|value| value.encode(long))
            .ok_or(System360Error::Specification {
                reason: "gate HFP network rejected an oracle-valid RX order",
            })?;
            store_float(next, slot, value, long);
            if matches!(
                operation,
                FloatRxOperation::Op6a
                    | FloatRxOperation::Op7a
                    | FloatRxOperation::Op6b
                    | FloatRxOperation::Op7b
            ) {
                next.condition_code = gates::hfp_condition_code(value, long);
            }
        }
        FloatRxOperation::Op69 | FloatRxOperation::Op79 => {
            next.condition_code = a.compare(b);
        }
    }
    Ok(())
}

fn store_float(state: &mut System360State, slot: usize, value: u64, long: bool) {
    state.fprs[slot] = if long {
        value
    } else {
        (value & 0xffff_ffff_0000_0000) | (state.fprs[slot] & 0xffff_ffff)
    };
}

fn effective_address_base(raw: &[u8], offset: usize, state: &System360State) -> u32 {
    let base = usize::from(raw[offset] >> 4);
    let displacement = (u32::from(raw[offset] & 15) << 8) | u32::from(raw[offset + 1]);
    gates::add_24(if base == 0 { 0 } else { state.gprs[base] }, displacement)
}

fn effective_address_rx(raw: &[u8], state: &System360State) -> u32 {
    let index = usize::from(raw[1] & 15);
    gates::add_24(
        if index == 0 { 0 } else { state.gprs[index] },
        effective_address_base(raw, 2, state),
    )
}

fn read_be16(memory: &[u8], address: usize) -> u16 {
    u16::from_be_bytes([memory[address], memory[address + 1]])
}

fn read_be32(memory: &[u8], address: usize) -> u32 {
    u32::from_be_bytes(
        memory[address..address + 4]
            .try_into()
            .expect("checked word"),
    )
}

fn read_be64(memory: &[u8], address: usize) -> u64 {
    u64::from_be_bytes(
        memory[address..address + 8]
            .try_into()
            .expect("checked doubleword"),
    )
}

fn write_bytes(memory: &mut [u8], address: usize, value: &[u8]) {
    memory[address..address + value.len()].copy_from_slice(value);
}

fn register_span(first: usize, last: usize) -> usize {
    if last >= first {
        last - first + 1
    } else {
        17 - first + last
    }
}

fn compare_bytes(a: &[u8], b: &[u8]) -> u8 {
    for (&left, &right) in a.iter().zip(b) {
        let condition = gates::unsigned_compare_condition_code(u32::from(left), u32::from(right));
        if condition != 0 {
            return condition;
        }
    }
    0
}

fn require_zero(value: u8, reason: &'static str) -> Result<(), System360Error> {
    if value == 0 {
        Ok(())
    } else {
        gate_error(reason)
    }
}

fn gate_error<T>(reason: &'static str) -> Result<T, System360Error> {
    Err(System360Error::Specification { reason })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_transport_width(opcode: u8) -> Option<usize> {
        match opcode {
            0x05..=0x07 | 0x0a | 0x10..=0x3f => Some(2),
            0x40..=0x7f | 0x86..=0x98 => Some(4),
            0xd2 | 0xd4..=0xd7 => Some(6),
            _ => None,
        }
    }

    #[test]
    fn every_first_byte_and_transport_width_fail_closed() {
        for opcode in u8::MIN..=u8::MAX {
            let authority = DecodeAuthority::new(opcode).expect("one-hot decode must settle");
            assert_eq!(
                authority.transport_width(),
                expected_transport_width(opcode)
            );
            assert_eq!(
                authority.operation().is_some(),
                CANDIDATES
                    .iter()
                    .any(|candidate| candidate.line == usize::from(opcode)),
                "opcode {opcode:#04x}"
            );
            assert_eq!(authority.lines.into_iter().sum::<u8>(), 1);
            assert!(authority.format_matches_operation());

            for width in [2_usize, 4, 6] {
                if authority.operation().is_some()
                    && authority
                        .transport_width()
                        .is_some_and(|expected| expected != width)
                {
                    let state = ibm_system360_simulator::System360Simulator::new().get_state();
                    let trace = StepTrace {
                        pc_before: 0,
                        pc_after: width as u32,
                        raw: vec![opcode; width],
                        width: width as u8,
                        mnemonic: String::new(),
                        description: String::new(),
                    };
                    assert!(matches!(
                        execute(&state, &trace, &authority),
                        Err(System360Error::Specification { .. })
                    ));
                }
            }
        }
    }
}
