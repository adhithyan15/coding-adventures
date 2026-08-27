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
const MAX_CARD_PUNCH_DEPTH: usize = 64;
const MAX_CHARACTER_QUEUE_DEPTH: usize = 65_536;
const MAX_CONTROLLER_COMMANDS: usize = 64;
const CONTROLLER_COUNT: usize = 8;
const CONTROLLER_READY_CONDITION: u8 = 0o20;
const CONTROLLER_CONDITION_MIN: u8 = 0o20;
const CONTROLLER_CONDITION_MAX: u8 = 0o35;
const CONTROLLER_PLUG_MASK: i32 = 0o700;
const CONTROLLER_CONDITION_MASK: i32 = 0o77;
const CONTROLLER_SELECT_BASE: i32 = 0o2500020;
const CONTROLLER_STATUS_SET_BASE: i32 = 0o2514000;
const CONTROLLER_STATUS_CLEAR_BASE: i32 = 0o2516000;
const API_X_GROUP: usize = 32;
const API_SAVED_PC_ADDRESS: i32 = 0o201;
const API_VECTOR_ADDRESS: i32 = 0o204;
const CARD_ADDRESS_ALIGNMENT: i32 = 128;
const CARD_ADDRESS_LIMIT: i32 = 2_048;
const CARD_MODE_MASK: i32 = 0x0f;
const CARD_RESERVED_MASK: i32 = 0x70;
const CARD_DECIMAL_WORDS: usize = 27;
const CARD_BINARY_WORDS: usize = 40;
const CARD_FULL_WORDS: usize = 80;
const CARD_DECIMAL_SYNC: i32 = 0o2606077;
const CARD_BINARY_SYNC: i32 = 0o2001777;
const CARD_FULL_SYNC: i32 = 0o2007777;
const AAU_WORD_MASK: u64 = (1_u64 << 40) - 1;
const AAU_FIXED_DATA_BITS: u32 = 38;
const AAU_FIXED_WORD_BITS: u32 = 39;
const AAU_FIXED_DATA_MASK: i64 = (1_i64 << AAU_FIXED_DATA_BITS) - 1;
const AAU_FIXED_WORD_MASK: i64 = (1_i64 << AAU_FIXED_WORD_BITS) - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AauMode {
    FixedPoint,
    NormalizedFloatingPoint,
    UnnormalizedFloatingPoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AauState {
    pub mode: Option<AauMode>,
    pub ready: bool,
    pub ax: u64,
    pub bx: u64,
    pub qx: u64,
    pub ix: u64,
    pub overflow: bool,
    pub underflow: bool,
    pub overflow_hold: bool,
    pub underflow_hold: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardFormat {
    Decimal,
    Binary10,
    Full12,
    MixedDecimal,
    MixedBinary,
}

impl CardFormat {
    pub const fn word_count(self) -> usize {
        match self {
            Self::Decimal => CARD_DECIMAL_WORDS,
            Self::Binary10 => CARD_BINARY_WORDS,
            Self::Full12 | Self::MixedDecimal | Self::MixedBinary => CARD_FULL_WORDS,
        }
    }
}

fn card_mode_code(mnemonic: &str) -> Option<i32> {
    Some(match mnemonic {
        "RCD" => 0o00,
        "RCB" => 0o01,
        "WCD" => 0o02,
        "WCB" => 0o03,
        "RCF" => 0o10,
        "RCM" => 0o12,
        "WCF" => 0o17,
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CardStatus {
    pub invalid_character: bool,
    pub output_stacker_full: bool,
    pub reader_malfunction: bool,
    pub end_of_file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardRecord {
    pub format: CardFormat,
    pub words: Vec<i32>,
    pub status: CardStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NRegisterDevice {
    Off,
    Typewriter,
    PaperTapeReader,
    PaperTapePunch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaperTapeFrame {
    pub data: i32,
    pub parity_error: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControllerStatus {
    pub online: bool,
    pub ready: bool,
    pub error: bool,
    pub conditions: u64,
    pub error_conditions: u64,
    pub api_enabled: bool,
}

impl Default for ControllerStatus {
    fn default() -> Self {
        Self {
            online: true,
            ready: true,
            error: false,
            conditions: 1_u64 << CONTROLLER_READY_CONDITION,
            error_conditions: 0,
            api_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControllerCommand {
    pub plug: u8,
    pub select_word: i32,
    pub command_word: i32,
    pub address_word: i32,
}

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
const OP_FLD: i32 = 0o30;
const OP_FAD: i32 = 0o31;
const OP_FSU: i32 = 0o32;
const OP_FST: i32 = 0o33;
const OP_FMP: i32 = 0o35;
const OP_FDV: i32 = 0o36;

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
    pub priority_mode: bool,
    pub priority_return_armed: bool,
    pub pending_controller_interrupts: u8,
    pub card_reader_api_enabled: bool,
    pub card_punch_api_enabled: bool,
    pub card_reader_interrupt_pending: bool,
    pub card_punch_interrupt_pending: bool,
    pub controller_selector_busy: bool,
    pub controller_selector_alarm: bool,
    pub selected_controller: Option<u8>,
    pub controllers: Vec<ControllerStatus>,
    pub clock_sixths: i32,
    pub selected_x_group: usize,
    pub n_ready: bool,
    pub typewriter_power: bool,
    pub n_device: NRegisterDevice,
    pub paper_tape_reader_running: bool,
    pub typewriter_keyboard_enabled: bool,
    pub n_overrun: bool,
    pub stop_on_parity_alarm: bool,
    pub card_reader_ready: bool,
    pub card_punch_ready: bool,
    pub card_reader_alarm: bool,
    pub card_punch_alarm: bool,
    pub priority_alarm: bool,
    pub control_switches: i32,
    pub x_words: Vec<i32>,
    pub halted: bool,
    pub memory: Vec<i32>,
    pub aau: AauState,
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
    card_format: Option<CardFormat>,
    controller_plug: Option<usize>,
    controller_condition: Option<u8>,
    controller_branch_when_set: bool,
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
        OP_FLD => "FLD",
        OP_FAD => "FAD",
        OP_FSU => "FSU",
        OP_FST => "FST",
        OP_FMP => "FMP",
        OP_FDV => "FDV",
        _ => return None,
    })
}

fn aau_general_word(mnemonic: &str) -> Option<i32> {
    Some(match mnemonic {
        "SET_FIXPOINT" => 0o3500010,
        "SET_NFLPOINT" => 0o3100010,
        "SET_UFLPOINT" => 0o3200010,
        "LAQ" => 0o3600002,
        "LQA" => 0o3200002,
        "MAQ" => 0o3100002,
        "XAQ" => 0o3500002,
        "ROV" => 0o3100004,
        "RUN" => 0o3200004,
        "RIN" => 0o3500004,
        "NOX" => 0o3100005,
        _ => return None,
    })
}

fn aau_general_name(word: i32) -> Option<&'static str> {
    Some(match word {
        0o3500010 => "AAU_SET_FIXPOINT",
        0o3100010 => "AAU_SET_NFLPOINT",
        0o3200010 => "AAU_SET_UFLPOINT",
        0o3600002 => "AAU_LAQ",
        0o3200002 => "AAU_LQA",
        0o3100002 => "AAU_MAQ",
        0o3500002 => "AAU_XAQ",
        0o3100004 => "AAU_ROV",
        0o3200004 => "AAU_RUN",
        0o3500004 => "AAU_RIN",
        0o3100005 => "AAU_NOX",
        _ => return None,
    })
}

fn aau_branch_word(mnemonic: &str) -> Option<i32> {
    Some(match mnemonic {
        "BAR" => 0o2514720,
        "BAN" => 0o2516720,
        "BMI" => 0o2514721,
        "BPL" => 0o2516721,
        "BZE" => 0o2514722,
        "BNZ" => 0o2516722,
        "BOV" => 0o2514723,
        "BNO" => 0o2516723,
        "BUF" => 0o2514724,
        "BNU" => 0o2516724,
        "BOO" => 0o2514725,
        "BON" => 0o2516725,
        "BUO" => 0o2514726,
        "BUN" => 0o2516726,
        "BER" => 0o2514727,
        "BNE" => 0o2516727,
        _ => return None,
    })
}

fn aau_branch_name(word: i32) -> Option<&'static str> {
    Some(match word {
        0o2514720 => "AAU_BAR",
        0o2516720 => "AAU_BAN",
        0o2514721 => "AAU_BMI",
        0o2516721 => "AAU_BPL",
        0o2514722 => "AAU_BZE",
        0o2516722 => "AAU_BNZ",
        0o2514723 => "AAU_BOV",
        0o2516723 => "AAU_BNO",
        0o2514724 => "AAU_BUF",
        0o2516724 => "AAU_BNU",
        0o2514725 => "AAU_BOO",
        0o2516725 => "AAU_BON",
        0o2514726 => "AAU_BUO",
        0o2516726 => "AAU_BUN",
        0o2514727 => "AAU_BER",
        0o2516727 => "AAU_BNE",
        _ => return None,
    })
}

fn fixed_word(mnemonic: &str) -> Option<i32> {
    Some(match mnemonic {
        "HCR" => 0o2500004,
        "OFF" => 0o2500005,
        "TYP" | "RPT" | "WPT" | "NIO" => 0o2500006,
        "TON" => 0o2500007,
        "RCS" => 0o2500011,
        "RON" => 0o2500014,
        "PON" => 0o2500015,
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
        "BCR" => 0o2514006,
        "BCN" => 0o2516006,
        "BPR" => 0o2514007,
        "BPN" => 0o2516007,
        _ => return None,
    })
}

fn fixed_name(word: i32) -> Option<&'static str> {
    Some(match word {
        0o2500004 => "HCR",
        0o2500005 => "OFF",
        0o2500007 => "TON",
        0o2500011 => "RCS",
        0o2500014 => "RON",
        0o2500015 => "PON",
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
        0o2514006 => "BCR",
        0o2516006 => "BCN",
        0o2514007 => "BPR",
        0o2516007 => "BPN",
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

fn aau_fixed_value(raw: u64) -> i64 {
    let (high, low) = unpack_aau_words(raw);
    let bits = (i64::from(high) << 19) | i64::from(low & DATA_MASK);
    if (high & SIGN_BIT) != 0 {
        bits - (1_i64 << AAU_FIXED_WORD_BITS)
    } else {
        bits
    }
}

fn aau_fixed_raw(value: i64) -> u64 {
    let bits = value & AAU_FIXED_WORD_MASK;
    let high = ((bits >> 19) & i64::from(MASK_20)) as i32;
    let low = with_sign((bits & i64::from(DATA_MASK)) as i32, sign_of(high));
    pack_aau_words(high, low)
}

fn aau_fixed_pair_value(ax: u64, qx: u64) -> i128 {
    let ax_bits = aau_fixed_value(ax) as i128 & ((1_i128 << AAU_FIXED_WORD_BITS) - 1);
    let qx_data = aau_fixed_value(qx) as i128 & ((1_i128 << AAU_FIXED_DATA_BITS) - 1);
    let bits = (ax_bits << AAU_FIXED_DATA_BITS) | qx_data;
    if (bits & (1_i128 << (AAU_FIXED_WORD_BITS + AAU_FIXED_DATA_BITS - 1))) != 0 {
        bits - (1_i128 << (AAU_FIXED_WORD_BITS + AAU_FIXED_DATA_BITS))
    } else {
        bits
    }
}

fn split_aau_fixed_pair(value: i128) -> (u64, u64) {
    const PAIR_BITS: u32 = AAU_FIXED_WORD_BITS + AAU_FIXED_DATA_BITS;
    let bits = value & ((1_i128 << PAIR_BITS) - 1);
    let ax_bits = (bits >> AAU_FIXED_DATA_BITS) as i64;
    let qx_data = (bits & ((1_i128 << AAU_FIXED_DATA_BITS) - 1)) as i64;
    let ax = aau_fixed_raw(ax_bits);
    let qx = aau_fixed_raw(if (ax_bits & (1_i64 << (AAU_FIXED_WORD_BITS - 1))) != 0 {
        qx_data | !AAU_FIXED_DATA_MASK
    } else {
        qx_data
    });
    (ax, qx)
}

fn sign_extend_i64(value: i64, bits: u32) -> i64 {
    let sign = 1_i64 << (bits - 1);
    let mask = (1_i64 << bits) - 1;
    let narrowed = value & mask;
    if narrowed & sign != 0 {
        narrowed - (1_i64 << bits)
    } else {
        narrowed
    }
}

fn aau_float_parts(raw: u64) -> (i32, i64) {
    let exponent_bits = ((raw >> 31) & 0x1ff) as i32;
    let exponent_magnitude = exponent_bits & 0xff;
    let exponent = if exponent_bits & 0x100 == 0 {
        exponent_magnitude
    } else if exponent_magnitude == 0 {
        -256
    } else {
        -exponent_magnitude
    };
    let sign = ((raw >> 19) & 1) as i64;
    let upper = ((raw >> 20) & 0x7ff) as i64;
    let lower = (raw & DATA_MASK as u64) as i64;
    let mantissa = sign_extend_i64((sign << 30) | (upper << 19) | lower, 31);
    (exponent, mantissa)
}

fn aau_float_raw(exponent: i32, mantissa: i64) -> u64 {
    let exponent_bits = if (0..=255).contains(&exponent) {
        exponent
    } else if (-256..=-1).contains(&exponent) {
        0x100 | (-exponent & 0xff)
    } else if exponent < -256 {
        (-exponent) & 0xff
    } else {
        0x100 | ((exponent - 256) & 0xff)
    };
    let mantissa_bits = mantissa & ((1_i64 << 31) - 1);
    let sign = (mantissa_bits >> 30) & 1;
    let upper = (mantissa_bits >> 19) & 0x7ff;
    let lower = mantissa_bits & i64::from(DATA_MASK);
    (((exponent_bits as u64) << 31) | ((upper as u64) << 20) | ((sign as u64) << 19) | lower as u64)
        & AAU_WORD_MASK
}

fn aau_float_pair_parts(ax: u64, qx: u64) -> (i32, i128) {
    let (exponent, ax_mantissa) = aau_float_parts(ax);
    let (_, qx_mantissa) = aau_float_parts(qx);
    let ax_bits = i128::from(ax_mantissa) & ((1_i128 << 31) - 1);
    let qx_data = i128::from(qx_mantissa) & ((1_i128 << 30) - 1);
    let bits = (ax_bits << 30) | qx_data;
    let mantissa = if bits & (1_i128 << 60) != 0 {
        bits - (1_i128 << 61)
    } else {
        bits
    };
    (exponent, mantissa)
}

fn aau_float_pair_raw(exponent: i32, mantissa: i128) -> (u64, u64) {
    let bits = mantissa & ((1_i128 << 61) - 1);
    let ax_bits = (bits >> 30) as i64;
    let qx_data = (bits & ((1_i128 << 30) - 1)) as i64;
    let ax = aau_float_raw(exponent, sign_extend_i64(ax_bits, 31));
    let qx_sign = if bits & (1_i128 << 60) != 0 {
        1_i64 << 30
    } else {
        0
    };
    let qx = aau_float_raw(exponent - 30, qx_sign | qx_data);
    (ax, qx)
}

fn arithmetic_shift_right(value: i128, count: u32) -> i128 {
    if count >= 127 {
        if value < 0 {
            -1
        } else {
            0
        }
    } else {
        value >> count
    }
}

fn align_aau_float(value: i128, exponent: i32, target_exponent: i32) -> i128 {
    arithmetic_shift_right(value, (target_exponent - exponent).max(0) as u32)
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

pub fn assemble_card_io(mnemonic: &str, base: i32, modifier: i32) -> Result<i32, String> {
    if !(0..CARD_ADDRESS_LIMIT).contains(&base) || base % CARD_ADDRESS_ALIGNMENT != 0 {
        return Err(format!(
            "GE-225 card I/O base must be a multiple of {CARD_ADDRESS_ALIGNMENT} below {CARD_ADDRESS_LIMIT}, got {base}"
        ));
    }
    let mode = match mnemonic {
        "RCD" => 0o00,
        "RCB" => 0o01,
        "WCD" => 0o02,
        "WCB" => 0o03,
        "RCF" => 0o10,
        "RCM" => 0o12,
        "WCF" => 0o17,
        _ => return Err(format!("unknown GE-225 card I/O instruction: {mnemonic}")),
    };
    encode_instruction(OP_RCD, modifier, base | mode)
}

pub fn assemble_controller_select(plug: i32, modifier: i32) -> Result<i32, String> {
    if !(0..CONTROLLER_COUNT as i32).contains(&plug) {
        return Err(format!("GE-225 controller plug out of range: {plug}"));
    }
    if !(0..=3).contains(&modifier) {
        return Err(format!("modifier out of range: {modifier}"));
    }
    Ok(CONTROLLER_SELECT_BASE | (plug << 6) | (modifier << MODIFIER_SHIFT))
}

pub fn assemble_controller_status(
    plug: i32,
    condition: i32,
    branch_when_set: bool,
) -> Result<i32, String> {
    if !(0..CONTROLLER_COUNT as i32).contains(&plug) {
        return Err(format!("GE-225 controller plug out of range: {plug}"));
    }
    if !(i32::from(CONTROLLER_CONDITION_MIN)..=i32::from(CONTROLLER_CONDITION_MAX))
        .contains(&condition)
    {
        return Err(format!(
            "GE-225 controller condition must be {:02o} through {:02o}, got {condition:o}",
            CONTROLLER_CONDITION_MIN, CONTROLLER_CONDITION_MAX
        ));
    }
    let base = if branch_when_set {
        CONTROLLER_STATUS_SET_BASE
    } else {
        CONTROLLER_STATUS_CLEAR_BASE
    };
    Ok(base | (plug << 6) | condition)
}

pub fn assemble_aau_general(mnemonic: &str) -> Result<i32, String> {
    aau_general_word(mnemonic)
        .ok_or_else(|| format!("unknown GE-225 AAU general instruction: {mnemonic}"))
}

pub fn assemble_aau_memory(mnemonic: &str, address: i32, modifier: i32) -> Result<i32, String> {
    let opcode = match mnemonic {
        "FLD" => OP_FLD,
        "FAD" => OP_FAD,
        "FSU" => OP_FSU,
        "FST" => OP_FST,
        "FMP" => OP_FMP,
        "FDV" => OP_FDV,
        _ => return Err(format!("unknown GE-225 AAU memory instruction: {mnemonic}")),
    };
    encode_instruction(opcode, modifier, address)
}

pub fn assemble_aau_branch(mnemonic: &str) -> Result<i32, String> {
    aau_branch_word(mnemonic)
        .ok_or_else(|| format!("unknown GE-225 AAU branch instruction: {mnemonic}"))
}

pub fn pack_aau_words(first: i32, second: i32) -> u64 {
    ((first as u64 & MASK_20 as u64) << 20) | (second as u64 & MASK_20 as u64)
}

pub fn unpack_aau_words(value: u64) -> (i32, i32) {
    (
        ((value >> 20) & MASK_20 as u64) as i32,
        (value & MASK_20 as u64) as i32,
    )
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
    controllers: [ControllerStatus; CONTROLLER_COUNT],
    controller_commands: Vec<ControllerCommand>,
    controller_selector_busy: bool,
    controller_selector_alarm: bool,
    selected_controller: Option<u8>,
    pending_controller_interrupts: u8,
    card_reader_api_enabled: bool,
    card_punch_api_enabled: bool,
    card_reader_interrupt_pending: bool,
    card_punch_interrupt_pending: bool,
    priority_mode: bool,
    priority_return_armed: bool,
    api_branch_inhibit: bool,
    interrupted_x_group: usize,
    card_reader_queue: VecDeque<CardRecord>,
    card_punch_output: Vec<CardRecord>,
    card_reader_continuous: Option<CardFormat>,
    card_reader_base: i32,
    card_reader_slot: usize,
    card_reader_online: bool,
    card_punch_online: bool,
    card_reader_fault: bool,
    card_punch_fault: bool,
    card_reader_alarm: bool,
    card_punch_alarm: bool,
    priority_alarm: bool,
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
    n_device: NRegisterDevice,
    paper_tape_reader_running: bool,
    paper_tape_input: VecDeque<PaperTapeFrame>,
    paper_tape_output: Vec<i32>,
    typewriter_keyboard_enabled: bool,
    typewriter_input: VecDeque<i32>,
    n_overrun: bool,
    stop_on_parity_alarm: bool,
    control_switches: i32,
    aau_mode: Option<AauMode>,
    aau_ready: bool,
    aau_ax: u64,
    aau_bx: u64,
    aau_qx: u64,
    aau_ix: u64,
    aau_overflow: bool,
    aau_underflow: bool,
    aau_overflow_hold: bool,
    aau_underflow_hold: bool,
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
            controllers: [ControllerStatus::default(); CONTROLLER_COUNT],
            controller_commands: vec![],
            controller_selector_busy: false,
            controller_selector_alarm: false,
            selected_controller: None,
            pending_controller_interrupts: 0,
            card_reader_api_enabled: false,
            card_punch_api_enabled: false,
            card_reader_interrupt_pending: false,
            card_punch_interrupt_pending: false,
            priority_mode: false,
            priority_return_armed: false,
            api_branch_inhibit: false,
            interrupted_x_group: 0,
            card_reader_queue: VecDeque::new(),
            card_punch_output: vec![],
            card_reader_continuous: None,
            card_reader_base: 0,
            card_reader_slot: 0,
            card_reader_online: true,
            card_punch_online: true,
            card_reader_fault: false,
            card_punch_fault: false,
            card_reader_alarm: false,
            card_punch_alarm: false,
            priority_alarm: false,
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
            n_device: NRegisterDevice::Off,
            paper_tape_reader_running: false,
            paper_tape_input: VecDeque::new(),
            paper_tape_output: vec![],
            typewriter_keyboard_enabled: false,
            typewriter_input: VecDeque::new(),
            n_overrun: false,
            stop_on_parity_alarm: false,
            control_switches: 0,
            aau_mode: None,
            aau_ready: true,
            aau_ax: 0,
            aau_bx: 0,
            aau_qx: 0,
            aau_ix: 0,
            aau_overflow: false,
            aau_underflow: false,
            aau_overflow_hold: false,
            aau_underflow_hold: false,
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
        self.controllers = [ControllerStatus::default(); CONTROLLER_COUNT];
        self.controller_commands.clear();
        self.controller_selector_busy = false;
        self.controller_selector_alarm = false;
        self.selected_controller = None;
        self.pending_controller_interrupts = 0;
        self.card_reader_api_enabled = false;
        self.card_punch_api_enabled = false;
        self.card_reader_interrupt_pending = false;
        self.card_punch_interrupt_pending = false;
        self.priority_mode = false;
        self.priority_return_armed = false;
        self.api_branch_inhibit = false;
        self.interrupted_x_group = 0;
        self.clock_sixths = 0;
        self.selected_x_group = 0;
        self.n_ready = true;
        self.typewriter_power = false;
        self.typewriter_output.clear();
        self.n_device = NRegisterDevice::Off;
        self.paper_tape_reader_running = false;
        self.paper_tape_input.clear();
        self.paper_tape_output.clear();
        self.typewriter_keyboard_enabled = false;
        self.typewriter_input.clear();
        self.n_overrun = false;
        self.stop_on_parity_alarm = false;
        self.card_reader_queue.clear();
        self.card_punch_output.clear();
        self.card_reader_continuous = None;
        self.card_reader_base = 0;
        self.card_reader_slot = 0;
        self.card_reader_online = true;
        self.card_punch_online = true;
        self.card_reader_fault = false;
        self.card_punch_fault = false;
        self.card_reader_alarm = false;
        self.card_punch_alarm = false;
        self.priority_alarm = false;
        self.control_switches = 0;
        self.aau_mode = None;
        self.aau_ready = true;
        self.aau_ax = 0;
        self.aau_bx = 0;
        self.aau_qx = 0;
        self.aau_ix = 0;
        self.aau_overflow = false;
        self.aau_underflow = false;
        self.aau_overflow_hold = false;
        self.aau_underflow_hold = false;
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
            priority_mode: self.priority_mode,
            priority_return_armed: self.priority_return_armed,
            pending_controller_interrupts: self.pending_controller_interrupts,
            card_reader_api_enabled: self.card_reader_api_enabled,
            card_punch_api_enabled: self.card_punch_api_enabled,
            card_reader_interrupt_pending: self.card_reader_interrupt_pending,
            card_punch_interrupt_pending: self.card_punch_interrupt_pending,
            controller_selector_busy: self.controller_selector_busy,
            controller_selector_alarm: self.controller_selector_alarm,
            selected_controller: self.selected_controller,
            controllers: self.controllers.to_vec(),
            clock_sixths: self.clock_sixths,
            selected_x_group: self.selected_x_group,
            n_ready: self.n_ready,
            typewriter_power: self.typewriter_power,
            n_device: self.n_device,
            paper_tape_reader_running: self.paper_tape_reader_running,
            typewriter_keyboard_enabled: self.typewriter_keyboard_enabled,
            n_overrun: self.n_overrun,
            stop_on_parity_alarm: self.stop_on_parity_alarm,
            card_reader_ready: self.card_reader_ready(),
            card_punch_ready: self.card_punch_ready(),
            card_reader_alarm: self.card_reader_alarm,
            card_punch_alarm: self.card_punch_alarm,
            priority_alarm: self.priority_alarm,
            control_switches: self.control_switches,
            x_words: (0..4)
                .map(|slot| self.memory[self.selected_x_group * 4 + slot] & MASK_20)
                .collect(),
            halted: self.halted,
            memory: self.memory.clone(),
            aau: AauState {
                mode: self.aau_mode,
                ready: self.aau_ready,
                ax: self.aau_ax,
                bx: self.aau_bx,
                qx: self.aau_qx,
                ix: self.aau_ix,
                overflow: self.aau_overflow,
                underflow: self.aau_underflow,
                overflow_hold: self.aau_overflow_hold,
                underflow_hold: self.aau_underflow_hold,
            },
        }
    }

    pub fn set_control_switches(&mut self, value: i32) {
        self.control_switches = value & MASK_20;
    }

    pub fn set_aau_ready(&mut self, ready: bool) {
        self.aau_ready = ready;
    }

    pub fn clear_aau_alerts(&mut self) {
        self.aau_overflow = false;
        self.aau_underflow = false;
        self.aau_overflow_hold = false;
        self.aau_underflow_hold = false;
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

    pub fn controller_commands(&self) -> &[ControllerCommand] {
        &self.controller_commands
    }

    pub fn take_controller_commands(&mut self) -> Vec<ControllerCommand> {
        std::mem::take(&mut self.controller_commands)
    }

    pub fn highest_priority_pending_controller(&self) -> Option<usize> {
        (0..CONTROLLER_COUNT)
            .find(|plug| (self.pending_controller_interrupts & (1_u8 << plug)) != 0)
    }

    pub fn set_controller_online(&mut self, plug: usize, online: bool) -> Result<(), String> {
        let controller = self.controller_mut(plug)?;
        controller.online = online;
        if !online {
            Self::set_controller_ready_value(controller, false);
        }
        Ok(())
    }

    pub fn set_controller_api_enabled(&mut self, plug: usize, enabled: bool) -> Result<(), String> {
        self.controller_mut(plug)?.api_enabled = enabled;
        Ok(())
    }

    pub fn set_controller_condition(
        &mut self,
        plug: usize,
        condition: u8,
        asserted: bool,
    ) -> Result<(), String> {
        Self::check_controller_condition(condition)?;
        if condition == CONTROLLER_READY_CONDITION {
            return self.set_controller_ready(plug, asserted);
        }
        let controller = self.controller_mut(plug)?;
        let mask = 1_u64 << condition;
        if asserted {
            controller.conditions |= mask;
        } else {
            controller.conditions &= !mask;
        }
        Ok(())
    }

    pub fn set_controller_error(&mut self, plug: usize, error: bool) -> Result<(), String> {
        let controller = self.controller_mut(plug)?;
        controller.error = error;
        if !error {
            controller.conditions &= !controller.error_conditions;
            controller.error_conditions = 0;
        }
        Ok(())
    }

    pub fn set_controller_error_condition(
        &mut self,
        plug: usize,
        condition: u8,
        asserted: bool,
    ) -> Result<(), String> {
        Self::check_controller_condition(condition)?;
        if condition == CONTROLLER_READY_CONDITION {
            return Err("GE-225 controller ready status cannot be an error condition".into());
        }
        let controller = self.controller_mut(plug)?;
        let mask = 1_u64 << condition;
        if asserted {
            controller.conditions |= mask;
            controller.error_conditions |= mask;
        } else {
            controller.conditions &= !mask;
            controller.error_conditions &= !mask;
        }
        controller.error = controller.error_conditions != 0;
        Ok(())
    }

    pub fn set_controller_ready(&mut self, plug: usize, ready: bool) -> Result<(), String> {
        let controller = self.controller_mut(plug)?;
        let transitioned = !controller.ready && ready;
        Self::set_controller_ready_value(controller, ready);
        if transitioned && controller.api_enabled {
            self.pending_controller_interrupts |= 1_u8 << plug;
        }
        Ok(())
    }

    pub fn complete_controller(
        &mut self,
        plug: usize,
        conditions: u64,
        error: bool,
    ) -> Result<(), String> {
        let controller = self.controller_mut(plug)?;
        if !controller.online {
            return Err(format!("GE-225 controller plug {plug} is offline"));
        }
        controller.conditions = conditions;
        controller.error = error;
        controller.error_conditions = 0;
        self.set_controller_ready(plug, true)
    }

    pub fn advance_controller_selector(&mut self) -> bool {
        if !self.controller_selector_busy {
            return false;
        }
        self.controller_selector_busy = false;
        self.selected_controller = None;
        true
    }

    pub fn set_card_reader_api_enabled(&mut self, enabled: bool) {
        self.card_reader_api_enabled = enabled;
    }

    pub fn set_card_punch_api_enabled(&mut self, enabled: bool) {
        self.card_punch_api_enabled = enabled;
    }

    pub fn clear_controller_selector_alarm(&mut self) {
        self.controller_selector_alarm = false;
        self.priority_alarm = self.card_reader_alarm || self.card_punch_alarm;
        self.halted = self.priority_alarm;
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
        let mut padded = words.iter().map(|word| word & MASK_20).collect::<Vec<_>>();
        padded.resize(CARD_DECIMAL_WORDS, 0);
        self.queue_card_reader_card(CardFormat::Decimal, &padded, CardStatus::default())
    }
    pub fn queue_card_reader_card(
        &mut self,
        format: CardFormat,
        words: &[i32],
        status: CardStatus,
    ) -> Result<(), String> {
        let reader_before = self.card_reader_ready();
        let punch_before = self.card_punch_ready();
        if words.len() != format.word_count() {
            return Err(format!(
                "GE-225 {format:?} card requires exactly {} words, got {}",
                format.word_count(),
                words.len()
            ));
        }
        if self.card_reader_queue.len() >= MAX_CARD_QUEUE_DEPTH {
            return Err(format!(
                "GE-225 card-reader queue is full at {MAX_CARD_QUEUE_DEPTH} records"
            ));
        }
        self.card_reader_queue.push_back(CardRecord {
            format,
            words: words.iter().map(|word| word & MASK_20).collect(),
            status,
        });
        self.record_direct_ready_transitions(reader_before, punch_before);
        Ok(())
    }
    pub fn card_punch_output(&self) -> &[CardRecord] {
        &self.card_punch_output
    }
    pub fn take_card_punch_output(&mut self) -> Vec<CardRecord> {
        let reader_before = self.card_reader_ready();
        let punch_before = self.card_punch_ready();
        let output = std::mem::take(&mut self.card_punch_output);
        self.record_direct_ready_transitions(reader_before, punch_before);
        output
    }
    pub fn set_card_reader_online(&mut self, online: bool) {
        let reader_before = self.card_reader_ready();
        let punch_before = self.card_punch_ready();
        self.card_reader_online = online;
        if !online {
            self.card_reader_continuous = None;
        }
        self.record_direct_ready_transitions(reader_before, punch_before);
    }
    pub fn set_card_punch_online(&mut self, online: bool) {
        let reader_before = self.card_reader_ready();
        let punch_before = self.card_punch_ready();
        self.card_punch_online = online;
        self.record_direct_ready_transitions(reader_before, punch_before);
    }
    pub fn set_card_reader_fault(&mut self, fault: bool) {
        let reader_before = self.card_reader_ready();
        let punch_before = self.card_punch_ready();
        self.card_reader_fault = fault;
        if fault {
            self.card_reader_continuous = None;
        }
        self.record_direct_ready_transitions(reader_before, punch_before);
    }
    pub fn set_card_punch_fault(&mut self, fault: bool) {
        let reader_before = self.card_reader_ready();
        let punch_before = self.card_punch_ready();
        self.card_punch_fault = fault;
        self.record_direct_ready_transitions(reader_before, punch_before);
    }
    pub fn set_stop_on_parity_alarm(&mut self, enabled: bool) {
        self.stop_on_parity_alarm = enabled;
    }
    pub fn clear_direct_io_alarms(&mut self) {
        let reader_before = self.card_reader_ready();
        let punch_before = self.card_punch_ready();
        self.card_reader_alarm = false;
        self.card_punch_alarm = false;
        self.priority_alarm = self.controller_selector_alarm;
        self.parity_error = false;
        self.halted = self.controller_selector_alarm;
        self.record_direct_ready_transitions(reader_before, punch_before);
    }
    pub fn queue_paper_tape_input(&mut self, frames: &[i32]) -> Result<(), String> {
        let frames = frames
            .iter()
            .map(|data| PaperTapeFrame {
                data: *data,
                parity_error: false,
            })
            .collect::<Vec<_>>();
        self.queue_paper_tape_frames(&frames)
    }
    pub fn queue_paper_tape_frames(&mut self, frames: &[PaperTapeFrame]) -> Result<(), String> {
        if self.paper_tape_input.len().saturating_add(frames.len()) > MAX_CHARACTER_QUEUE_DEPTH {
            return Err(format!(
                "GE-225 paper-tape input queue exceeds {MAX_CHARACTER_QUEUE_DEPTH} frames"
            ));
        }
        if let Some(frame) = frames
            .iter()
            .find(|frame| !(0..=N_MASK).contains(&frame.data))
        {
            return Err(format!(
                "GE-225 decoded paper-tape frame out of range: {}",
                frame.data
            ));
        }
        self.paper_tape_input.extend(frames.iter().copied());
        Ok(())
    }
    pub fn paper_tape_output(&self) -> &[i32] {
        &self.paper_tape_output
    }
    pub fn take_paper_tape_output(&mut self) -> Vec<i32> {
        std::mem::take(&mut self.paper_tape_output)
    }
    pub fn queue_typewriter_input(&mut self, codes: &[i32]) -> Result<(), String> {
        if self.typewriter_input.len().saturating_add(codes.len()) > MAX_CHARACTER_QUEUE_DEPTH {
            return Err(format!(
                "GE-225 typewriter input queue exceeds {MAX_CHARACTER_QUEUE_DEPTH} characters"
            ));
        }
        if let Some(code) = codes.iter().find(|code| {
            !(0..=N_MASK).contains(code)
                || (typewriter_char(**code).is_none() && !matches!(**code, 0o37 | 0o76))
        }) {
            return Err(format!("invalid GE-225 typewriter input code: {code:o}"));
        }
        self.typewriter_input.extend(codes.iter().copied());
        Ok(())
    }
    pub fn advance_paper_tape_reader(&mut self) -> Result<bool, String> {
        if self.n_device != NRegisterDevice::PaperTapeReader || !self.paper_tape_reader_running {
            return Err("GE-225 paper-tape reader is not running".into());
        }
        let Some(frame) = self.paper_tape_input.pop_front() else {
            return Ok(false);
        };
        self.n_overrun |= self.n_ready;
        self.n = frame.data;
        self.parity_error |= frame.parity_error;
        self.n_ready = true;
        if frame.parity_error && self.stop_on_parity_alarm {
            self.paper_tape_reader_running = false;
            self.priority_alarm = true;
            self.halted = true;
        }
        Ok(true)
    }
    pub fn advance_typewriter_input(&mut self) -> Result<bool, String> {
        if self.n_device != NRegisterDevice::Typewriter || !self.typewriter_keyboard_enabled {
            return Err("GE-225 typewriter keyboard input is not enabled".into());
        }
        let Some(code) = self.typewriter_input.pop_front() else {
            return Ok(false);
        };
        self.n_overrun |= self.n_ready;
        self.n = code;
        self.n_ready = true;
        Ok(true)
    }
    pub fn advance_card_reader(&mut self) -> Result<bool, String> {
        let reader_before = self.card_reader_ready();
        let punch_before = self.card_punch_ready();
        let Some(format) = self.card_reader_continuous else {
            return Err("GE-225 card reader is not in continuous mode".into());
        };
        if self.card_reader_queue.is_empty() {
            self.card_reader_continuous = None;
            self.record_direct_ready_transitions(reader_before, punch_before);
            return Ok(false);
        }
        self.transfer_card_input(format)?;
        self.record_direct_ready_transitions(reader_before, punch_before);
        Ok(true)
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
            return Ok(if decoded.mnemonic == "SEL" {
                let plug = decoded
                    .controller_plug
                    .ok_or_else(|| "GE-225 SEL decoder omitted its plug".to_string())?;
                format!("SEL P{plug}{suffix}")
            } else if decoded.mnemonic == "BCS" {
                let plug = decoded
                    .controller_plug
                    .ok_or_else(|| "GE-225 BCS decoder omitted its plug".to_string())?;
                let condition = decoded.controller_condition.ok_or_else(|| {
                    "GE-225 BCS decoder omitted its controller condition".to_string()
                })?;
                let sense = if decoded.controller_branch_when_set {
                    "SET"
                } else {
                    "CLEAR"
                };
                format!("BCS {condition:02o},P{plug},{sense}")
            } else if let Some(group) = decoded.sxg_group {
                format!("SXG {group}")
            } else if let Some(count) = decoded.count {
                format!("{} {count}{suffix}", decoded.mnemonic)
            } else if let Some(name) = decoded.mnemonic.strip_prefix("AAU_") {
                if aau_branch_word(name).is_some() {
                    format!("BAR {name}")
                } else {
                    name.to_string()
                }
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
        if self.api_branch_inhibit {
            self.api_branch_inhibit = false;
        } else {
            self.enter_api_interrupt_if_pending()?;
        }
        let reader_ready_before = self.card_reader_ready();
        let punch_ready_before = self.card_punch_ready();
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
                    let operand = if let Some(mode) = card_mode_code(execution_decoded.mnemonic) {
                        address | mode
                    } else {
                        address
                    };
                    ir_word = (instruction_word & !ADDR_MASK) | (operand & ADDR_MASK);
                }
            }
        }
        self.preflight_core(
            &execution_decoded,
            effective_address,
            sequential_pc,
            pc_before,
        )?;
        self.preflight_direct_io(&execution_decoded, effective_address)?;
        self.preflight_controller(&execution_decoded, sequential_pc)?;
        self.preflight_decimal(&execution_decoded, effective_address)?;
        self.preflight_aau(&execution_decoded, effective_address, sequential_pc)?;
        self.ir = ir_word;
        self.pc = sequential_pc;
        let a_before = self.a;
        let q_before = self.q;
        let priority_return = self.priority_mode
            && self.priority_return_armed
            && !execution_decoded.fixed_word
            && execution_decoded.mnemonic == "BRU"
            && execution_decoded.modifier.unwrap_or(0) != 0;
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
        if priority_return {
            self.priority_mode = false;
            self.priority_return_armed = false;
            self.selected_x_group = self.interrupted_x_group;
        }
        if !execution_decoded.fixed_word && execution_decoded.mnemonic == "BRU" {
            self.api_branch_inhibit = true;
        }
        self.record_direct_ready_transitions(reader_ready_before, punch_ready_before);
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

    fn controller_mut(&mut self, plug: usize) -> Result<&mut ControllerStatus, String> {
        self.controllers
            .get_mut(plug)
            .ok_or_else(|| format!("GE-225 controller plug out of range: {plug}"))
    }

    fn check_controller_condition(condition: u8) -> Result<(), String> {
        if !(CONTROLLER_CONDITION_MIN..=CONTROLLER_CONDITION_MAX).contains(&condition) {
            return Err(format!(
                "GE-225 controller condition must be {:02o} through {:02o}, got {condition:o}",
                CONTROLLER_CONDITION_MIN, CONTROLLER_CONDITION_MAX
            ));
        }
        Ok(())
    }

    fn set_controller_ready_value(controller: &mut ControllerStatus, ready: bool) {
        controller.ready = ready;
        let mask = 1_u64 << CONTROLLER_READY_CONDITION;
        if ready {
            controller.conditions |= mask;
        } else {
            controller.conditions &= !mask;
        }
    }

    fn record_direct_ready_transitions(&mut self, reader_before: bool, punch_before: bool) {
        if !reader_before && self.card_reader_ready() && self.card_reader_api_enabled {
            self.card_reader_interrupt_pending = true;
        }
        if !punch_before && self.card_punch_ready() && self.card_punch_api_enabled {
            self.card_punch_interrupt_pending = true;
        }
    }

    fn api_interrupt_pending(&self) -> bool {
        self.pending_controller_interrupts != 0
            || self.card_reader_interrupt_pending
            || self.card_punch_interrupt_pending
    }

    fn enter_api_interrupt_if_pending(&mut self) -> Result<(), String> {
        if !self.automatic_interrupt_mode || self.priority_mode || !self.api_interrupt_pending() {
            return Ok(());
        }
        self.check_address(API_SAVED_PC_ADDRESS)?;
        self.check_address(API_VECTOR_ADDRESS)?;
        self.memory[API_SAVED_PC_ADDRESS as usize] = self.pc & MASK_20;
        self.interrupted_x_group = self.selected_x_group;
        self.selected_x_group = API_X_GROUP;
        self.pc = API_VECTOR_ADDRESS;
        self.automatic_interrupt_mode = false;
        self.priority_mode = true;
        self.priority_return_armed = false;
        self.pending_controller_interrupts = 0;
        self.card_reader_interrupt_pending = false;
        self.card_punch_interrupt_pending = false;
        Ok(())
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

    fn card_reader_ready(&self) -> bool {
        self.card_reader_online
            && !self.card_reader_fault
            && self.card_reader_continuous.is_none()
            && !self.card_reader_queue.is_empty()
    }

    fn card_punch_ready(&self) -> bool {
        self.card_punch_online
            && !self.card_punch_fault
            && self.card_punch_output.len() < MAX_CARD_PUNCH_DEPTH
    }

    fn card_address(&self, address: i32) -> Result<i32, String> {
        if !(0..CARD_ADDRESS_LIMIT).contains(&address) || address % CARD_ADDRESS_ALIGNMENT != 0 {
            return Err(format!(
                "GE-225 card I/O base must be a multiple of {CARD_ADDRESS_ALIGNMENT} below {CARD_ADDRESS_LIMIT}, got {address}"
            ));
        }
        Ok(address)
    }

    fn card_sync_word(format: CardFormat, status: CardStatus, hopper_empty: bool) -> i32 {
        let mut word = match format {
            CardFormat::Decimal => CARD_DECIMAL_SYNC,
            CardFormat::Binary10 => CARD_BINARY_SYNC,
            CardFormat::Full12 | CardFormat::MixedDecimal | CardFormat::MixedBinary => {
                CARD_FULL_SYNC
            }
        };
        if hopper_empty {
            word |= DECIMAL_FLAG_BIT;
        } else {
            word &= !DECIMAL_FLAG_BIT;
        }
        if status.output_stacker_full {
            word &= !(1 << 3);
        }
        if status.reader_malfunction {
            word &= !(1 << 2);
        }
        if status.invalid_character {
            word &= !(1 << 1);
        }
        if hopper_empty && status.end_of_file {
            word &= !1;
        } else {
            word |= 1;
        }
        word & MASK_20
    }

    fn reader_alarm(&mut self) {
        self.card_reader_alarm = true;
        self.priority_alarm = true;
        self.card_reader_continuous = None;
        self.halted = true;
    }

    fn punch_alarm(&mut self) {
        self.card_punch_alarm = true;
        self.priority_alarm = true;
        self.halted = true;
    }

    fn transfer_card_input(&mut self, expected: CardFormat) -> Result<(), String> {
        let Some(record) = self.card_reader_queue.front().cloned() else {
            self.card_reader_continuous = None;
            return Ok(());
        };
        if record.format != expected {
            self.reader_alarm();
            return Ok(());
        }
        let offset = match expected {
            CardFormat::Decimal => self.card_reader_slot * 32,
            CardFormat::Binary10 => self.card_reader_slot * 64,
            CardFormat::Full12 | CardFormat::MixedDecimal | CardFormat::MixedBinary => 0,
        };
        let destination = self
            .card_reader_base
            .checked_add(
                i32::try_from(offset)
                    .map_err(|_| "GE-225 card-reader destination offset overflow".to_string())?,
            )
            .ok_or_else(|| "GE-225 card-reader destination overflow".to_string())?;
        let data_range = self.checked_range(destination, record.words.len())?;
        let sync_offset = match expected {
            CardFormat::Decimal => 27,
            CardFormat::Binary10 => 41,
            CardFormat::Full12 | CardFormat::MixedDecimal | CardFormat::MixedBinary => 83,
        };
        let sync_address = destination
            .checked_add(sync_offset)
            .ok_or_else(|| "GE-225 card-reader sync address overflow".to_string())?;
        self.check_address(sync_address)?;
        let hopper_empty = self.card_reader_queue.len() == 1;
        let sync = Self::card_sync_word(record.format, record.status, hopper_empty);
        self.card_reader_queue.pop_front();
        for (address, mut word) in data_range.zip(record.words) {
            if record.format == CardFormat::MixedBinary && address == destination as usize {
                word |= SIGN_BIT;
            }
            self.memory[address] = word & MASK_20;
        }
        self.memory[sync_address as usize] = sync;
        self.card_reader_slot = match expected {
            CardFormat::Decimal => (self.card_reader_slot + 1) % 4,
            CardFormat::Binary10 => (self.card_reader_slot + 1) % 2,
            CardFormat::Full12 | CardFormat::MixedDecimal | CardFormat::MixedBinary => 0,
        };
        if self.card_reader_queue.is_empty() {
            self.card_reader_continuous = None;
        }
        Ok(())
    }

    fn transfer_card_punch(&mut self, format: CardFormat, base: i32) -> Result<(), String> {
        let range = self.checked_range(base, format.word_count())?;
        let words = range.map(|address| self.memory[address]).collect();
        self.card_punch_output.push(CardRecord {
            format,
            words,
            status: CardStatus::default(),
        });
        Ok(())
    }

    fn preflight_direct_io(
        &self,
        decoded: &DecodedInstruction,
        effective_address: Option<i32>,
    ) -> Result<(), String> {
        if let Some(format) = decoded.card_format {
            let base = self.card_address(effective_address.ok_or_else(|| {
                format!("GE-225 {} decoder omitted its card base", decoded.mnemonic)
            })?)?;
            match decoded.mnemonic {
                "RCD" | "RCB" | "RCF" => {
                    self.checked_range(base, format.word_count())?;
                    let sync = match format {
                        CardFormat::Decimal => base + 27,
                        CardFormat::Binary10 => base + 41,
                        CardFormat::Full12 | CardFormat::MixedDecimal | CardFormat::MixedBinary => {
                            base + 83
                        }
                    };
                    self.check_address(sync)?;
                }
                "WCD" | "WCB" | "WCF" => {
                    self.checked_range(base, format.word_count())?;
                }
                _ => {}
            }
        } else if decoded.mnemonic == "RCM" {
            let base = self.card_address(
                effective_address
                    .ok_or_else(|| "GE-225 RCM decoder omitted its card base".to_string())?,
            )?;
            self.checked_range(base, CARD_FULL_WORDS)?;
            self.check_address(base + 83)?;
        }
        match decoded.mnemonic {
            "TYP" if self.typewriter_output.len() >= MAX_CHARACTER_QUEUE_DEPTH => Err(format!(
                "GE-225 typewriter output is full at {MAX_CHARACTER_QUEUE_DEPTH} characters"
            )),
            "WPT" if self.paper_tape_output.len() >= MAX_CHARACTER_QUEUE_DEPTH => Err(format!(
                "GE-225 paper-tape output is full at {MAX_CHARACTER_QUEUE_DEPTH} frames"
            )),
            _ => Ok(()),
        }
    }

    fn preflight_core(
        &self,
        decoded: &DecodedInstruction,
        effective_address: Option<i32>,
        sequential_pc: i32,
        pc_before: i32,
    ) -> Result<(), String> {
        let raw_address = decoded.address.unwrap_or(0);
        let modifier = decoded.modifier.unwrap_or(0);
        if matches!(decoded.mnemonic, "DLD" | "DAD" | "DSU" | "DST" | "DCB") {
            let address = effective_address.ok_or_else(|| {
                format!(
                    "GE-225 {} decoder omitted its effective address",
                    decoded.mnemonic
                )
            })?;
            if address & 1 == 0 {
                self.following_address(address)?;
            }
        }
        if matches!(decoded.mnemonic, "LDX" | "STX") {
            self.check_address(raw_address)?;
        }
        if matches!(
            decoded.mnemonic,
            "BXL" | "BXH" | "LDX" | "SPB" | "INX" | "STX"
        ) {
            self.check_address(self.x_address(modifier as usize)?)?;
        }
        if decoded.mnemonic == "SPB" {
            self.direct_branch_target(pc_before, raw_address)?;
        }
        if decoded.mnemonic == "MOV" {
            let word_count = usize::try_from((-to_signed20(self.q)).max(0))
                .map_err(|_| "GE-225 MOV word count overflow".to_string())?;
            self.checked_range(raw_address, word_count)?;
            self.checked_range(self.a & X_MASK, word_count)?;
            self.check_address(self.x_address(0)?)?;
        }

        let skip = match decoded.mnemonic {
            "BXL" => i32::from((self.get_x_word(modifier as usize)? & ADDR_MASK) >= raw_address),
            "BXH" => i32::from((self.get_x_word(modifier as usize)? & ADDR_MASK) < raw_address),
            "CAB" => {
                let address = effective_address.ok_or_else(|| {
                    "GE-225 CAB decoder omitted its effective address".to_string()
                })?;
                match arith_compare(self.read_word(address)?, self.a) {
                    0 => 1,
                    ordering if ordering < 0 => 2,
                    _ => 0,
                }
            }
            "DCB" => {
                let address = effective_address.ok_or_else(|| {
                    "GE-225 DCB decoder omitted its effective address".to_string()
                })?;
                let first = self.read_word(address)?;
                let second = if address & 1 != 0 {
                    first
                } else {
                    self.read_word(self.following_address(address)?)?
                };
                match arith_compare_double(first, second, self.a, self.q) {
                    0 => 1,
                    ordering if ordering < 0 => 2,
                    _ => 0,
                }
            }
            "BCS" => {
                let plug = decoded
                    .controller_plug
                    .ok_or_else(|| "GE-225 BCS decoder omitted its controller plug".to_string())?;
                let condition = decoded
                    .controller_condition
                    .ok_or_else(|| "GE-225 BCS decoder omitted its condition".to_string())?;
                let asserted = (self.controllers[plug].conditions & (1_u64 << condition)) != 0;
                let branch = if decoded.controller_branch_when_set {
                    asserted
                } else {
                    !asserted
                };
                i32::from(!branch)
            }
            mnemonic => self
                .branch_test_condition(mnemonic)
                .map_or(0, |condition| i32::from(!condition)),
        };
        if skip != 0 {
            let target = sequential_pc
                .checked_add(skip)
                .ok_or_else(|| "GE-225 decision skip overflows P".to_string())?;
            self.check_address(target)?;
        }
        Ok(())
    }

    fn preflight_controller(
        &self,
        decoded: &DecodedInstruction,
        sequential_pc: i32,
    ) -> Result<(), String> {
        if decoded.mnemonic != "SEL" {
            return Ok(());
        }
        let plug = decoded
            .controller_plug
            .ok_or_else(|| "GE-225 SEL decoder omitted its controller plug".to_string())?;
        if self.controller_selector_busy || !self.controllers[plug].online {
            return Ok(());
        }
        self.checked_range(sequential_pc, 2)?;
        let continuation = sequential_pc
            .checked_add(2)
            .ok_or_else(|| "GE-225 SEL continuation overflow".to_string())?;
        self.check_address(continuation)?;
        if self.controller_commands.len() >= MAX_CONTROLLER_COMMANDS {
            return Err(format!(
                "GE-225 controller command capture is full at {MAX_CONTROLLER_COMMANDS} commands"
            ));
        }
        Ok(())
    }

    fn execute_controller_select(&mut self, decoded: &DecodedInstruction) -> Result<(), String> {
        let plug = decoded
            .controller_plug
            .ok_or_else(|| "GE-225 SEL decoder omitted its controller plug".to_string())?;
        if self.controller_selector_busy || !self.controllers[plug].online {
            self.controller_selector_alarm = true;
            self.priority_alarm = true;
            self.halted = true;
            return Ok(());
        }
        let command_word = self.read_word(self.pc)?;
        let address_word = self.read_word(self.following_address(self.pc)?)?;
        let controller = &mut self.controllers[plug];
        controller.error = false;
        controller.conditions &= !controller.error_conditions;
        controller.error_conditions = 0;
        Self::set_controller_ready_value(&mut self.controllers[plug], false);
        self.controller_commands.push(ControllerCommand {
            plug: plug as u8,
            select_word: self.ir,
            command_word,
            address_word,
        });
        self.controller_selector_busy = true;
        self.selected_controller = Some(plug as u8);
        self.advance_pc(2)
    }

    fn execute_controller_status(&mut self, decoded: &DecodedInstruction) -> Result<(), String> {
        let plug = decoded
            .controller_plug
            .ok_or_else(|| "GE-225 BCS decoder omitted its controller plug".to_string())?;
        let condition = decoded
            .controller_condition
            .ok_or_else(|| "GE-225 BCS decoder omitted its condition".to_string())?;
        let asserted = (self.controllers[plug].conditions & (1_u64 << condition)) != 0;
        let branch = if decoded.controller_branch_when_set {
            asserted
        } else {
            !asserted
        };
        if !branch {
            self.advance_pc(1)?;
        }
        Ok(())
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

    fn preflight_aau(
        &self,
        decoded: &DecodedInstruction,
        effective_address: Option<i32>,
        sequential_pc: i32,
    ) -> Result<(), String> {
        let memory_instruction = matches!(
            decoded.mnemonic,
            "FLD" | "FAD" | "FSU" | "FST" | "FMP" | "FDV"
        );
        let general_instruction = decoded.mnemonic.starts_with("AAU_")
            && aau_branch_word(decoded.mnemonic.trim_start_matches("AAU_")).is_none();
        if (memory_instruction || general_instruction) && !self.aau_ready {
            return Err(format!(
                "GE-225 AAU is not ready for {}",
                decoded.mnemonic.trim_start_matches("AAU_")
            ));
        }
        if let Some(branch) = decoded
            .mnemonic
            .strip_prefix("AAU_")
            .filter(|name| aau_branch_word(name).is_some())
        {
            if !self.aau_branch_condition(branch) {
                let target = sequential_pc
                    .checked_add(1)
                    .ok_or_else(|| "GE-225 AAU status skip overflows P".to_string())?;
                self.check_address(target)?;
            }
            return Ok(());
        }
        if !memory_instruction {
            return Ok(());
        }
        let raw_address = decoded
            .address
            .ok_or_else(|| format!("GE-225 {} decoder omitted its address", decoded.mnemonic))?;
        let modifier = decoded.modifier.unwrap_or(0);
        if modifier == 0 && raw_address <= 0o17 {
            return Err(format!(
                "GE-225 AAU {} requires an unmodified address greater than 15: {raw_address:o}",
                decoded.mnemonic
            ));
        }
        if matches!(decoded.mnemonic, "FAD" | "FSU" | "FMP" | "FDV") && self.aau_mode.is_none() {
            return Err(format!(
                "GE-225 AAU {} requires a calculation mode",
                decoded.mnemonic
            ));
        }
        let address = effective_address.ok_or_else(|| {
            format!(
                "GE-225 {} decoder omitted its effective address",
                decoded.mnemonic
            )
        })?;
        if address & 1 == 0 {
            self.following_address(address)?;
        }
        Ok(())
    }

    fn read_aau_operand(&mut self, address: i32) -> Result<u64, String> {
        let first = self.read_word(address)?;
        let second = if address & 1 == 0 {
            self.read_word(self.following_address(address)?)?
        } else {
            first
        };
        self.m = second;
        Ok(pack_aau_words(first, second))
    }

    fn write_aau_operand(&mut self, address: i32, value: u64) -> Result<(), String> {
        let (first, second) = unpack_aau_words(value);
        if address & 1 == 0 {
            let following = self.following_address(address)?;
            self.write_word(address, first)?;
            self.write_word(following, second)?;
        } else {
            self.write_word(address, second)?;
        }
        self.m = second;
        Ok(())
    }

    fn accept_aau_instruction(&mut self) {
        self.aau_overflow = false;
        self.aau_underflow = false;
    }

    fn set_aau_overflow(&mut self) {
        self.aau_overflow = true;
        self.aau_overflow_hold = true;
    }

    fn set_aau_underflow(&mut self) {
        self.aau_underflow = true;
        self.aau_underflow_hold = true;
    }

    fn finish_aau_float_pair(&mut self, mut exponent: i32, mut mantissa: i128, normalize: bool) {
        if mantissa == 0 {
            self.aau_ax = 0;
            self.aau_qx = 0;
            return;
        }
        let maximum = (1_i128 << 60) - 1;
        let minimum = -(1_i128 << 60);
        while mantissa > maximum || mantissa < minimum {
            mantissa >>= 1;
            exponent += 1;
        }
        if normalize {
            while mantissa.unsigned_abs() < (1_u128 << 59) {
                mantissa <<= 1;
                exponent -= 1;
            }
        }
        if exponent > 255 {
            self.set_aau_overflow();
        } else if exponent < -256 {
            self.set_aau_underflow();
            self.aau_ax = 0;
            self.aau_qx = 0;
            return;
        }
        (self.aau_ax, self.aau_qx) = aau_float_pair_raw(exponent, mantissa);
    }

    fn normalize_aau_pair(&mut self) -> Result<(), String> {
        let (exponent, mantissa) = aau_float_pair_parts(self.aau_ax, self.aau_qx);
        self.finish_aau_float_pair(exponent, mantissa, true);
        Ok(())
    }

    fn execute_aau_floating(&mut self, mnemonic: &str, address: i32) -> Result<(), String> {
        let normalized = self.aau_mode == Some(AauMode::NormalizedFloatingPoint);
        self.aau_bx = self.read_aau_operand(address)?;
        let (bx_exponent, bx_mantissa) = aau_float_parts(self.aau_bx);
        match mnemonic {
            "FAD" | "FSU" => {
                let (ax_exponent, ax_mantissa) = aau_float_parts(self.aau_ax);
                let target_exponent = ax_exponent.max(bx_exponent);
                let left =
                    align_aau_float(i128::from(ax_mantissa) << 30, ax_exponent, target_exponent);
                let right =
                    align_aau_float(i128::from(bx_mantissa) << 30, bx_exponent, target_exponent);
                let result = if mnemonic == "FAD" {
                    left + right
                } else {
                    left - right
                };
                self.finish_aau_float_pair(target_exponent, result, normalized);
            }
            "FMP" => {
                let (qx_exponent, qx_mantissa) = aau_float_parts(self.aau_qx);
                let product = i128::from(qx_mantissa) * i128::from(bx_mantissa);
                self.finish_aau_float_pair(qx_exponent + bx_exponent, product, normalized);
            }
            "FDV" => {
                let (mut ax_exponent, dividend) = aau_float_pair_parts(self.aau_ax, self.aau_qx);
                if bx_mantissa == 0 {
                    if dividend < 0 {
                        (self.aau_ax, self.aau_qx) =
                            aau_float_pair_raw(ax_exponent, dividend.abs());
                    }
                    self.set_aau_overflow();
                    return Ok(());
                }
                if dividend == 0 {
                    self.aau_ax = 0;
                    self.aau_qx = 0;
                    return Ok(());
                }
                let dividend_negative = dividend < 0;
                let divisor_negative = bx_mantissa < 0;
                let mut dividend_magnitude = dividend.abs();
                let divisor_magnitude = i128::from(bx_mantissa).abs();
                if (dividend_magnitude >> 30) >= divisor_magnitude {
                    dividend_magnitude >>= 1;
                    ax_exponent += 1;
                    if (dividend_magnitude >> 30) >= divisor_magnitude {
                        (self.aau_ax, self.aau_qx) =
                            aau_float_pair_raw(ax_exponent, dividend_magnitude);
                        self.set_aau_overflow();
                        return Ok(());
                    }
                }
                let mut quotient_exponent = ax_exponent - bx_exponent;
                let quotient_magnitude = dividend_magnitude / divisor_magnitude;
                let mut quotient = if dividend_negative ^ divisor_negative {
                    -quotient_magnitude
                } else {
                    quotient_magnitude
                };
                while !(-(1_i128 << 30)..=(1_i128 << 30) - 1).contains(&quotient) {
                    quotient >>= 1;
                    quotient_exponent += 1;
                }
                if normalized {
                    while quotient != 0 && quotient.unsigned_abs() < (1_u128 << 29) {
                        quotient <<= 1;
                        quotient_exponent -= 1;
                    }
                }
                if quotient_exponent > 255 {
                    self.set_aau_overflow();
                } else if quotient_exponent < -256 {
                    self.set_aau_underflow();
                    self.aau_ax = 0;
                    self.aau_qx = 0;
                } else {
                    let remainder_magnitude = dividend_magnitude % divisor_magnitude;
                    let remainder = if dividend_negative {
                        -remainder_magnitude
                    } else {
                        remainder_magnitude
                    };
                    self.aau_ax = aau_float_raw(quotient_exponent, quotient as i64);
                    self.aau_qx = aau_float_raw(
                        quotient_exponent - 30,
                        remainder.clamp(-(1_i128 << 30), (1_i128 << 30) - 1) as i64,
                    );
                }
            }
            _ => {
                return Err(format!(
                    "unimplemented GE-225 AAU floating instruction: {mnemonic}"
                ))
            }
        }
        Ok(())
    }

    fn execute_aau_memory(&mut self, mnemonic: &str, address: i32) -> Result<(), String> {
        self.aau_ix = self.ir as u64;
        self.accept_aau_instruction();
        match mnemonic {
            "FLD" => self.aau_ax = self.read_aau_operand(address)?,
            "FST" => self.write_aau_operand(address, self.aau_ax)?,
            "FAD" | "FSU" if self.aau_mode == Some(AauMode::FixedPoint) => {
                self.aau_bx = self.read_aau_operand(address)?;
                let left = aau_fixed_value(self.aau_ax);
                let right = aau_fixed_value(self.aau_bx);
                let total = if mnemonic == "FAD" {
                    i128::from(left) + i128::from(right)
                } else {
                    i128::from(left) - i128::from(right)
                };
                let maximum = (1_i128 << AAU_FIXED_DATA_BITS) - 1;
                let minimum = -(1_i128 << AAU_FIXED_DATA_BITS);
                if total > maximum {
                    self.set_aau_overflow();
                } else if total < minimum {
                    self.set_aau_underflow();
                }
                self.aau_ax = aau_fixed_raw(total as i64);
            }
            "FMP" if self.aau_mode == Some(AauMode::FixedPoint) => {
                self.aau_bx = self.read_aau_operand(address)?;
                let product = i128::from(aau_fixed_value(self.aau_qx))
                    * i128::from(aau_fixed_value(self.aau_bx));
                (self.aau_ax, self.aau_qx) = split_aau_fixed_pair(product);
            }
            "FDV" if self.aau_mode == Some(AauMode::FixedPoint) => {
                self.aau_bx = self.read_aau_operand(address)?;
                let divisor = i128::from(aau_fixed_value(self.aau_bx));
                let dividend = aau_fixed_pair_value(self.aau_ax, self.aau_qx);
                let high = i128::from(aau_fixed_value(self.aau_ax));
                if divisor == 0 || high.abs() >= divisor.abs() {
                    if dividend < 0 {
                        (self.aau_ax, self.aau_qx) = split_aau_fixed_pair(-dividend);
                    }
                    self.set_aau_overflow();
                } else if dividend != 0 {
                    let quotient = dividend / divisor;
                    let remainder = dividend % divisor;
                    self.aau_ax = aau_fixed_raw(quotient as i64);
                    self.aau_qx = aau_fixed_raw(remainder as i64);
                }
            }
            "FAD" | "FSU" | "FMP" | "FDV" => {
                self.execute_aau_floating(mnemonic, address)?;
            }
            _ => return Err(format!("unimplemented GE-225 AAU instruction: {mnemonic}")),
        }
        Ok(())
    }

    fn execute_aau_fixed(&mut self, mnemonic: &str) -> Result<(), String> {
        self.aau_ix = self.ir as u64;
        if let Some(branch) = mnemonic
            .strip_prefix("AAU_")
            .filter(|name| aau_branch_word(name).is_some())
        {
            let condition = self.aau_branch_condition(branch);
            if matches!(branch, "BOO" | "BON") && self.aau_overflow_hold {
                self.aau_overflow_hold = false;
            }
            if matches!(branch, "BUO" | "BUN") && self.aau_underflow_hold {
                self.aau_underflow_hold = false;
            }
            if !condition {
                self.advance_pc(1)?;
            }
            return Ok(());
        }

        self.accept_aau_instruction();
        match mnemonic {
            "AAU_SET_FIXPOINT" => self.aau_mode = Some(AauMode::FixedPoint),
            "AAU_SET_NFLPOINT" => self.aau_mode = Some(AauMode::NormalizedFloatingPoint),
            "AAU_SET_UFLPOINT" => self.aau_mode = Some(AauMode::UnnormalizedFloatingPoint),
            "AAU_LAQ" => self.aau_ax = self.aau_qx,
            "AAU_LQA" => self.aau_qx = self.aau_ax,
            "AAU_MAQ" => {
                self.aau_qx = self.aau_ax;
                self.aau_ax = 0;
            }
            "AAU_XAQ" => std::mem::swap(&mut self.aau_ax, &mut self.aau_qx),
            "AAU_ROV" => self.aau_overflow_hold = false,
            "AAU_RUN" => self.aau_underflow_hold = false,
            "AAU_RIN" => {
                self.aau_overflow_hold = false;
                self.aau_underflow_hold = false;
            }
            "AAU_NOX" => self.normalize_aau_pair()?,
            _ => {
                return Err(format!(
                    "unimplemented GE-225 AAU general instruction: {mnemonic}"
                ))
            }
        }
        Ok(())
    }

    fn aau_branch_condition(&self, branch: &str) -> bool {
        let floating = matches!(
            self.aau_mode,
            Some(AauMode::NormalizedFloatingPoint | AauMode::UnnormalizedFloatingPoint)
        );
        let (first, second) = unpack_aau_words(self.aau_ax);
        let minus = if floating {
            (second & SIGN_BIT) != 0
        } else {
            (first & SIGN_BIT) != 0
        };
        match branch {
            "BAR" => self.aau_ready,
            "BAN" => !self.aau_ready,
            "BMI" => minus,
            "BPL" => !minus,
            "BZE" => self.aau_ax == 0,
            "BNZ" => self.aau_ax != 0,
            "BOV" => self.aau_overflow,
            "BNO" => !self.aau_overflow,
            "BUF" => self.aau_underflow,
            "BNU" => !self.aau_underflow,
            "BOO" => self.aau_overflow_hold,
            "BON" => !self.aau_overflow_hold,
            "BUO" => self.aau_underflow_hold,
            "BUN" => !self.aau_underflow_hold,
            "BER" => self.aau_overflow || self.aau_underflow,
            "BNE" => !self.aau_overflow && !self.aau_underflow,
            _ => false,
        }
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
            "FLD" | "FAD" | "FSU" | "FST" | "FMP" | "FDV" => {
                self.execute_aau_memory(mnemonic, effective_address)?;
            }
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
            "RCD" | "RCB" | "RCF" => {
                if !self.card_reader_ready() {
                    self.reader_alarm();
                } else {
                    let format = match mnemonic {
                        "RCD" => CardFormat::Decimal,
                        "RCB" => CardFormat::Binary10,
                        "RCF" => CardFormat::Full12,
                        _ => {
                            return Err(format!(
                                "GE-225 internal card-reader mode error: {mnemonic}"
                            ));
                        }
                    };
                    self.card_reader_base = effective_address;
                    self.card_reader_slot = 0;
                    self.card_reader_continuous = if matches!(mnemonic, "RCD" | "RCB") {
                        Some(format)
                    } else {
                        None
                    };
                    self.transfer_card_input(format)?;
                    if mnemonic == "RCF" && self.card_reader_ready() && self.card_reader_api_enabled
                    {
                        self.card_reader_interrupt_pending = true;
                    }
                }
            }
            "RCM" => {
                if !self.card_reader_ready() {
                    self.reader_alarm();
                } else {
                    let format = self
                        .card_reader_queue
                        .front()
                        .map(|record| record.format)
                        .unwrap_or(CardFormat::MixedDecimal);
                    if !matches!(format, CardFormat::MixedDecimal | CardFormat::MixedBinary) {
                        self.reader_alarm();
                    } else {
                        self.card_reader_base = effective_address;
                        self.card_reader_slot = 0;
                        self.card_reader_continuous = None;
                        self.transfer_card_input(format)?;
                        if self.card_reader_ready() && self.card_reader_api_enabled {
                            self.card_reader_interrupt_pending = true;
                        }
                    }
                }
            }
            "WCD" | "WCB" | "WCF" => {
                if !self.card_punch_ready() {
                    self.punch_alarm();
                } else {
                    let format = match mnemonic {
                        "WCD" => CardFormat::Decimal,
                        "WCB" => CardFormat::Binary10,
                        "WCF" => CardFormat::Full12,
                        _ => {
                            return Err(format!(
                                "GE-225 internal card-punch mode error: {mnemonic}"
                            ));
                        }
                    };
                    self.transfer_card_punch(format, effective_address)?;
                    if self.card_punch_ready() && self.card_punch_api_enabled {
                        self.card_punch_interrupt_pending = true;
                    }
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
        if mnemonic.starts_with("AAU_") {
            return self.execute_aau_fixed(mnemonic);
        }
        match mnemonic {
            "HCR" => self.card_reader_continuous = None,
            "OFF" => {
                self.typewriter_power = false;
                self.n_device = NRegisterDevice::Off;
                self.paper_tape_reader_running = false;
                self.typewriter_keyboard_enabled = false;
                self.n_ready = false;
            }
            "TYP" => {
                if !self.typewriter_power || self.n_device != NRegisterDevice::Typewriter {
                    self.n_ready = false;
                    return Ok(());
                }
                let code = self.n & N_MASK;
                if code == 0o37 {
                    self.typewriter_output.push("\r".into());
                } else if code == 0o76 {
                    self.typewriter_output.push("\t".into());
                } else if code != 0o72 && code != 0o75 {
                    if let Some(ch) = typewriter_char(code) {
                        self.typewriter_output.push(ch.into());
                    } else {
                        self.n_ready = false;
                        return Ok(());
                    }
                }
                self.n_ready = true;
            }
            "RPT" => {
                self.paper_tape_reader_running = true;
                self.n_ready = false;
                self.advance_paper_tape_reader()?;
            }
            "WPT" => {
                self.paper_tape_output.push(self.n & N_MASK);
                self.n_ready = true;
            }
            "NIO" => self.n_ready = false,
            "TON" => {
                self.n_device = NRegisterDevice::Typewriter;
                self.typewriter_power = true;
                self.paper_tape_reader_running = false;
                self.typewriter_keyboard_enabled = false;
                self.n_ready = true;
            }
            "RON" => {
                self.n_device = NRegisterDevice::PaperTapeReader;
                self.typewriter_power = false;
                self.paper_tape_reader_running = false;
                self.typewriter_keyboard_enabled = false;
                self.n = 0;
                self.n_ready = false;
            }
            "PON" => {
                self.n_device = NRegisterDevice::PaperTapePunch;
                self.typewriter_power = false;
                self.paper_tape_reader_running = false;
                self.typewriter_keyboard_enabled = false;
                self.n_ready = true;
            }
            "RCS" => self.a |= self.control_switches,
            "HPT" => match self.n_device {
                NRegisterDevice::PaperTapeReader => self.paper_tape_reader_running = false,
                NRegisterDevice::Typewriter => {
                    self.typewriter_keyboard_enabled = true;
                    self.n_ready = false;
                }
                NRegisterDevice::Off | NRegisterDevice::PaperTapePunch => {}
            },
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
            "SEL" => self.execute_controller_select(decoded)?,
            "BCS" => self.execute_controller_status(decoded)?,
            "SET_PST" => {
                self.automatic_interrupt_mode = true;
                if self.priority_mode {
                    self.priority_return_armed = true;
                }
            }
            "SET_PBK" => self.automatic_interrupt_mode = false,
            "BOD" | "BEV" | "BMI" | "BPL" | "BZE" | "BNZ" | "BOV" | "BNO" | "BPE" | "BPC"
            | "BNR" | "BNN" | "BCR" | "BCN" | "BPR" | "BPN" => {
                self.execute_branch_test(mnemonic)?
            }
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
        let cond = self.branch_test_condition(mnemonic).unwrap_or(false);
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

    fn branch_test_condition(&self, mnemonic: &str) -> Option<bool> {
        Some(match mnemonic {
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
            "BCR" => self.card_reader_ready(),
            "BCN" => !self.card_reader_ready(),
            "BPR" => self.card_punch_ready(),
            "BPN" => !self.card_punch_ready(),
            _ => return None,
        })
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
        if matches!(mnemonic, "SNA" | "NAQ" | "ANQ") {
            self.n_ready = false;
        }
        Ok(())
    }

    fn decode_word(&self, word: i32) -> Result<DecodedInstruction, String> {
        let normalized = word & MASK_20;
        let (opcode, modifier, address) = decode_instruction(normalized);
        let canonical = normalized & !MODIFIER_MASK;
        if let Some(name) = aau_general_name(normalized) {
            return Ok(DecodedInstruction {
                mnemonic: name,
                modifier: Some(0),
                address: None,
                count: None,
                sxg_group: None,
                card_format: None,
                controller_plug: None,
                controller_condition: None,
                controller_branch_when_set: false,
                fixed_word: true,
            });
        }
        if let Some(name) = aau_branch_name(normalized) {
            return Ok(DecodedInstruction {
                mnemonic: name,
                modifier: Some(0),
                address: None,
                count: None,
                sxg_group: None,
                card_format: None,
                controller_plug: None,
                controller_condition: None,
                controller_branch_when_set: false,
                fixed_word: true,
            });
        }
        if canonical == 0o2500006 {
            let mnemonic = match self.n_device {
                NRegisterDevice::Typewriter => "TYP",
                NRegisterDevice::PaperTapeReader => "RPT",
                NRegisterDevice::PaperTapePunch => "WPT",
                NRegisterDevice::Off => "NIO",
            };
            return Ok(DecodedInstruction {
                mnemonic,
                modifier: Some(modifier),
                address: None,
                count: None,
                sxg_group: None,
                card_format: None,
                controller_plug: None,
                controller_condition: None,
                controller_branch_when_set: false,
                fixed_word: true,
            });
        }
        if (canonical & !CONTROLLER_PLUG_MASK) == CONTROLLER_SELECT_BASE {
            return Ok(DecodedInstruction {
                mnemonic: "SEL",
                modifier: Some(modifier),
                address: None,
                count: None,
                sxg_group: None,
                card_format: None,
                controller_plug: Some(((canonical & CONTROLLER_PLUG_MASK) >> 6) as usize),
                controller_condition: None,
                controller_branch_when_set: false,
                fixed_word: true,
            });
        }
        let controller_status_base =
            normalized & !(CONTROLLER_PLUG_MASK | CONTROLLER_CONDITION_MASK);
        let controller_condition = (normalized & CONTROLLER_CONDITION_MASK) as u8;
        if matches!(
            controller_status_base,
            CONTROLLER_STATUS_SET_BASE | CONTROLLER_STATUS_CLEAR_BASE
        ) && (CONTROLLER_CONDITION_MIN..=CONTROLLER_CONDITION_MAX)
            .contains(&controller_condition)
        {
            return Ok(DecodedInstruction {
                mnemonic: "BCS",
                modifier: None,
                address: None,
                count: None,
                sxg_group: None,
                card_format: None,
                controller_plug: Some(((normalized & CONTROLLER_PLUG_MASK) >> 6) as usize),
                controller_condition: Some(controller_condition),
                controller_branch_when_set: controller_status_base == CONTROLLER_STATUS_SET_BASE,
                fixed_word: true,
            });
        }
        if let Some(name) = fixed_name(canonical) {
            return Ok(DecodedInstruction {
                mnemonic: name,
                modifier: Some(modifier),
                address: None,
                count: None,
                sxg_group: None,
                card_format: None,
                controller_plug: None,
                controller_condition: None,
                controller_branch_when_set: false,
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
                card_format: None,
                controller_plug: None,
                controller_condition: None,
                controller_branch_when_set: false,
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
                        card_format: None,
                        controller_plug: None,
                        controller_condition: None,
                        controller_branch_when_set: false,
                        fixed_word: true,
                    });
                }
            }
        }
        if opcode == OP_RCD {
            if (address & CARD_RESERVED_MASK) != 0 {
                return Err(format!(
                    "GE-225 card instruction has nonzero reserved bits: {normalized:07o}"
                ));
            }
            let base = address & !(CARD_ADDRESS_ALIGNMENT - 1);
            if base >= CARD_ADDRESS_LIMIT {
                return Err(format!(
                    "GE-225 card instruction base must be below {CARD_ADDRESS_LIMIT}: {base}"
                ));
            }
            let (mnemonic, card_format) = match address & CARD_MODE_MASK {
                0o00 => ("RCD", Some(CardFormat::Decimal)),
                0o01 => ("RCB", Some(CardFormat::Binary10)),
                0o02 => ("WCD", Some(CardFormat::Decimal)),
                0o03 => ("WCB", Some(CardFormat::Binary10)),
                0o10 => ("RCF", Some(CardFormat::Full12)),
                0o12 => ("RCM", None),
                0o17 => ("WCF", Some(CardFormat::Full12)),
                mode => {
                    return Err(format!(
                        "unknown GE-225 card instruction mode {mode:02o} in {normalized:07o}"
                    ))
                }
            };
            return Ok(DecodedInstruction {
                mnemonic,
                modifier: Some(modifier),
                address: Some(base),
                count: None,
                sxg_group: None,
                card_format,
                controller_plug: None,
                controller_condition: None,
                controller_branch_when_set: false,
                fixed_word: false,
            });
        }
        let mnemonic = base_opcode_name(opcode)
            .ok_or_else(|| format!("unknown GE-225 opcode field {opcode:o}"))?;
        Ok(DecodedInstruction {
            mnemonic,
            modifier: Some(modifier),
            address: Some(address),
            count: None,
            sxg_group: None,
            card_format: None,
            controller_plug: None,
            controller_condition: None,
            controller_branch_when_set: false,
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
        sim.load_words(
            &[
                assemble_card_io("RCD", 128, 0).unwrap(),
                assemble_fixed("NOP").unwrap(),
            ],
            64,
        )
        .unwrap();
        sim.set_program_counter(64).unwrap();
        sim.run(2).unwrap();
        let state = sim.get_state();
        assert_eq!(state.memory[128], 0x11111);
        assert_eq!(state.memory[129], 0x22222);
    }
}
