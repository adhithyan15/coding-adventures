use std::collections::VecDeque;

const MASK_20: i32 = (1 << 20) - 1;
const DATA_MASK: i32 = (1 << 19) - 1;
const SIGN_BIT: i32 = 1 << 19;
const ADDR_MASK: i32 = 0x1fff;
const X_MASK: i32 = 0x7fff;
const MODIFIER_SHIFT: i32 = 13;
const MODIFIER_MASK: i32 = 0x03 << MODIFIER_SHIFT;
const N_MASK: i32 = 0x3f;
const DECIMAL_FLAG_BIT: i32 = 1 << 18;
const CLOCK_DAY_SIXTHS: i32 = 24 * 60 * 60 * 6;
const CLOCK_WORD_MODULUS: u64 = 1 << 19;
const DECIMAL_SINGLE_MODULUS: i32 = 1_000;
const DECIMAL_DOUBLE_MODULUS: i64 = 1_000_000;
const SXG_BASE: i32 = 0o2506003;
const SXG_GROUP_SHIFT: i32 = 3;
const SXG_GROUP_MASK: i32 = 0x1f << SXG_GROUP_SHIFT;
const DOUBLE_DATA_BITS: u32 = 38;
const DOUBLE_WORD_BITS: u32 = 39;
const DOUBLE_DATA_MASK: i64 = (1_i64 << DOUBLE_DATA_BITS) - 1;
const DOUBLE_WORD_MASK: i64 = (1_i64 << DOUBLE_WORD_BITS) - 1;
const WORD_BYTES: usize = 3;
const MIN_MEMORY_WORDS: i32 = 4_096;
const MAX_MEMORY_WORDS: i32 = 16_384;
const MAX_CARD_RECORD_WORDS: usize = 27;
const MAX_CARD_QUEUE_DEPTH: usize = 64;

const OP_LDA: i32 = 0o00;
const OP_ADD: i32 = 0o01;
const OP_SUB: i32 = 0o02;
const OP_STA: i32 = 0o03;
const OP_BXL: i32 = 0o04;
const OP_BXH: i32 = 0o05;
const OP_LDX: i32 = 0o06;
const OP_SPB: i32 = 0o07;
const OP_DLD: i32 = 0o10;
const OP_DAD: i32 = 0o11;
const OP_DSU: i32 = 0o12;
const OP_DST: i32 = 0o13;
const OP_INX: i32 = 0o14;
const OP_MPY: i32 = 0o15;
const OP_DVD: i32 = 0o16;
const OP_STX: i32 = 0o17;
const OP_EXT: i32 = 0o20;
const OP_CAB: i32 = 0o21;
const OP_DCB: i32 = 0o22;
const OP_ORY: i32 = 0o23;
const OP_MOV: i32 = 0o24;
const OP_RCD: i32 = 0o25;
const OP_BRU: i32 = 0o26;
const OP_STO: i32 = 0o27;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Indicators {
    pub carry: bool,
    pub zero: bool,
    pub negative: bool,
    pub overflow: bool,
    pub parity_error: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub a: i32,
    pub q: i32,
    pub m: i32,
    pub n: i32,
    pub pc: i32,
    pub ir: i32,
    pub indicators: Indicators,
    pub overflow: bool,
    pub parity_error: bool,
    pub decimal_mode: bool,
    pub decimal_carry: i32,
    pub automatic_interrupt_mode: bool,
    pub clock_sixths: i32,
    pub selected_x_group: usize,
    pub n_ready: bool,
    pub typewriter_power: bool,
    pub control_switches: i32,
    pub x_words: Vec<i32>,
    pub halted: bool,
    pub memory: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    pub address: i32,
    pub instruction_word: i32,
    pub mnemonic: String,
    pub a_before: i32,
    pub a_after: i32,
    pub q_before: i32,
    pub q_after: i32,
    pub effective_address: Option<i32>,
}

#[derive(Debug, Clone)]
struct DecodedInstruction {
    mnemonic: &'static str,
    modifier: Option<i32>,
    address: Option<i32>,
    count: Option<i32>,
    sxg_group: Option<usize>,
    fixed_word: bool,
}

fn base_opcode_name(opcode: i32) -> Option<&'static str> {
    Some(match opcode {
        OP_LDA => "LDA",
        OP_ADD => "ADD",
        OP_SUB => "SUB",
        OP_STA => "STA",
        OP_BXL => "BXL",
        OP_BXH => "BXH",
        OP_LDX => "LDX",
        OP_SPB => "SPB",
        OP_DLD => "DLD",
        OP_DAD => "DAD",
        OP_DSU => "DSU",
        OP_DST => "DST",
        OP_INX => "INX",
        OP_MPY => "MPY",
        OP_DVD => "DVD",
        OP_STX => "STX",
        OP_EXT => "EXT",
        OP_CAB => "CAB",
        OP_DCB => "DCB",
        OP_ORY => "ORY",
        OP_MOV => "MOV",
        OP_RCD => "RCD",
        OP_BRU => "BRU",
        OP_STO => "STO",
        _ => return None,
    })
}

fn fixed_word(mnemonic: &str) -> Option<i32> {
    Some(match mnemonic {
        "OFF" => 0o2500005,
        "TYP" => 0o2500006,
        "TON" => 0o2500007,
        "RCS" => 0o2500011,
        "HPT" => 0o2500016,
        "LDZ" => 0o2504002,
        "LDO" => 0o2504022,
        "LMO" => 0o2504102,
        "CPL" => 0o2504502,
        "NEG" => 0o2504522,
        "CHS" => 0o2504040,
        "NOP" => 0o2504012,
        "LAQ" => 0o2504001,
        "LQA" => 0o2504004,
        "XAQ" => 0o2504005,
        "MAQ" => 0o2504006,
        "ADO" => 0o2504032,
        "SBO" => 0o2504112,
        "LAC" => 0o2504202,
        "LCA" => 0o2504210,
        "SET_DECMODE" => 0o2506011,
        "SET_BINMODE" => 0o2506012,
        "SET_PST" => 0o2506015,
        "SET_PBK" => 0o2506016,
        "BOD" => 0o2514000,
        "BEV" => 0o2516000,
        "BMI" => 0o2514001,
        "BPL" => 0o2516001,
        "BZE" => 0o2514002,
        "BNZ" => 0o2516002,
        "BOV" => 0o2514003,
        "BNO" => 0o2516003,
        "BPE" => 0o2514004,
        "BPC" => 0o2516004,
        "BNR" => 0o2514005,
        "BNN" => 0o2516005,
        _ => return None,
    })
}

fn fixed_name(word: i32) -> Option<&'static str> {
    Some(match word {
        0o2500005 => "OFF",
        0o2500006 => "TYP",
        0o2500007 => "TON",
        0o2500011 => "RCS",
        0o2500016 => "HPT",
        0o2504002 => "LDZ",
        0o2504022 => "LDO",
        0o2504102 => "LMO",
        0o2504502 => "CPL",
        0o2504522 => "NEG",
        0o2504040 => "CHS",
        0o2504012 => "NOP",
        0o2504001 => "LAQ",
        0o2504004 => "LQA",
        0o2504005 => "XAQ",
        0o2504006 => "MAQ",
        0o2504032 => "ADO",
        0o2504112 => "SBO",
        0o2504202 => "LAC",
        0o2504210 => "LCA",
        0o2506011 => "SET_DECMODE",
        0o2506012 => "SET_BINMODE",
        0o2506015 => "SET_PST",
        0o2506016 => "SET_PBK",
        0o2514000 => "BOD",
        0o2516000 => "BEV",
        0o2514001 => "BMI",
        0o2516001 => "BPL",
        0o2514002 => "BZE",
        0o2516002 => "BNZ",
        0o2514003 => "BOV",
        0o2516003 => "BNO",
        0o2514004 => "BPE",
        0o2516004 => "BPC",
        0o2514005 => "BNR",
        0o2516005 => "BNN",
        _ => return None,
    })
}

fn shift_base(mnemonic: &str) -> Option<i32> {
    Some(match mnemonic {
        "SRA" => 0o2510000,
        "SNA" => 0o2510100,
        "SCA" => 0o2510040,
        "SAN" => 0o2510400,
        "SRD" => 0o2511000,
        "NAQ" => 0o2511100,
        "SCD" => 0o2511200,
        "ANQ" => 0o2511400,
        "SLA" => 0o2512000,
        "SLD" => 0o2512200,
        "NOR" => 0o2513000,
        "DNO" => 0o2513200,
        _ => return None,
    })
}

fn typewriter_char(code: i32) -> Option<&'static str> {
    Some(match code {
        0o00 => "0",
        0o01 => "1",
        0o02 => "2",
        0o03 => "3",
        0o04 => "4",
        0o05 => "5",
        0o06 => "6",
        0o07 => "7",
        0o10 => "8",
        0o11 => "9",
        0o13 => "/",
        0o21 => "A",
        0o22 => "B",
        0o23 => "C",
        0o24 => "D",
        0o25 => "E",
        0o26 => "F",
        0o27 => "G",
        0o30 => "H",
        0o31 => "I",
        0o33 => "-",
        0o40 => ".",
        0o41 => "J",
        0o42 => "K",
        0o43 => "L",
        0o44 => "M",
        0o45 => "N",
        0o46 => "O",
        0o47 => "P",
        0o50 => "Q",
        0o51 => "R",
        0o53 => "$",
        0o60 => " ",
        0o62 => "S",
        0o63 => "T",
        0o64 => "U",
        0o65 => "V",
        0o66 => "W",
        0o67 => "X",
        0o70 => "Y",
        0o71 => "Z",
        _ => return None,
    })
}

fn to_signed20(value: i32) -> i32 {
    let word = value & MASK_20;
    if (word & SIGN_BIT) != 0 {
        word - (1 << 20)
    } else {
        word
    }
}

fn from_signed20(value: i32) -> i32 {
    value & MASK_20
}

fn decimal_digits(word: i32) -> Result<i32, String> {
    let hundreds = (word >> 12) & 0x0f;
    let tens = (word >> 6) & 0x0f;
    let ones = word & 0x0f;
    if hundreds > 9 || tens > 9 || ones > 9 {
        return Err(format!(
            "invalid GE-225 BCD digits in word {:07o}: {hundreds}{tens}{ones}",
            word & MASK_20
        ));
    }
    Ok(hundreds * 100 + tens * 10 + ones)
}

fn encode_decimal_word(digits: i32, negative: bool, flagged: bool) -> i32 {
    let hundreds = digits / 100;
    let tens = (digits / 10) % 10;
    let ones = digits % 10;
    (if negative { SIGN_BIT } else { 0 })
        | (if flagged { DECIMAL_FLAG_BIT } else { 0 })
        | (hundreds << 12)
        | (tens << 6)
        | ones
}

fn signed_decimal(raw: i64, negative: bool, modulus: i64) -> i64 {
    if negative && raw != 0 {
        raw - modulus
    } else {
        raw
    }
}

fn wrap_flagged_decimal(total: i64, modulus: i64) -> (i64, bool, bool) {
    let overflow = !(-(modulus - 1)..=(modulus - 1)).contains(&total);
    let negative = if total >= modulus {
        true
    } else if total <= -modulus {
        false
    } else {
        total < 0
    };
    (total.rem_euclid(modulus), negative, overflow)
}

fn decimal_word_operation(
    accumulator: i32,
    operand: i32,
    subtract: bool,
    carry: i32,
) -> Result<(i32, i32, bool), String> {
    let left_raw = decimal_digits(accumulator)?;
    let right_raw = decimal_digits(operand)?;
    let accumulator_flagged = (accumulator & DECIMAL_FLAG_BIT) != 0;
    if (operand & DECIMAL_FLAG_BIT) != 0 && !accumulator_flagged {
        return Err("GE-225 decimal operand is flagged while A is unflagged".into());
    }

    if accumulator_flagged {
        let left = signed_decimal(
            i64::from(left_raw),
            (accumulator & SIGN_BIT) != 0,
            i64::from(DECIMAL_SINGLE_MODULUS),
        );
        let right = signed_decimal(
            i64::from(right_raw),
            (operand & SIGN_BIT) != 0,
            i64::from(DECIMAL_SINGLE_MODULUS),
        );
        let total = left + if subtract { -right } else { right } + i64::from(carry);
        let (digits, negative, overflow) =
            wrap_flagged_decimal(total, i64::from(DECIMAL_SINGLE_MODULUS));
        return Ok((
            encode_decimal_word(digits as i32, negative, true),
            0,
            overflow,
        ));
    }

    let total = left_raw + if subtract { -right_raw } else { right_raw } + carry;
    let next_carry = if total >= DECIMAL_SINGLE_MODULUS {
        1
    } else if total < 0 {
        -1
    } else {
        0
    };
    Ok((
        encode_decimal_word(total.rem_euclid(DECIMAL_SINGLE_MODULUS), false, false),
        next_carry,
        false,
    ))
}

fn decimal_pair_operation(
    a: i32,
    q: i32,
    high_operand: i32,
    low_operand: i32,
    subtract: bool,
    carry: i32,
) -> Result<(i32, i32, i32, bool), String> {
    let a_high = decimal_digits(a)?;
    let a_low = decimal_digits(q)?;
    let operand_high = decimal_digits(high_operand)?;
    let operand_low = decimal_digits(low_operand)?;
    let accumulator_flagged = (a & DECIMAL_FLAG_BIT) != 0;
    if (high_operand & DECIMAL_FLAG_BIT) != 0 && !accumulator_flagged {
        return Err("GE-225 double-decimal operand is flagged while A is unflagged".into());
    }

    let left_raw = i64::from(a_high * DECIMAL_SINGLE_MODULUS + a_low);
    let right_raw = i64::from(operand_high * DECIMAL_SINGLE_MODULUS + operand_low);
    if accumulator_flagged {
        let left = signed_decimal(left_raw, (a & SIGN_BIT) != 0, DECIMAL_DOUBLE_MODULUS);
        let right = signed_decimal(
            right_raw,
            (high_operand & SIGN_BIT) != 0,
            DECIMAL_DOUBLE_MODULUS,
        );
        let total = left + if subtract { -right } else { right } + i64::from(carry);
        let (raw, negative, overflow) = wrap_flagged_decimal(total, DECIMAL_DOUBLE_MODULUS);
        let high = (raw / i64::from(DECIMAL_SINGLE_MODULUS)) as i32;
        let low = (raw % i64::from(DECIMAL_SINGLE_MODULUS)) as i32;
        return Ok((
            encode_decimal_word(high, negative, true),
            encode_decimal_word(low, false, false),
            0,
            overflow,
        ));
    }

    let total = left_raw + if subtract { -right_raw } else { right_raw } + i64::from(carry);
    let next_carry = if total >= DECIMAL_DOUBLE_MODULUS {
        1
    } else if total < 0 {
        -1
    } else {
        0
    };
    let raw = total.rem_euclid(DECIMAL_DOUBLE_MODULUS);
    Ok((
        encode_decimal_word(
            (raw / i64::from(DECIMAL_SINGLE_MODULUS)) as i32,
            false,
            false,
        ),
        encode_decimal_word(
            (raw % i64::from(DECIMAL_SINGLE_MODULUS)) as i32,
            false,
            false,
        ),
        next_carry,
        false,
    ))
}

fn sign_of(word: i32) -> i32 {
    if (word & SIGN_BIT) != 0 {
        1
    } else {
        0
    }
}
fn with_sign(word: i32, sign: i32) -> i32 {
    ((sign & 1) << 19) | (word & DATA_MASK)
}

fn to_signed_double(high: i32, low: i32) -> i64 {
    let raw = (i64::from(high & MASK_20) << 19) | i64::from(low & DATA_MASK);
    if (high & SIGN_BIT) != 0 {
        raw - (1_i64 << DOUBLE_WORD_BITS)
    } else {
        raw
    }
}

fn split_signed_double(value: i64) -> (i32, i32) {
    let raw = value & DOUBLE_WORD_MASK;
    let high = ((raw >> 19) & i64::from(MASK_20)) as i32;
    let low = with_sign((raw & i64::from(DATA_MASK)) as i32, sign_of(high));
    (high, low)
}

fn arith_compare(left: i32, right: i32) -> i32 {
    match to_signed20(left).cmp(&to_signed20(right)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 0,
    }
}

fn arith_compare_double(left_high: i32, left_low: i32, right_high: i32, right_low: i32) -> i32 {
    match to_signed_double(left_high, left_low).cmp(&to_signed_double(right_high, right_low)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 0,
    }
}

pub fn encode_instruction(opcode: i32, modifier: i32, address: i32) -> Result<i32, String> {
    if !(0..=0o37).contains(&opcode) {
        return Err(format!("opcode out of range: {opcode}"));
    }
    if !(0..=0o3).contains(&modifier) {
        return Err(format!("modifier out of range: {modifier}"));
    }
    if !(0..=ADDR_MASK).contains(&address) {
        return Err(format!("address out of range: {address}"));
    }
    Ok(((opcode & 0x1f) << 15) | ((modifier & 0x03) << 13) | (address & ADDR_MASK))
}

pub fn decode_instruction(word: i32) -> (i32, i32, i32) {
    let normalized = word & MASK_20;
    (
        (normalized >> 15) & 0x1f,
        (normalized >> 13) & 0x03,
        normalized & ADDR_MASK,
    )
}

pub fn assemble_fixed(mnemonic: &str) -> Result<i32, String> {
    if mnemonic == "SXG" {
        return Err("SXG requires a group; use assemble_select_x_group".into());
    }
    fixed_word(mnemonic).ok_or_else(|| format!("unknown fixed GE-225 instruction: {mnemonic}"))
}

pub fn assemble_shift(mnemonic: &str, count: i32) -> Result<i32, String> {
    assemble_shift_modified(mnemonic, count, 0)
}

pub fn assemble_fixed_modified(mnemonic: &str, modifier: i32) -> Result<i32, String> {
    if !(0..=3).contains(&modifier) {
        return Err(format!("modifier out of range: {modifier}"));
    }
    Ok(assemble_fixed(mnemonic)? | (modifier << MODIFIER_SHIFT))
}

pub fn assemble_shift_modified(mnemonic: &str, count: i32, modifier: i32) -> Result<i32, String> {
    if !(0..=0o37).contains(&count) {
        return Err(format!("shift count out of range: {count}"));
    }
    if !(0..=3).contains(&modifier) {
        return Err(format!("modifier out of range: {modifier}"));
    }
    shift_base(mnemonic)
        .map(|base| base | count | (modifier << MODIFIER_SHIFT))
        .ok_or_else(|| format!("unknown GE-225 shift instruction: {mnemonic}"))
}

pub fn assemble_select_x_group(group: i32) -> Result<i32, String> {
    if !(0..=31).contains(&group) {
        return Err(format!("X register group out of range: {group}"));
    }
    Ok(SXG_BASE | (group << SXG_GROUP_SHIFT))
}

pub fn pack_words(words: &[i32]) -> Vec<u8> {
    let mut blob = vec![0; words.len() * WORD_BYTES];
    for (index, word) in words.iter().enumerate() {
        let normalized = word & MASK_20;
        blob[index * WORD_BYTES] = ((normalized >> 16) & 0xff) as u8;
        blob[index * WORD_BYTES + 1] = ((normalized >> 8) & 0xff) as u8;
        blob[index * WORD_BYTES + 2] = (normalized & 0xff) as u8;
    }
    blob
}

pub fn unpack_words(program: &[u8]) -> Result<Vec<i32>, String> {
    if !program.len().is_multiple_of(WORD_BYTES) {
        return Err(format!(
            "GE-225 byte stream must be a multiple of {WORD_BYTES} bytes, got {}",
            program.len()
        ));
    }
    Ok(program
        .as_chunks::<WORD_BYTES>()
        .0
        .iter()
        .map(|chunk| {
            (((chunk[0] as i32) << 16) | ((chunk[1] as i32) << 8) | chunk[2] as i32) & MASK_20
        })
        .collect())
}

pub struct Simulator {
    memory_size: i32,
    memory: Vec<i32>,
    card_reader_queue: VecDeque<Vec<i32>>,
    a: i32,
    q: i32,
    m: i32,
    n: i32,
    pc: i32,
    ir: i32,
    overflow: bool,
    parity_error: bool,
    decimal_mode: bool,
    decimal_carry: i32,
    automatic_interrupt_mode: bool,
    clock_sixths: i32,
    selected_x_group: usize,
    n_ready: bool,
    typewriter_power: bool,
    typewriter_output: Vec<String>,
    control_switches: i32,
    halted: bool,
}

impl Simulator {
    pub fn new(memory_words: i32) -> Result<Self, String> {
        if !(MIN_MEMORY_WORDS..=MAX_MEMORY_WORDS).contains(&memory_words) {
            return Err(format!(
                "memory_words must be between {MIN_MEMORY_WORDS} and {MAX_MEMORY_WORDS}, got {memory_words}"
            ));
        }
        Ok(Self {
            memory_size: memory_words,
            memory: vec![0; memory_words as usize],
            card_reader_queue: VecDeque::new(),
            a: 0,
            q: 0,
            m: 0,
            n: 0,
            pc: 0,
            ir: 0,
            overflow: false,
            parity_error: false,
            decimal_mode: false,
            decimal_carry: 0,
            automatic_interrupt_mode: false,
            clock_sixths: 0,
            selected_x_group: 0,
            n_ready: true,
            typewriter_power: false,
            typewriter_output: vec![],
            control_switches: 0,
            halted: false,
        })
    }

    pub fn reset(&mut self) {
        self.a = 0;
        self.q = 0;
        self.m = 0;
        self.n = 0;
        self.pc = 0;
        self.ir = 0;
        self.overflow = false;
        self.parity_error = false;
        self.decimal_mode = false;
        self.decimal_carry = 0;
        self.automatic_interrupt_mode = false;
        self.clock_sixths = 0;
        self.selected_x_group = 0;
        self.n_ready = true;
        self.typewriter_power = false;
        self.typewriter_output.clear();
        self.control_switches = 0;
        self.halted = false;
    }

    pub fn get_state(&self) -> State {
        State {
            a: self.a,
            q: self.q,
            m: self.m,
            n: self.n,
            pc: self.pc,
            ir: self.ir,
            indicators: Indicators {
                carry: self.overflow,
                zero: self.a == 0,
                negative: (self.a & SIGN_BIT) != 0,
                overflow: self.overflow,
                parity_error: self.parity_error,
            },
            overflow: self.overflow,
            parity_error: self.parity_error,
            decimal_mode: self.decimal_mode,
            decimal_carry: self.decimal_carry,
            automatic_interrupt_mode: self.automatic_interrupt_mode,
            clock_sixths: self.clock_sixths,
            selected_x_group: self.selected_x_group,
            n_ready: self.n_ready,
            typewriter_power: self.typewriter_power,
            control_switches: self.control_switches,
            x_words: (0..4)
                .map(|slot| self.memory[self.selected_x_group * 4 + slot] & MASK_20)
                .collect(),
            halted: self.halted,
            memory: self.memory.clone(),
        }
    }

    pub fn set_control_switches(&mut self, value: i32) {
        self.control_switches = value & MASK_20;
    }

    pub fn set_clock_sixths(&mut self, value: i32) -> Result<(), String> {
        if !(0..=DATA_MASK).contains(&value) {
            return Err(format!(
                "GE-225 clock must fit its 19-bit C register, got {value}"
            ));
        }
        self.clock_sixths = value;
        Ok(())
    }

    pub fn advance_clock_sixths(&mut self, ticks: u64) {
        let day = CLOCK_DAY_SIXTHS as u64;
        let current = self.clock_sixths as u64;
        self.clock_sixths = if current < day {
            ((current + ticks % day) % day) as i32
        } else {
            let ticks_to_word_wrap = CLOCK_WORD_MODULUS - current;
            if ticks < ticks_to_word_wrap {
                (current + ticks) as i32
            } else {
                ((ticks - ticks_to_word_wrap) % day) as i32
            }
        };
    }

    pub fn clear_decimal_carry(&mut self) {
        self.decimal_carry = 0;
    }
    pub fn set_program_counter(&mut self, address: i32) -> Result<(), String> {
        self.set_pc(address)
    }
    pub fn queue_card_reader_record(&mut self, words: &[i32]) -> Result<(), String> {
        if words.len() > MAX_CARD_RECORD_WORDS {
            return Err(format!(
                "GE-225 card-reader record exceeds {MAX_CARD_RECORD_WORDS} words: {}",
                words.len()
            ));
        }
        if self.card_reader_queue.len() >= MAX_CARD_QUEUE_DEPTH {
            return Err(format!(
                "GE-225 card-reader queue is full at {MAX_CARD_QUEUE_DEPTH} records"
            ));
        }
        self.card_reader_queue
            .push_back(words.iter().map(|w| w & MASK_20).collect());
        Ok(())
    }
    pub fn get_typewriter_output(&self) -> String {
        self.typewriter_output.join("")
    }
    pub fn load_words(&mut self, words: &[i32], start_address: i32) -> Result<(), String> {
        let range = self.checked_range(start_address, words.len())?;
        for (destination, word) in range.zip(words) {
            self.memory[destination] = word & MASK_20;
        }
        Ok(())
    }
    pub fn read_word(&self, address: i32) -> Result<i32, String> {
        self.check_address(address)?;
        Ok(self.memory[address as usize])
    }
    pub fn write_word(&mut self, address: i32, value: i32) -> Result<(), String> {
        self.check_address(address)?;
        self.memory[address as usize] = value & MASK_20;
        Ok(())
    }

    pub fn disassemble_word(&self, word: i32) -> Result<String, String> {
        let decoded = self.decode_word(word)?;
        if decoded.fixed_word {
            let modifier = decoded.modifier.unwrap_or(0);
            let suffix = if modifier == 0 {
                String::new()
            } else {
                format!(",X{modifier}")
            };
            return Ok(if let Some(group) = decoded.sxg_group {
                format!("SXG {group}")
            } else if let Some(count) = decoded.count {
                format!("{} {count}{suffix}", decoded.mnemonic)
            } else {
                format!("{}{suffix}", decoded.mnemonic)
            });
        }
        let address = decoded
            .address
            .ok_or_else(|| "GE-225 decoder omitted a memory-reference address".to_string())?;
        let modifier = decoded
            .modifier
            .ok_or_else(|| "GE-225 decoder omitted a memory-reference modifier".to_string())?;
        Ok(format!(
            "{} 0x{:03X},X{}",
            decoded.mnemonic, address, modifier
        ))
    }

    pub fn step(&mut self) -> Result<Trace, String> {
        if self.halted {
            return Err("cannot step a halted GE-225 simulator".into());
        }
        let pc_before = self.pc;
        let instruction_word = self.read_word(pc_before)?;
        let decoded = self.decode_word(instruction_word)?;
        let mut execution_decoded = decoded.clone();
        let mut ir_word = instruction_word;
        if decoded.fixed_word {
            let modifier = decoded.modifier.unwrap_or(0);
            if modifier != 0 {
                if decoded.sxg_group.is_some() {
                    return Err("SXG cannot be automatically modified".into());
                }
                let (_, _, operand) = decode_instruction(instruction_word);
                let increment = self.get_x_word(modifier as usize)? & ADDR_MASK;
                let modified_operand = (operand + increment) & ADDR_MASK;
                ir_word = (instruction_word & !ADDR_MASK) | modified_operand;
                if let Some(count) = decoded.count {
                    let modified_count = count + increment;
                    if modified_count > 31 {
                        return Err(format!(
                            "modified GE-225 shift count exceeds 31: {count} + {increment}"
                        ));
                    }
                    execution_decoded.count = Some(modified_count);
                    execution_decoded.modifier = Some(0);
                } else {
                    let modified_word = (OP_RCD << 15) | modified_operand;
                    execution_decoded = self.decode_word(modified_word)?;
                    if !execution_decoded.fixed_word {
                        return Err(format!(
                            "automatic modification produced a non-fixed GE-225 instruction: {modified_word:07o}"
                        ));
                    }
                }
            }
            if matches!(execution_decoded.mnemonic, "SNA" | "NAQ" | "ANQ") && !self.n_ready {
                return Err(format!(
                    "{} requires the GE-225 N register to be ready",
                    execution_decoded.mnemonic
                ));
            }
        }
        let sequential_pc = pc_before
            .checked_add(1)
            .ok_or_else(|| "GE-225 P counter overflow".to_string())?;
        if sequential_pc >= self.memory_size && !matches!(decoded.mnemonic, "BRU" | "SPB") {
            return Err(format!(
                "GE-225 sequential P counter leaves installed memory: {sequential_pc}"
            ));
        }
        let mut effective_address = None;
        if !execution_decoded.fixed_word {
            let address = execution_decoded
                .address
                .ok_or_else(|| "GE-225 decoder omitted a memory-reference address".to_string())?;
            let modifier = execution_decoded
                .modifier
                .ok_or_else(|| "GE-225 decoder omitted a memory-reference modifier".to_string())?;
            if execution_decoded.mnemonic == "BRU" {
                effective_address = Some(if modifier == 0 {
                    self.direct_branch_target(sequential_pc, address)?
                } else {
                    self.resolve_effective_address(address, modifier)?
                });
            } else if !matches!(
                execution_decoded.mnemonic,
                "BXL" | "BXH" | "LDX" | "SPB" | "INX" | "STX" | "MOV"
            ) {
                effective_address = Some(self.resolve_effective_address(address, modifier)?);
            }
            if modifier != 0 {
                if let Some(address) = effective_address {
                    ir_word = (instruction_word & !ADDR_MASK) | (address & ADDR_MASK);
                }
            }
        }
        self.preflight_decimal(&execution_decoded, effective_address)?;
        self.ir = ir_word;
        self.pc = sequential_pc;
        let a_before = self.a;
        let q_before = self.q;
        if !execution_decoded.fixed_word {
            let address = execution_decoded
                .address
                .ok_or_else(|| "GE-225 decoder omitted a memory-reference address".to_string())?;
            let modifier = execution_decoded
                .modifier
                .ok_or_else(|| "GE-225 decoder omitted a memory-reference modifier".to_string())?;
            self.execute_memory_reference(
                execution_decoded.mnemonic,
                modifier,
                effective_address.unwrap_or(address),
                address,
                pc_before,
            )?;
        } else {
            self.execute_fixed(&execution_decoded)?;
        }
        self.check_address(self.pc)?;
        Ok(Trace {
            address: pc_before,
            instruction_word,
            mnemonic: self.disassemble_word(instruction_word)?,
            a_before,
            a_after: self.a,
            q_before,
            q_after: self.q,
            effective_address,
        })
    }

    pub fn run(&mut self, max_steps: usize) -> Result<Vec<Trace>, String> {
        let mut traces = vec![];
        for _ in 0..max_steps {
            if self.halted {
                break;
            }
            traces.push(self.step()?);
        }
        Ok(traces)
    }

    fn x_address(&self, slot: usize) -> Result<i32, String> {
        if slot >= 4 {
            return Err(format!("GE-225 X-word slot out of range: {slot}"));
        }
        let address = self
            .selected_x_group
            .checked_mul(4)
            .and_then(|base| base.checked_add(slot))
            .ok_or_else(|| "GE-225 X-word address overflow".to_string())?;
        i32::try_from(address).map_err(|_| "GE-225 X-word address overflow".to_string())
    }

    fn get_x_word(&self, slot: usize) -> Result<i32, String> {
        self.read_word(self.x_address(slot)?)
    }

    fn set_x_word(&mut self, slot: usize, value: i32) -> Result<(), String> {
        self.write_word(self.x_address(slot)?, value)
    }

    fn preflight_decimal(
        &self,
        decoded: &DecodedInstruction,
        effective_address: Option<i32>,
    ) -> Result<(), String> {
        if !self.decimal_mode {
            return Ok(());
        }
        match decoded.mnemonic {
            "ADD" | "SUB" => {
                let address = effective_address.ok_or_else(|| {
                    format!(
                        "GE-225 {} decoder omitted its effective address",
                        decoded.mnemonic
                    )
                })?;
                let operand = self.read_word(address)?;
                decimal_word_operation(
                    self.a,
                    operand,
                    decoded.mnemonic == "SUB",
                    self.decimal_carry,
                )?;
            }
            "DAD" | "DSU" => {
                let address = effective_address.ok_or_else(|| {
                    format!(
                        "GE-225 {} decoder omitted its effective address",
                        decoded.mnemonic
                    )
                })?;
                let first = self.read_word(address)?;
                let second = if (address & 1) != 0 {
                    first
                } else {
                    self.read_word(self.following_address(address)?)?
                };
                decimal_pair_operation(
                    self.a,
                    self.q,
                    first,
                    second,
                    decoded.mnemonic == "DSU",
                    self.decimal_carry,
                )?;
            }
            "ADO" | "SBO" => {
                decimal_word_operation(
                    self.a,
                    encode_decimal_word(1, false, false),
                    decoded.mnemonic == "SBO",
                    self.decimal_carry,
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn execute_memory_reference(
        &mut self,
        mnemonic: &str,
        modifier: i32,
        effective_or_raw_address: i32,
        raw_address: i32,
        pc_before: i32,
    ) -> Result<(), String> {
        let effective_address = effective_or_raw_address;
        match mnemonic {
            "LDA" => {
                self.m = self.read_word(effective_address)?;
                self.a = self.m;
            }
            "ADD" => {
                let operand = self.read_word(effective_address)?;
                if self.decimal_mode {
                    let (result, carry, overflow) =
                        decimal_word_operation(self.a, operand, false, self.decimal_carry)?;
                    self.m = operand;
                    self.a = result;
                    self.decimal_carry = carry;
                    self.overflow |= overflow;
                } else {
                    self.m = operand;
                    let total = to_signed20(self.a) + to_signed20(self.m);
                    self.a = from_signed20(total);
                    self.overflow |= !(-(1 << 19)..=(1 << 19) - 1).contains(&total);
                }
            }
            "SUB" => {
                let operand = self.read_word(effective_address)?;
                if self.decimal_mode {
                    let (result, carry, overflow) =
                        decimal_word_operation(self.a, operand, true, self.decimal_carry)?;
                    self.m = operand;
                    self.a = result;
                    self.decimal_carry = carry;
                    self.overflow |= overflow;
                } else {
                    self.m = operand;
                    let total = to_signed20(self.a) - to_signed20(self.m);
                    self.a = from_signed20(total);
                    self.overflow |= !(-(1 << 19)..=(1 << 19) - 1).contains(&total);
                }
            }
            "STA" => self.write_word(effective_address, self.a)?,
            "BXL" => {
                if (self.get_x_word(modifier as usize)? & ADDR_MASK) >= raw_address {
                    self.advance_pc(1)?;
                }
            }
            "BXH" => {
                if (self.get_x_word(modifier as usize)? & ADDR_MASK) < raw_address {
                    self.advance_pc(1)?;
                }
            }
            "LDX" => {
                let word = self.read_word(raw_address)?;
                self.set_x_word(modifier as usize, word)?;
            }
            "SPB" => {
                let target = self.direct_branch_target(pc_before, raw_address)?;
                let x_address = self.x_address(modifier as usize)?;
                self.check_address(x_address)?;
                self.memory[x_address as usize] = pc_before & MASK_20;
                self.pc = target;
            }
            "DLD" => {
                let first = self.read_word(effective_address)?;
                if (effective_address & 1) != 0 {
                    self.a = first;
                    self.q = first;
                } else {
                    let second = self.read_word(self.following_address(effective_address)?)?;
                    self.a = first;
                    self.q = second;
                }
            }
            "DAD" => {
                let first = self.read_word(effective_address)?;
                let second = if (effective_address & 1) != 0 {
                    first
                } else {
                    self.read_word(self.following_address(effective_address)?)?
                };
                if self.decimal_mode {
                    let (a, q, carry, overflow) = decimal_pair_operation(
                        self.a,
                        self.q,
                        first,
                        second,
                        false,
                        self.decimal_carry,
                    )?;
                    self.a = a;
                    self.q = q;
                    self.decimal_carry = carry;
                    self.overflow |= overflow;
                } else {
                    let total = to_signed_double(self.a, self.q) + to_signed_double(first, second);
                    (self.a, self.q) = split_signed_double(total);
                    self.overflow |= !(-(1_i64 << DOUBLE_DATA_BITS)
                        ..=((1_i64 << DOUBLE_DATA_BITS) - 1))
                        .contains(&total);
                }
            }
            "DSU" => {
                let first = self.read_word(effective_address)?;
                let second = if (effective_address & 1) != 0 {
                    first
                } else {
                    self.read_word(self.following_address(effective_address)?)?
                };
                if self.decimal_mode {
                    let (a, q, carry, overflow) = decimal_pair_operation(
                        self.a,
                        self.q,
                        first,
                        second,
                        true,
                        self.decimal_carry,
                    )?;
                    self.a = a;
                    self.q = q;
                    self.decimal_carry = carry;
                    self.overflow |= overflow;
                } else {
                    let total = to_signed_double(self.a, self.q) - to_signed_double(first, second);
                    (self.a, self.q) = split_signed_double(total);
                    self.overflow |= !(-(1_i64 << DOUBLE_DATA_BITS)
                        ..=((1_i64 << DOUBLE_DATA_BITS) - 1))
                        .contains(&total);
                }
            }
            "DST" => {
                if (effective_address & 1) != 0 {
                    self.write_word(effective_address, self.q)?;
                } else {
                    let second = self.following_address(effective_address)?;
                    self.write_word(effective_address, self.a)?;
                    self.write_word(second, self.q)?;
                }
            }
            "INX" => {
                let current = self.get_x_word(modifier as usize)?;
                let incremented = (current & !X_MASK) | ((current + raw_address) & X_MASK);
                self.set_x_word(modifier as usize, incremented)?;
            }
            "MPY" => {
                self.m = self.read_word(effective_address)?;
                let product = i64::from(to_signed20(self.q)) * i64::from(to_signed20(self.m))
                    + i64::from(to_signed20(self.a));
                (self.a, self.q) = split_signed_double(product);
                self.overflow = !(-(1_i64 << DOUBLE_DATA_BITS)..=((1_i64 << DOUBLE_DATA_BITS) - 1))
                    .contains(&product);
            }
            "DVD" => {
                self.m = self.read_word(effective_address)?;
                let divisor = i64::from(to_signed20(self.m));
                self.overflow = false;
                if divisor == 0 || i64::from(to_signed20(self.a)).abs() >= divisor.abs() {
                    self.overflow = true;
                    return Ok(());
                }
                let dividend = to_signed_double(self.a, self.q);
                let quotient_mag = dividend.abs() / divisor.abs();
                let remainder_mag = dividend.abs() % divisor.abs();
                let quotient = if (dividend < 0) ^ (divisor < 0) {
                    -quotient_mag
                } else {
                    quotient_mag
                };
                let remainder = if quotient < 0 {
                    -remainder_mag
                } else {
                    remainder_mag
                };
                if !(-(1_i64 << 19)..=((1_i64 << 19) - 1)).contains(&quotient) {
                    self.overflow = true;
                    return Ok(());
                }
                self.a = from_signed20(quotient as i32);
                self.q = from_signed20(remainder as i32);
            }
            "STX" => self.write_word(raw_address, self.get_x_word(modifier as usize)?)?,
            "EXT" => {
                self.m = self.read_word(effective_address)?;
                self.a &= (!self.m) & MASK_20;
            }
            "CAB" => {
                self.m = self.read_word(effective_address)?;
                match arith_compare(self.m, self.a) {
                    0 => self.advance_pc(1)?,
                    x if x < 0 => self.advance_pc(2)?,
                    _ => {}
                }
            }
            "DCB" => {
                let first = self.read_word(effective_address)?;
                let second = if (effective_address & 1) != 0 {
                    first
                } else {
                    self.read_word(self.following_address(effective_address)?)?
                };
                match arith_compare_double(first, second, self.a, self.q) {
                    0 => self.advance_pc(1)?,
                    x if x < 0 => self.advance_pc(2)?,
                    _ => {}
                }
            }
            "ORY" => {
                let word = self.read_word(effective_address)?;
                self.write_word(effective_address, word | self.a)?;
            }
            "MOV" => {
                let word_count = usize::try_from((-to_signed20(self.q)).max(0))
                    .map_err(|_| "GE-225 MOV word count overflow".to_string())?;
                let destination = self.a & X_MASK;
                let source_range = self.checked_range(raw_address, word_count)?;
                let destination_range = self.checked_range(destination, word_count)?;
                let moved: Vec<i32> = source_range.map(|address| self.memory[address]).collect();
                for (address, word) in destination_range.zip(moved) {
                    self.memory[address] = word;
                }
                self.set_x_word(0, self.pc)?;
                self.a = 0;
            }
            "RCD" => {
                if self.card_reader_queue.is_empty() {
                    return Err("RCD executed with no queued card-reader record".into());
                }
                let record_len = self.card_reader_queue.front().map_or(0, Vec::len);
                let range = self.checked_range(effective_address, record_len)?;
                let record = self
                    .card_reader_queue
                    .pop_front()
                    .ok_or_else(|| "RCD executed with no queued card-reader record".to_string())?;
                for (address, word) in range.zip(record) {
                    self.memory[address] = word;
                }
            }
            "BRU" => self.set_pc(effective_address)?,
            "STO" => {
                let existing = self.read_word(effective_address)?;
                self.write_word(
                    effective_address,
                    (existing & !ADDR_MASK) | (self.a & ADDR_MASK),
                )?;
            }
            _ => {
                return Err(format!(
                    "unimplemented GE-225 memory-reference instruction: {mnemonic}"
                ))
            }
        }
        Ok(())
    }

    fn execute_fixed(&mut self, decoded: &DecodedInstruction) -> Result<(), String> {
        let mnemonic = decoded.mnemonic;
        let count = decoded.count.unwrap_or(0);
        match mnemonic {
            "OFF" => {
                self.typewriter_power = false;
                self.n_ready = true;
            }
            "TYP" => {
                if !self.typewriter_power {
                    self.n_ready = false;
                    return Ok(());
                }
                let code = self.n & N_MASK;
                if code == 0o37 {
                    self.typewriter_output.push("\r".into());
                } else if code == 0o76 {
                    self.typewriter_output.push("\t".into());
                } else if code != 0o72 && code != 0o75 {
                    let ch = typewriter_char(code)
                        .ok_or_else(|| "invalid typewriter code".to_string())?;
                    self.typewriter_output.push(ch.into());
                }
                self.n_ready = true;
            }
            "TON" => self.typewriter_power = true,
            "RCS" => self.a |= self.control_switches,
            "HPT" => self.n_ready = false,
            "LDZ" => self.a = 0,
            "LDO" => self.a = 1,
            "LMO" => self.a = MASK_20,
            "CPL" => self.a = (!self.a) & MASK_20,
            "NEG" => {
                let before = to_signed20(self.a);
                self.a = from_signed20(-before);
                self.overflow |= before == -(1 << 19);
            }
            "CHS" => self.a ^= SIGN_BIT,
            "NOP" => {}
            "LAQ" => self.a = self.q,
            "LQA" => self.q = self.a,
            "XAQ" => std::mem::swap(&mut self.a, &mut self.q),
            "MAQ" => {
                self.q = self.a;
                self.a = 0;
            }
            "ADO" => {
                if self.decimal_mode {
                    let one = encode_decimal_word(1, false, false);
                    let (result, carry, overflow) =
                        decimal_word_operation(self.a, one, false, self.decimal_carry)?;
                    self.a = result;
                    self.decimal_carry = carry;
                    self.overflow |= overflow;
                } else {
                    let total = to_signed20(self.a) + 1;
                    self.a = from_signed20(total);
                    self.overflow |= !(-(1 << 19)..=(1 << 19) - 1).contains(&total);
                }
            }
            "SBO" => {
                if self.decimal_mode {
                    let one = encode_decimal_word(1, false, false);
                    let (result, carry, overflow) =
                        decimal_word_operation(self.a, one, true, self.decimal_carry)?;
                    self.a = result;
                    self.decimal_carry = carry;
                    self.overflow |= overflow;
                } else {
                    let total = to_signed20(self.a) - 1;
                    self.a = from_signed20(total);
                    self.overflow |= !(-(1 << 19)..=(1 << 19) - 1).contains(&total);
                }
            }
            "LAC" => self.a = self.clock_sixths & DATA_MASK,
            "LCA" => self.clock_sixths = self.a & DATA_MASK,
            "SET_DECMODE" => self.decimal_mode = true,
            "SET_BINMODE" => self.decimal_mode = false,
            "SXG" => {
                self.selected_x_group = decoded
                    .sxg_group
                    .ok_or_else(|| "GE-225 SXG decoder omitted its group".to_string())?;
            }
            "SET_PST" => self.automatic_interrupt_mode = true,
            "SET_PBK" => self.automatic_interrupt_mode = false,
            "BOD" | "BEV" | "BMI" | "BPL" | "BZE" | "BNZ" | "BOV" | "BNO" | "BPE" | "BPC"
            | "BNR" | "BNN" => self.execute_branch_test(mnemonic)?,
            _ => {
                if shift_base(mnemonic).is_some() {
                    self.execute_shift(mnemonic, count)?;
                } else {
                    return Err(format!(
                        "unimplemented GE-225 fixed instruction: {mnemonic}"
                    ));
                }
            }
        }
        Ok(())
    }

    fn execute_branch_test(&mut self, mnemonic: &str) -> Result<(), String> {
        let cond = match mnemonic {
            "BOD" => (self.a & 1) != 0,
            "BEV" => (self.a & 1) == 0,
            "BMI" => (self.a & SIGN_BIT) != 0,
            "BPL" => (self.a & SIGN_BIT) == 0,
            "BZE" => self.a == 0,
            "BNZ" => self.a != 0,
            "BOV" => self.overflow,
            "BNO" => !self.overflow,
            "BPE" => self.parity_error,
            "BPC" => !self.parity_error,
            "BNR" => self.n_ready,
            "BNN" => !self.n_ready,
            _ => false,
        };
        if !cond {
            self.advance_pc(1)?;
        }
        if matches!(mnemonic, "BOV" | "BNO") {
            self.overflow = false;
        }
        if matches!(mnemonic, "BPE" | "BPC") {
            self.parity_error = false;
        }
        Ok(())
    }

    fn execute_shift(&mut self, mnemonic: &str, count: i32) -> Result<(), String> {
        let a_sign = sign_of(self.a);
        let mut a_data = self.a & DATA_MASK;
        let q_sign = sign_of(self.q);
        let mut q_data = self.q & DATA_MASK;
        match mnemonic {
            "SRA" => self.a = from_signed20(to_signed20(self.a) >> count.min(19)),
            "SLA" if count != 0 => {
                self.overflow |= (a_data >> (19 - count).max(0)) != 0;
                self.a = with_sign((a_data << count) & DATA_MASK, a_sign);
            }
            "SCA" => {
                let rotation = count % 19;
                if rotation != 0 {
                    a_data = ((a_data >> rotation) | (a_data << (19 - rotation))) & DATA_MASK;
                }
                self.a = with_sign(a_data, a_sign);
            }
            "SAN" => {
                let fill = if a_sign == 1 { (1_i64 << count) - 1 } else { 0 };
                let mut combined =
                    (((a_data & DATA_MASK) as i64) << 6) | i64::from(self.n & N_MASK);
                combined = ((fill << 25) | combined) >> count;
                self.a = with_sign(((combined >> 6) as i32) & DATA_MASK, a_sign);
                self.n = (combined as i32) & N_MASK;
            }
            "SNA" => {
                let combined = (((self.n & N_MASK) << 19) | a_data) >> count;
                self.n = (combined >> 19) & N_MASK;
                self.a = with_sign(combined & DATA_MASK, a_sign);
            }
            "SRD" => {
                let combined = (i64::from(a_data) << 19) | i64::from(q_data);
                let signed = if a_sign == 1 {
                    combined | !DOUBLE_DATA_MASK
                } else {
                    combined
                };
                let shifted = (signed >> count) & DOUBLE_DATA_MASK;
                self.a = with_sign(((shifted >> 19) as i32) & DATA_MASK, a_sign);
                self.q = with_sign((shifted as i32) & DATA_MASK, a_sign);
            }
            "NAQ" => {
                let combined = ((((self.n & N_MASK) as i64) << 38)
                    | (((a_data & DATA_MASK) as i64) << 19)
                    | (q_data as i64))
                    >> count;
                self.n = ((combined >> 38) as i32) & N_MASK;
                self.a = with_sign(((combined >> 19) as i32) & DATA_MASK, a_sign);
                self.q = with_sign((combined as i32) & DATA_MASK, a_sign);
            }
            "SCD" => {
                let rotation = count % 38;
                let mut combined = (((a_data & DATA_MASK) as i64) << 19) | (q_data as i64);
                if rotation != 0 {
                    combined = ((combined >> rotation) | (combined << (38 - rotation)))
                        & ((1_i64 << 38) - 1);
                }
                self.a = with_sign(((combined >> 19) as i32) & DATA_MASK, a_sign);
                self.q = with_sign((combined as i32) & DATA_MASK, a_sign);
            }
            "ANQ" => {
                for _ in 0..count {
                    let bit = self.a & 1;
                    self.a = from_signed20(to_signed20(self.a) >> 1);
                    q_data = ((bit << 18) | ((self.q & DATA_MASK) >> 1)) & DATA_MASK;
                    self.q = with_sign(q_data, a_sign);
                    self.n = ((bit << 5) | (self.n >> 1)) & N_MASK;
                }
                self.q = with_sign(self.q, a_sign);
            }
            "SLD" => {
                let mut combined = (((a_data & DATA_MASK) as i64) << 19) | (q_data as i64);
                if count != 0 {
                    self.overflow |= (combined >> (38 - count).max(0)) != 0;
                }
                combined = (combined << count) & ((1_i64 << 38) - 1);
                self.a = with_sign(((combined >> 19) as i32) & DATA_MASK, q_sign);
                self.q = with_sign((combined as i32) & DATA_MASK, q_sign);
            }
            "NOR" => {
                let mut shifts = 0;
                let target_bit = if a_sign == 0 { 0 } else { 1 };
                while shifts < count {
                    let lead = (a_data >> 18) & 1;
                    if lead != target_bit {
                        break;
                    }
                    self.overflow |= lead == 1;
                    a_data = (a_data << 1) & DATA_MASK;
                    shifts += 1;
                }
                self.a = with_sign(a_data, a_sign);
                self.write_word(0, count - shifts)?;
            }
            "DNO" => {
                let mut shifts = 0;
                let target_bit = if a_sign == 0 { 0 } else { 1 };
                let mut combined = (((a_data & DATA_MASK) as i64) << 19) | (q_data as i64);
                while shifts < count {
                    let lead = (combined >> 37) & 1_i64;
                    if lead != target_bit {
                        break;
                    }
                    self.overflow |= lead == 1;
                    combined = (combined << 1) & ((1_i64 << 38) - 1);
                    shifts += 1;
                }
                self.a = with_sign(((combined >> 19) as i32) & DATA_MASK, q_sign);
                self.q = with_sign((combined as i32) & DATA_MASK, q_sign);
                self.write_word(0, count - shifts)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn decode_word(&self, word: i32) -> Result<DecodedInstruction, String> {
        let normalized = word & MASK_20;
        let (opcode, modifier, address) = decode_instruction(normalized);
        let canonical = normalized & !MODIFIER_MASK;
        if let Some(name) = fixed_name(canonical) {
            return Ok(DecodedInstruction {
                mnemonic: name,
                modifier: Some(modifier),
                address: None,
                count: None,
                sxg_group: None,
                fixed_word: true,
            });
        }
        if (canonical & !SXG_GROUP_MASK) == SXG_BASE {
            return Ok(DecodedInstruction {
                mnemonic: "SXG",
                modifier: Some(modifier),
                address: None,
                count: None,
                sxg_group: Some(((canonical & SXG_GROUP_MASK) >> SXG_GROUP_SHIFT) as usize),
                fixed_word: true,
            });
        }
        for name in [
            "SRA", "SNA", "SCA", "SAN", "SRD", "NAQ", "SCD", "ANQ", "SLA", "SLD", "NOR", "DNO",
        ] {
            if let Some(base) = shift_base(name) {
                if (canonical & !0o37) == base {
                    return Ok(DecodedInstruction {
                        mnemonic: name,
                        modifier: Some(modifier),
                        address: None,
                        count: Some(canonical & 0o37),
                        sxg_group: None,
                        fixed_word: true,
                    });
                }
            }
        }
        let mnemonic = base_opcode_name(opcode)
            .ok_or_else(|| format!("unknown GE-225 opcode field {opcode:o}"))?;
        Ok(DecodedInstruction {
            mnemonic,
            modifier: Some(modifier),
            address: Some(address),
            count: None,
            sxg_group: None,
            fixed_word: false,
        })
    }

    fn resolve_effective_address(&self, address: i32, modifier: i32) -> Result<i32, String> {
        let effective = if modifier == 0 {
            address
        } else {
            (address + self.get_x_word(modifier as usize)?) & X_MASK
        };
        self.check_address(effective)?;
        Ok(effective)
    }

    fn direct_branch_target(&self, instruction_address: i32, address: i32) -> Result<i32, String> {
        let target = (instruction_address & !ADDR_MASK) | address;
        self.check_address(target)?;
        Ok(target)
    }

    fn checked_range(
        &self,
        start_address: i32,
        word_count: usize,
    ) -> Result<std::ops::Range<usize>, String> {
        let start = usize::try_from(start_address)
            .map_err(|_| format!("address out of range: {start_address}"))?;
        let end = start.checked_add(word_count).ok_or_else(|| {
            format!("address range overflows: start={start_address}, words={word_count}")
        })?;
        if end > self.memory.len() {
            return Err(format!(
                "address range out of range: start={start_address}, words={word_count}, memory_words={}",
                self.memory.len()
            ));
        }
        Ok(start..end)
    }

    fn following_address(&self, address: i32) -> Result<i32, String> {
        let next = address
            .checked_add(1)
            .ok_or_else(|| format!("address overflow after {address}"))?;
        self.check_address(next)?;
        Ok(next)
    }

    fn set_pc(&mut self, address: i32) -> Result<(), String> {
        self.check_address(address)?;
        self.pc = address;
        Ok(())
    }

    fn advance_pc(&mut self, count: i32) -> Result<(), String> {
        let address = self
            .pc
            .checked_add(count)
            .ok_or_else(|| "GE-225 P counter overflow".to_string())?;
        self.set_pc(address)
    }

    fn check_address(&self, address: i32) -> Result<(), String> {
        if address < 0 || address >= self.memory_size {
            Err(format!("address out of range: {address}"))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ins(opcode: i32, address: i32, modifier: i32) -> i32 {
        encode_instruction(opcode, modifier, address).unwrap()
    }

    #[test]
    fn encode_decode_round_trip() {
        let word = ins(0o01, 0x1234 & 0x1fff, 0o2);
        assert_eq!(decode_instruction(word), (0o01, 0o2, 0x1234 & 0x1fff));
        assert_eq!(
            unpack_words(&pack_words(&[word, assemble_fixed("NOP").unwrap()])).unwrap(),
            vec![word, assemble_fixed("NOP").unwrap()]
        );
    }

    #[test]
    fn lda_add_sta_program() {
        let mut sim = Simulator::new(4096).unwrap();
        sim.load_words(
            &[
                ins(0o00, 10, 0),
                ins(0o01, 11, 0),
                ins(0o03, 12, 0),
                assemble_fixed("NOP").unwrap(),
                0,
                0,
                0,
                0,
                0,
                0,
                1,
                2,
                0,
            ],
            0,
        )
        .unwrap();
        sim.run(4).unwrap();
        let state = sim.get_state();
        assert_eq!(state.a, 3);
        assert_eq!(state.memory[12], 3);
    }

    #[test]
    fn spb_stores_instruction_address() {
        let mut sim = Simulator::new(4096).unwrap();
        sim.load_words(
            &[
                ins(0o07, 4, 2),
                assemble_fixed("NOP").unwrap(),
                assemble_fixed("NOP").unwrap(),
                assemble_fixed("NOP").unwrap(),
                ins(0o00, 10, 0),
                assemble_fixed("NOP").unwrap(),
                0,
                0,
                0,
                0,
                0x12345,
            ],
            0,
        )
        .unwrap();
        sim.run(3).unwrap();
        let state = sim.get_state();
        assert_eq!(state.x_words[2], 0);
        assert_eq!(state.a, 0x12345);
    }

    #[test]
    fn odd_address_double_ops() {
        let mut sim = Simulator::new(4096).unwrap();
        sim.write_word(11, 0x13579).unwrap();
        sim.load_words(
            &[
                ins(0o10, 11, 0),
                ins(0o13, 13, 0),
                assemble_fixed("NOP").unwrap(),
            ],
            0,
        )
        .unwrap();
        sim.run(3).unwrap();
        let state = sim.get_state();
        assert_eq!(state.a, 0x13579);
        assert_eq!(state.q, 0x13579);
        assert_eq!(state.memory[13], 0x13579);
    }

    #[test]
    fn mov_moves_blocks() {
        let mut sim = Simulator::new(4096).unwrap();
        sim.write_word(20, 0x11111).unwrap();
        sim.write_word(21, 0x22222).unwrap();
        sim.write_word(30, 40).unwrap();
        sim.write_word(31, (1 << 20) - 2).unwrap();
        sim.load_words(
            &[
                ins(0o00, 30, 0),
                assemble_fixed("LQA").unwrap(),
                ins(0o00, 31, 0),
                assemble_fixed("XAQ").unwrap(),
                ins(0o24, 20, 0),
                assemble_fixed("NOP").unwrap(),
            ],
            0,
        )
        .unwrap();
        sim.run(6).unwrap();
        let state = sim.get_state();
        assert_eq!(state.a, 0);
        assert_eq!(state.memory[40], 0x11111);
        assert_eq!(state.memory[41], 0x22222);
    }

    #[test]
    fn console_typewriter_path() {
        let mut sim = Simulator::new(4096).unwrap();
        sim.set_control_switches(0o1633);
        sim.load_words(
            &[
                assemble_fixed("RCS").unwrap(),
                assemble_fixed("TON").unwrap(),
                assemble_shift("SAN", 6).unwrap(),
                assemble_fixed("TYP").unwrap(),
                assemble_fixed("NOP").unwrap(),
            ],
            0,
        )
        .unwrap();
        sim.run(5).unwrap();
        assert_eq!(sim.get_typewriter_output(), "-");
        assert!(sim.get_state().typewriter_power);
    }

    #[test]
    fn rcd_loads_queued_record() {
        let mut sim = Simulator::new(4096).unwrap();
        sim.queue_card_reader_record(&[0x11111, 0x22222]).unwrap();
        sim.load_words(&[ins(0o25, 10, 0), assemble_fixed("NOP").unwrap()], 0)
            .unwrap();
        sim.run(2).unwrap();
        let state = sim.get_state();
        assert_eq!(state.memory[10], 0x11111);
        assert_eq!(state.memory[11], 0x22222);
    }
}
