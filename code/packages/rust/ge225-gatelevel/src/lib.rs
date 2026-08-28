//! Gate-level General Electric GE-225 simulator.
//!
//! Persistent words and registers are arrays of simulated D flip-flops.
//! Arithmetic and logic results use the repository's ripple-carry and primitive
//! gate networks. Host control flow sequences a clocked instruction cycle and
//! host integers identify memory addresses and trace fields.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};
use logic_gates::sequential::{register, FlipFlopState};
use std::collections::VecDeque;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const MIN_MEMORY_WORDS: usize = 4_096;
pub const MAX_MEMORY_WORDS: usize = 16_384;
pub const WORD_MASK: i32 = (1 << 20) - 1;
pub const DATA_MASK: i32 = (1 << 19) - 1;
pub const SIGN_BIT: i32 = 1 << 19;
pub const ADDRESS_MASK: i32 = 0x1fff;
const X_MASK: i32 = 0x7fff;
const CLOCK_DAY_SIXTHS: i32 = 24 * 60 * 60 * 6;
const N_MASK: i32 = 0x3f;
const MAX_CARD_QUEUE_DEPTH: usize = 64;
const MAX_CARD_PUNCH_DEPTH: usize = 64;
const MAX_CHARACTER_QUEUE_DEPTH: usize = 65_536;
const CARD_ADDRESS_ALIGNMENT: i32 = 128;
const CARD_ADDRESS_LIMIT: i32 = 2_048;
const CARD_DECIMAL_SYNC: i32 = 0o2606077;
const CARD_BINARY_SYNC: i32 = 0o2001777;
const CARD_FULL_SYNC: i32 = 0o2007777;
const CONTROLLER_COUNT: usize = 8;
const MAX_CONTROLLER_COMMANDS: usize = 64;
const CONTROLLER_READY_CONDITION: u8 = 0o20;
const CONTROLLER_CONDITION_MIN: u8 = 0o20;
const CONTROLLER_CONDITION_MAX: u8 = 0o35;
const CONTROLLER_PLUG_MASK: i32 = 0o700;
const CONTROLLER_CONDITION_MASK: i32 = 0o77;
const CONTROLLER_SELECT_BASE: i32 = 0o2500020;
const CONTROLLER_STATUS_SET_BASE: i32 = 0o2514000;
const CONTROLLER_STATUS_CLEAR_BASE: i32 = 0o2516000;
const API_SAVED_PC_ADDRESS: i32 = 0o201;
const API_VECTOR_ADDRESS: i32 = 0o204;
const API_X_GROUP: usize = 32;
const AAU_FIXED_WORD_BITS: usize = 39;

/// Persistent non-memory bits in the complete P006 GE-225 model.
pub const CENTRAL_FLIP_FLOPS: usize = 1_437;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AauMode {
    FixedPoint,
    NormalizedFloatingPoint,
    UnnormalizedFloatingPoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
            Self::Decimal => 27,
            Self::Binary10 => 40,
            Self::Full12 | Self::MixedDecimal | Self::MixedBinary => 80,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CardStatus {
    pub invalid_character: bool,
    pub output_stacker_full: bool,
    pub reader_malfunction: bool,
    pub end_of_file: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardRecord {
    pub format: CardFormat,
    pub words: Vec<i32>,
    pub status: CardStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NRegisterDevice {
    Off,
    Typewriter,
    PaperTapeReader,
    PaperTapePunch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PaperTapeFrame {
    pub data: i32,
    pub parity_error: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControllerStatus {
    pub online: bool,
    pub ready: bool,
    pub error: bool,
    pub conditions: u64,
    pub error_conditions: u64,
    pub api_enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControllerCommand {
    pub plug: u8,
    pub select_word: i32,
    pub command_word: i32,
    pub address_word: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BitRegister<const WIDTH: usize> {
    state: [FlipFlopState; WIDTH],
}

type DecimalPairResult = ([u8; 20], [u8; 20], [u8; 2], u8);

impl<const WIDTH: usize> BitRegister<WIDTH> {
    fn zero() -> Self {
        Self::new(&[0; WIDTH])
    }

    fn new(initial: &[u8; WIDTH]) -> Self {
        let mut value = Self {
            state: std::array::from_fn(|_| FlipFlopState::default()),
        };
        value.write(initial);
        value
    }

    fn read(&self) -> [u8; WIDTH] {
        let mut state = self.state.clone();
        register(&[0; WIDTH], 0, &mut state)
            .try_into()
            .expect("a fixed-width gate register preserves its width")
    }

    fn write(&mut self, data: &[u8; WIDTH]) {
        register(data, 0, &mut self.state);
        register(data, 1, &mut self.state);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ge225GateState {
    pub a: i32,
    pub q: i32,
    pub m: i32,
    pub n: i32,
    pub pc: i32,
    pub ir: i32,
    pub overflow: bool,
    pub parity_error: bool,
    pub decimal_mode: bool,
    pub decimal_carry: i32,
    pub clock_sixths: i32,
    pub n_ready: bool,
    pub selected_x_group: usize,
    pub halted: bool,
    pub card_reader_ready: bool,
    pub card_punch_ready: bool,
    pub card_reader_continuous: Option<CardFormat>,
    pub card_reader_base: i32,
    pub card_reader_slot: usize,
    pub card_reader_online: bool,
    pub card_punch_online: bool,
    pub card_reader_fault: bool,
    pub card_punch_fault: bool,
    pub card_reader_alarm: bool,
    pub card_punch_alarm: bool,
    pub priority_alarm: bool,
    pub n_device: NRegisterDevice,
    pub typewriter_power: bool,
    pub paper_tape_reader_running: bool,
    pub typewriter_keyboard_enabled: bool,
    pub n_overrun: bool,
    pub stop_on_parity_alarm: bool,
    pub control_switches: i32,
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
    pub aau: AauState,
    pub memory: Vec<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub pc_before: i32,
    pub pc_after: i32,
    pub instruction: i32,
    pub mnemonic: String,
    pub a_before: i32,
    pub a_after: i32,
    pub q_before: i32,
    pub q_after: i32,
    pub effective_address: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Ge225GateError {
    InvalidMemorySize {
        words: usize,
    },
    InvalidOrigin {
        origin: usize,
    },
    ProgramTooLarge {
        words: usize,
        capacity: usize,
    },
    AddressOutOfRange {
        address: i32,
        capacity: usize,
    },
    Halted,
    UnknownInstruction {
        word: i32,
        pc: i32,
    },
    InvalidAutomaticModification {
        word: i32,
    },
    ShiftCountOutOfRange {
        count: i32,
    },
    InvalidBcd {
        word: i32,
    },
    FlaggedDecimalOperand {
        double: bool,
    },
    InvalidClock {
        value: i32,
    },
    InvalidCardRecordLength {
        format: CardFormat,
        words: usize,
    },
    CardReaderQueueFull,
    InvalidCardAddress {
        address: i32,
    },
    InvalidCharacter {
        code: i32,
    },
    CharacterQueueFull,
    DeviceNotActive {
        device: &'static str,
    },
    ControllerPlugOutOfRange {
        plug: usize,
    },
    ControllerConditionOutOfRange {
        condition: u8,
    },
    ControllerReadyCannotBeError,
    ControllerOffline {
        plug: usize,
    },
    ControllerCommandQueueFull,
    AauNotReady {
        instruction: &'static str,
    },
    AauModeRequired {
        instruction: &'static str,
    },
    InvalidAauAddress {
        instruction: &'static str,
        address: i32,
    },
}

impl Display for Ge225GateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMemorySize { words } => write!(
                formatter,
                "memory must contain {MIN_MEMORY_WORDS}..={MAX_MEMORY_WORDS} words, got {words}"
            ),
            Self::InvalidOrigin { origin } => write!(formatter, "invalid load origin {origin}"),
            Self::ProgramTooLarge { words, capacity } => {
                write!(
                    formatter,
                    "program has {words} words but capacity is {capacity}"
                )
            }
            Self::AddressOutOfRange { address, capacity } => {
                write!(
                    formatter,
                    "address {address} is outside {capacity}-word memory"
                )
            }
            Self::Halted => write!(formatter, "the GE-225 gate-level simulator is halted"),
            Self::UnknownInstruction { word, pc } => {
                write!(formatter, "unknown GE-225 word {word:07o} at P={pc:05o}")
            }
            Self::InvalidAutomaticModification { word } => write!(
                formatter,
                "GE-225 automatic modification produced an invalid fixed word {word:07o}"
            ),
            Self::ShiftCountOutOfRange { count } => {
                write!(formatter, "modified GE-225 shift count exceeds 31: {count}")
            }
            Self::InvalidBcd { word } => {
                write!(
                    formatter,
                    "invalid GE-225 BCD digits in word {:07o}",
                    word & WORD_MASK
                )
            }
            Self::FlaggedDecimalOperand { double } => write!(
                formatter,
                "GE-225 {}decimal operand is flagged while A is unflagged",
                if *double { "double-" } else { "" }
            ),
            Self::InvalidClock { value } => {
                write!(
                    formatter,
                    "GE-225 clock must fit its 19-bit C register, got {value}"
                )
            }
            Self::InvalidCardRecordLength { format, words } => write!(
                formatter,
                "GE-225 {format:?} card requires exactly {} words, got {words}",
                format.word_count()
            ),
            Self::CardReaderQueueFull => write!(
                formatter,
                "GE-225 card-reader queue is full at {MAX_CARD_QUEUE_DEPTH} records"
            ),
            Self::InvalidCardAddress { address } => write!(
                formatter,
                "GE-225 card I/O base must be a multiple of {CARD_ADDRESS_ALIGNMENT} below {CARD_ADDRESS_LIMIT}, got {address}"
            ),
            Self::InvalidCharacter { code } => {
                write!(formatter, "invalid GE-225 character code: {code:o}")
            }
            Self::CharacterQueueFull => write!(
                formatter,
                "GE-225 character queue exceeds {MAX_CHARACTER_QUEUE_DEPTH} entries"
            ),
            Self::DeviceNotActive { device } => {
                write!(formatter, "GE-225 {device} is not active")
            }
            Self::ControllerPlugOutOfRange { plug } => {
                write!(formatter, "GE-225 controller plug out of range: {plug}")
            }
            Self::ControllerConditionOutOfRange { condition } => write!(
                formatter,
                "GE-225 controller condition must be {CONTROLLER_CONDITION_MIN:02o} through {CONTROLLER_CONDITION_MAX:02o}, got {condition:o}"
            ),
            Self::ControllerReadyCannotBeError => {
                write!(formatter, "GE-225 controller ready status cannot be an error condition")
            }
            Self::ControllerOffline { plug } => {
                write!(formatter, "GE-225 controller plug {plug} is offline")
            }
            Self::ControllerCommandQueueFull => write!(
                formatter,
                "GE-225 controller command capture is full at {MAX_CONTROLLER_COMMANDS} commands"
            ),
            Self::AauNotReady { instruction } => {
                write!(formatter, "GE-225 AAU is not ready for {instruction}")
            }
            Self::AauModeRequired { instruction } => {
                write!(formatter, "GE-225 AAU {instruction} requires a calculation mode")
            }
            Self::InvalidAauAddress {
                instruction,
                address,
            } => write!(
                formatter,
                "GE-225 AAU {instruction} requires an unmodified address greater than 15: {address:o}"
            ),
        }
    }
}

impl Error for Ge225GateError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operation {
    Lda,
    Add,
    Sub,
    Sta,
    Bxl,
    Bxh,
    Ldx,
    Spb,
    Dld,
    Dad,
    Dsu,
    Dst,
    Inx,
    Mpy,
    Dvd,
    Stx,
    Ext,
    Cab,
    Dcb,
    Ory,
    Mov,
    Bru,
    Sto,
    Ldz,
    Ldo,
    Lmo,
    Cpl,
    Neg,
    Chs,
    Nop,
    Laq,
    Lqa,
    Xaq,
    Maq,
    Ado,
    Sbo,
    Lac,
    Lca,
    SetDecimalMode,
    SetBinaryMode,
    Bod,
    Bev,
    Bmi,
    Bpl,
    Bze,
    Bnz,
    Bov,
    Bno,
    Sra,
    Sna,
    Sca,
    San,
    Srd,
    Naq,
    Scd,
    Anq,
    Sla,
    Sld,
    Nor,
    Dno,
    Sxg,
    Rcd,
    Rcb,
    Wcd,
    Wcb,
    Rcf,
    Rcm,
    Wcf,
    Hcr,
    Off,
    NCommand,
    Ton,
    Rcs,
    Ron,
    Pon,
    Hpt,
    Bpe,
    Bpc,
    Bnr,
    Bnn,
    Bcr,
    Bcn,
    Bpr,
    Bpn,
    Sel,
    BcsSet,
    BcsClear,
    SetPst,
    SetPbk,
    Fld,
    Fad,
    Fsu,
    Fst,
    Fmp,
    Fdv,
    AauSetFixpoint,
    AauSetNflpoint,
    AauSetUflpoint,
    AauLaq,
    AauLqa,
    AauMaq,
    AauXaq,
    AauRov,
    AauRun,
    AauRin,
    AauNox,
    AauBar,
    AauBan,
    AauBmi,
    AauBpl,
    AauBze,
    AauBnz,
    AauBov,
    AauBno,
    AauBuf,
    AauBnu,
    AauBoo,
    AauBon,
    AauBuo,
    AauBun,
    AauBer,
    AauBne,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ge225GateLevel {
    memory: Vec<BitRegister<20>>,
    a: BitRegister<20>,
    q: BitRegister<20>,
    m: BitRegister<20>,
    n: BitRegister<6>,
    pc: BitRegister<15>,
    ir: BitRegister<20>,
    overflow: BitRegister<1>,
    parity_error: BitRegister<1>,
    decimal_mode: BitRegister<1>,
    decimal_carry: BitRegister<2>,
    clock_sixths: BitRegister<19>,
    n_ready: BitRegister<1>,
    selected_x_group: BitRegister<6>,
    halted: BitRegister<1>,
    card_reader_continuous: BitRegister<2>,
    card_reader_base: BitRegister<15>,
    card_reader_slot: BitRegister<2>,
    card_reader_online: BitRegister<1>,
    card_punch_online: BitRegister<1>,
    card_reader_fault: BitRegister<1>,
    card_punch_fault: BitRegister<1>,
    card_reader_alarm: BitRegister<1>,
    card_punch_alarm: BitRegister<1>,
    priority_alarm: BitRegister<1>,
    typewriter_power: BitRegister<1>,
    n_device: BitRegister<2>,
    paper_tape_reader_running: BitRegister<1>,
    typewriter_keyboard_enabled: BitRegister<1>,
    n_overrun: BitRegister<1>,
    stop_on_parity_alarm: BitRegister<1>,
    control_switches: BitRegister<20>,
    card_reader_queue: VecDeque<CardRecord>,
    card_punch_output: Vec<CardRecord>,
    typewriter_output: Vec<String>,
    paper_tape_input: VecDeque<PaperTapeFrame>,
    paper_tape_output: Vec<i32>,
    typewriter_input: VecDeque<i32>,
    controller_online: [BitRegister<1>; CONTROLLER_COUNT],
    controller_ready: [BitRegister<1>; CONTROLLER_COUNT],
    controller_error: [BitRegister<1>; CONTROLLER_COUNT],
    controller_conditions: [BitRegister<64>; CONTROLLER_COUNT],
    controller_error_conditions: [BitRegister<64>; CONTROLLER_COUNT],
    controller_api_enabled: [BitRegister<1>; CONTROLLER_COUNT],
    controller_commands: Vec<ControllerCommand>,
    controller_selector_busy: BitRegister<1>,
    controller_selector_alarm: BitRegister<1>,
    selected_controller: BitRegister<4>,
    pending_controller_interrupts: BitRegister<8>,
    card_reader_api_enabled: BitRegister<1>,
    card_punch_api_enabled: BitRegister<1>,
    card_reader_interrupt_pending: BitRegister<1>,
    card_punch_interrupt_pending: BitRegister<1>,
    priority_mode: BitRegister<1>,
    priority_return_armed: BitRegister<1>,
    api_branch_inhibit: BitRegister<1>,
    interrupted_x_group: BitRegister<6>,
    automatic_interrupt_mode: BitRegister<1>,
    aau_mode: BitRegister<2>,
    aau_ready: BitRegister<1>,
    aau_ax: BitRegister<40>,
    aau_bx: BitRegister<40>,
    aau_qx: BitRegister<40>,
    aau_ix: BitRegister<40>,
    aau_overflow: BitRegister<1>,
    aau_underflow: BitRegister<1>,
    aau_overflow_hold: BitRegister<1>,
    aau_underflow_hold: BitRegister<1>,
}

impl Ge225GateLevel {
    pub fn new(memory_words: usize) -> Result<Self, Ge225GateError> {
        if !(MIN_MEMORY_WORDS..=MAX_MEMORY_WORDS).contains(&memory_words) {
            return Err(Ge225GateError::InvalidMemorySize {
                words: memory_words,
            });
        }
        Ok(Self {
            memory: (0..memory_words).map(|_| BitRegister::zero()).collect(),
            a: BitRegister::zero(),
            q: BitRegister::zero(),
            m: BitRegister::zero(),
            n: BitRegister::zero(),
            pc: BitRegister::zero(),
            ir: BitRegister::zero(),
            overflow: BitRegister::zero(),
            parity_error: BitRegister::zero(),
            decimal_mode: BitRegister::zero(),
            decimal_carry: BitRegister::zero(),
            clock_sixths: BitRegister::zero(),
            n_ready: BitRegister::new(&[1]),
            selected_x_group: BitRegister::zero(),
            halted: BitRegister::zero(),
            card_reader_continuous: BitRegister::zero(),
            card_reader_base: BitRegister::zero(),
            card_reader_slot: BitRegister::zero(),
            card_reader_online: BitRegister::new(&[1]),
            card_punch_online: BitRegister::new(&[1]),
            card_reader_fault: BitRegister::zero(),
            card_punch_fault: BitRegister::zero(),
            card_reader_alarm: BitRegister::zero(),
            card_punch_alarm: BitRegister::zero(),
            priority_alarm: BitRegister::zero(),
            typewriter_power: BitRegister::zero(),
            n_device: BitRegister::zero(),
            paper_tape_reader_running: BitRegister::zero(),
            typewriter_keyboard_enabled: BitRegister::zero(),
            n_overrun: BitRegister::zero(),
            stop_on_parity_alarm: BitRegister::zero(),
            control_switches: BitRegister::zero(),
            card_reader_queue: VecDeque::new(),
            card_punch_output: Vec::new(),
            typewriter_output: Vec::new(),
            paper_tape_input: VecDeque::new(),
            paper_tape_output: Vec::new(),
            typewriter_input: VecDeque::new(),
            controller_online: std::array::from_fn(|_| BitRegister::new(&[1])),
            controller_ready: std::array::from_fn(|_| BitRegister::new(&[1])),
            controller_error: std::array::from_fn(|_| BitRegister::zero()),
            controller_conditions: std::array::from_fn(|_| {
                BitRegister::new(&u64_to_bits::<64>(1_u64 << CONTROLLER_READY_CONDITION))
            }),
            controller_error_conditions: std::array::from_fn(|_| BitRegister::zero()),
            controller_api_enabled: std::array::from_fn(|_| BitRegister::zero()),
            controller_commands: Vec::new(),
            controller_selector_busy: BitRegister::zero(),
            controller_selector_alarm: BitRegister::zero(),
            selected_controller: BitRegister::zero(),
            pending_controller_interrupts: BitRegister::zero(),
            card_reader_api_enabled: BitRegister::zero(),
            card_punch_api_enabled: BitRegister::zero(),
            card_reader_interrupt_pending: BitRegister::zero(),
            card_punch_interrupt_pending: BitRegister::zero(),
            priority_mode: BitRegister::zero(),
            priority_return_armed: BitRegister::zero(),
            api_branch_inhibit: BitRegister::zero(),
            interrupted_x_group: BitRegister::zero(),
            automatic_interrupt_mode: BitRegister::zero(),
            aau_mode: BitRegister::zero(),
            aau_ready: BitRegister::new(&[1]),
            aau_ax: BitRegister::zero(),
            aau_bx: BitRegister::zero(),
            aau_qx: BitRegister::zero(),
            aau_ix: BitRegister::zero(),
            aau_overflow: BitRegister::zero(),
            aau_underflow: BitRegister::zero(),
            aau_overflow_hold: BitRegister::zero(),
            aau_underflow_hold: BitRegister::zero(),
        })
    }

    pub fn reset(&mut self) {
        for word in &mut self.memory {
            word.write(&[0; 20]);
        }
        self.a.write(&[0; 20]);
        self.q.write(&[0; 20]);
        self.m.write(&[0; 20]);
        self.n.write(&[0; 6]);
        self.pc.write(&[0; 15]);
        self.ir.write(&[0; 20]);
        self.overflow.write(&[0]);
        self.parity_error.write(&[0]);
        self.decimal_mode.write(&[0]);
        self.decimal_carry.write(&[0; 2]);
        self.clock_sixths.write(&[0; 19]);
        self.n_ready.write(&[1]);
        self.selected_x_group.write(&[0; 6]);
        self.halted.write(&[0]);
        self.card_reader_continuous.write(&[0; 2]);
        self.card_reader_base.write(&[0; 15]);
        self.card_reader_slot.write(&[0; 2]);
        self.card_reader_online.write(&[1]);
        self.card_punch_online.write(&[1]);
        self.card_reader_fault.write(&[0]);
        self.card_punch_fault.write(&[0]);
        self.card_reader_alarm.write(&[0]);
        self.card_punch_alarm.write(&[0]);
        self.priority_alarm.write(&[0]);
        self.typewriter_power.write(&[0]);
        self.n_device.write(&[0; 2]);
        self.paper_tape_reader_running.write(&[0]);
        self.typewriter_keyboard_enabled.write(&[0]);
        self.n_overrun.write(&[0]);
        self.stop_on_parity_alarm.write(&[0]);
        self.control_switches.write(&[0; 20]);
        self.card_reader_queue.clear();
        self.card_punch_output.clear();
        self.typewriter_output.clear();
        self.paper_tape_input.clear();
        self.paper_tape_output.clear();
        self.typewriter_input.clear();
        for plug in 0..CONTROLLER_COUNT {
            self.controller_online[plug].write(&[1]);
            self.controller_ready[plug].write(&[1]);
            self.controller_error[plug].write(&[0]);
            self.controller_conditions[plug]
                .write(&u64_to_bits::<64>(1_u64 << CONTROLLER_READY_CONDITION));
            self.controller_error_conditions[plug].write(&[0; 64]);
            self.controller_api_enabled[plug].write(&[0]);
        }
        self.controller_commands.clear();
        self.controller_selector_busy.write(&[0]);
        self.controller_selector_alarm.write(&[0]);
        self.selected_controller.write(&[0; 4]);
        self.pending_controller_interrupts.write(&[0; 8]);
        self.card_reader_api_enabled.write(&[0]);
        self.card_punch_api_enabled.write(&[0]);
        self.card_reader_interrupt_pending.write(&[0]);
        self.card_punch_interrupt_pending.write(&[0]);
        self.priority_mode.write(&[0]);
        self.priority_return_armed.write(&[0]);
        self.api_branch_inhibit.write(&[0]);
        self.interrupted_x_group.write(&[0; 6]);
        self.automatic_interrupt_mode.write(&[0]);
        self.aau_mode.write(&[0; 2]);
        self.aau_ready.write(&[1]);
        self.aau_ax.write(&[0; 40]);
        self.aau_bx.write(&[0; 40]);
        self.aau_qx.write(&[0; 40]);
        self.aau_ix.write(&[0; 40]);
        self.aau_overflow.write(&[0]);
        self.aau_underflow.write(&[0]);
        self.aau_overflow_hold.write(&[0]);
        self.aau_underflow_hold.write(&[0]);
    }

    pub fn load_words(&mut self, words: &[i32], origin: usize) -> Result<(), Ge225GateError> {
        if origin > self.memory.len() {
            return Err(Ge225GateError::InvalidOrigin { origin });
        }
        if words.len() > self.memory.len() - origin {
            return Err(Ge225GateError::ProgramTooLarge {
                words: words.len(),
                capacity: self.memory.len() - origin,
            });
        }
        for (destination, word) in self.memory[origin..].iter_mut().zip(words) {
            destination.write(&i32_to_bits::<20>(*word & WORD_MASK));
        }
        Ok(())
    }

    pub fn read_word(&self, address: i32) -> Result<i32, Ge225GateError> {
        let index = self.checked_address(address)?;
        Ok(bits_to_i32(&self.memory[index].read()))
    }

    pub fn write_word(&mut self, address: i32, word: i32) -> Result<(), Ge225GateError> {
        let index = self.checked_address(address)?;
        self.memory[index].write(&i32_to_bits::<20>(word & WORD_MASK));
        Ok(())
    }

    pub fn queue_card_reader_card(
        &mut self,
        format: CardFormat,
        words: &[i32],
        status: CardStatus,
    ) -> Result<(), Ge225GateError> {
        let reader_before = self.card_reader_ready();
        let punch_before = self.card_punch_ready();
        if words.len() != format.word_count() {
            return Err(Ge225GateError::InvalidCardRecordLength {
                format,
                words: words.len(),
            });
        }
        if self.card_reader_queue.len() >= MAX_CARD_QUEUE_DEPTH {
            return Err(Ge225GateError::CardReaderQueueFull);
        }
        self.card_reader_queue.push_back(CardRecord {
            format,
            words: words.iter().map(|word| word & WORD_MASK).collect(),
            status,
        });
        self.record_direct_ready_transitions(reader_before, punch_before);
        Ok(())
    }

    pub fn queue_card_reader_record(&mut self, words: &[i32]) -> Result<(), Ge225GateError> {
        if words.len() > CardFormat::Decimal.word_count() {
            return Err(Ge225GateError::InvalidCardRecordLength {
                format: CardFormat::Decimal,
                words: words.len(),
            });
        }
        let mut padded = words
            .iter()
            .map(|word| word & WORD_MASK)
            .collect::<Vec<_>>();
        padded.resize(CardFormat::Decimal.word_count(), 0);
        self.queue_card_reader_card(CardFormat::Decimal, &padded, CardStatus::default())
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
        self.card_reader_online.write(&[u8::from(online)]);
        if !online {
            self.card_reader_continuous.write(&[0; 2]);
        }
        self.record_direct_ready_transitions(reader_before, punch_before);
    }

    pub fn set_card_punch_online(&mut self, online: bool) {
        let reader_before = self.card_reader_ready();
        let punch_before = self.card_punch_ready();
        self.card_punch_online.write(&[u8::from(online)]);
        self.record_direct_ready_transitions(reader_before, punch_before);
    }

    pub fn set_card_reader_fault(&mut self, fault: bool) {
        let reader_before = self.card_reader_ready();
        let punch_before = self.card_punch_ready();
        self.card_reader_fault.write(&[u8::from(fault)]);
        if fault {
            self.card_reader_continuous.write(&[0; 2]);
        }
        self.record_direct_ready_transitions(reader_before, punch_before);
    }

    pub fn set_card_punch_fault(&mut self, fault: bool) {
        let reader_before = self.card_reader_ready();
        let punch_before = self.card_punch_ready();
        self.card_punch_fault.write(&[u8::from(fault)]);
        self.record_direct_ready_transitions(reader_before, punch_before);
    }

    pub fn set_stop_on_parity_alarm(&mut self, enabled: bool) {
        self.stop_on_parity_alarm.write(&[u8::from(enabled)]);
    }

    pub fn set_control_switches(&mut self, value: i32) {
        self.control_switches
            .write(&i32_to_bits::<20>(value & WORD_MASK));
    }

    pub fn clear_direct_io_alarms(&mut self) {
        self.card_reader_alarm.write(&[0]);
        self.card_punch_alarm.write(&[0]);
        let selector_alarm = self.controller_selector_alarm.read()[0];
        self.priority_alarm.write(&[selector_alarm]);
        self.parity_error.write(&[0]);
        self.halted.write(&[selector_alarm]);
    }

    pub fn queue_paper_tape_input(&mut self, frames: &[i32]) -> Result<(), Ge225GateError> {
        let frames: Vec<_> = frames
            .iter()
            .map(|data| PaperTapeFrame {
                data: *data,
                parity_error: false,
            })
            .collect();
        self.queue_paper_tape_frames(&frames)
    }

    pub fn queue_paper_tape_frames(
        &mut self,
        frames: &[PaperTapeFrame],
    ) -> Result<(), Ge225GateError> {
        if self.paper_tape_input.len().saturating_add(frames.len()) > MAX_CHARACTER_QUEUE_DEPTH {
            return Err(Ge225GateError::CharacterQueueFull);
        }
        if let Some(frame) = frames
            .iter()
            .find(|frame| !(0..=N_MASK).contains(&frame.data))
        {
            return Err(Ge225GateError::InvalidCharacter { code: frame.data });
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

    pub fn queue_typewriter_input(&mut self, codes: &[i32]) -> Result<(), Ge225GateError> {
        if self.typewriter_input.len().saturating_add(codes.len()) > MAX_CHARACTER_QUEUE_DEPTH {
            return Err(Ge225GateError::CharacterQueueFull);
        }
        if let Some(code) = codes.iter().find(|code| {
            !(0..=N_MASK).contains(code)
                || (typewriter_char(**code).is_none() && !matches!(**code, 0o37 | 0o76))
        }) {
            return Err(Ge225GateError::InvalidCharacter { code: *code });
        }
        self.typewriter_input.extend(codes.iter().copied());
        Ok(())
    }

    pub fn advance_paper_tape_reader(&mut self) -> Result<bool, Ge225GateError> {
        if decode_n_device(self.n_device.read()) != NRegisterDevice::PaperTapeReader
            || self.paper_tape_reader_running.read()[0] == 0
        {
            return Err(Ge225GateError::DeviceNotActive {
                device: "paper-tape reader",
            });
        }
        let Some(frame) = self.paper_tape_input.pop_front() else {
            return Ok(false);
        };
        let overrun = or_gate(self.n_overrun.read()[0], self.n_ready.read()[0]);
        self.n_overrun.write(&[overrun]);
        self.n.write(&i32_to_bits::<6>(frame.data));
        self.parity_error.write(&[or_gate(
            self.parity_error.read()[0],
            u8::from(frame.parity_error),
        )]);
        self.n_ready.write(&[1]);
        if and_gate(
            u8::from(frame.parity_error),
            self.stop_on_parity_alarm.read()[0],
        ) == 1
        {
            self.paper_tape_reader_running.write(&[0]);
            self.priority_alarm.write(&[1]);
            self.halted.write(&[1]);
        }
        Ok(true)
    }

    pub fn advance_typewriter_input(&mut self) -> Result<bool, Ge225GateError> {
        if decode_n_device(self.n_device.read()) != NRegisterDevice::Typewriter
            || self.typewriter_keyboard_enabled.read()[0] == 0
        {
            return Err(Ge225GateError::DeviceNotActive {
                device: "typewriter keyboard",
            });
        }
        let Some(code) = self.typewriter_input.pop_front() else {
            return Ok(false);
        };
        self.n_overrun
            .write(&[or_gate(self.n_overrun.read()[0], self.n_ready.read()[0])]);
        self.n.write(&i32_to_bits::<6>(code));
        self.n_ready.write(&[1]);
        Ok(true)
    }

    pub fn advance_card_reader(&mut self) -> Result<bool, Ge225GateError> {
        let reader_before = self.card_reader_ready();
        let punch_before = self.card_punch_ready();
        let Some(format) = decode_continuous(self.card_reader_continuous.read()) else {
            return Err(Ge225GateError::DeviceNotActive {
                device: "card reader",
            });
        };
        if self.card_reader_queue.is_empty() {
            self.card_reader_continuous.write(&[0; 2]);
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

    pub fn controller_commands(&self) -> &[ControllerCommand] {
        &self.controller_commands
    }

    pub fn take_controller_commands(&mut self) -> Vec<ControllerCommand> {
        std::mem::take(&mut self.controller_commands)
    }

    pub fn highest_priority_pending_controller(&self) -> Option<usize> {
        let pending = self.pending_controller_interrupts.read();
        (0..CONTROLLER_COUNT).find(|plug| pending[*plug] == 1)
    }

    pub fn set_controller_online(
        &mut self,
        plug: usize,
        online: bool,
    ) -> Result<(), Ge225GateError> {
        self.controller_index(plug)?;
        self.controller_online[plug].write(&[u8::from(online)]);
        if !online {
            self.set_controller_ready_value(plug, false);
        }
        Ok(())
    }

    pub fn set_controller_api_enabled(
        &mut self,
        plug: usize,
        enabled: bool,
    ) -> Result<(), Ge225GateError> {
        self.controller_index(plug)?;
        self.controller_api_enabled[plug].write(&[u8::from(enabled)]);
        Ok(())
    }

    pub fn set_controller_condition(
        &mut self,
        plug: usize,
        condition: u8,
        asserted: bool,
    ) -> Result<(), Ge225GateError> {
        self.controller_index(plug)?;
        self.check_controller_condition(condition)?;
        if condition == CONTROLLER_READY_CONDITION {
            return self.set_controller_ready(plug, asserted);
        }
        let mut conditions = self.controller_conditions[plug].read();
        conditions[condition as usize] = u8::from(asserted);
        self.controller_conditions[plug].write(&conditions);
        Ok(())
    }

    pub fn set_controller_error(&mut self, plug: usize, error: bool) -> Result<(), Ge225GateError> {
        self.controller_index(plug)?;
        self.controller_error[plug].write(&[u8::from(error)]);
        if !error {
            let errors = self.controller_error_conditions[plug].read();
            let conditions = self.controller_conditions[plug].read();
            self.controller_conditions[plug].write(&std::array::from_fn(|bit| {
                and_gate(conditions[bit], not_gate(errors[bit]))
            }));
            self.controller_error_conditions[plug].write(&[0; 64]);
        }
        Ok(())
    }

    pub fn set_controller_error_condition(
        &mut self,
        plug: usize,
        condition: u8,
        asserted: bool,
    ) -> Result<(), Ge225GateError> {
        self.controller_index(plug)?;
        self.check_controller_condition(condition)?;
        if condition == CONTROLLER_READY_CONDITION {
            return Err(Ge225GateError::ControllerReadyCannotBeError);
        }
        let mut conditions = self.controller_conditions[plug].read();
        let mut errors = self.controller_error_conditions[plug].read();
        conditions[condition as usize] = u8::from(asserted);
        errors[condition as usize] = u8::from(asserted);
        self.controller_conditions[plug].write(&conditions);
        self.controller_error_conditions[plug].write(&errors);
        self.controller_error[plug].write(&[not_gate(is_zero(&errors))]);
        Ok(())
    }

    pub fn set_controller_ready(&mut self, plug: usize, ready: bool) -> Result<(), Ge225GateError> {
        self.controller_index(plug)?;
        let transitioned = and_gate(
            not_gate(self.controller_ready[plug].read()[0]),
            u8::from(ready),
        );
        self.set_controller_ready_value(plug, ready);
        if and_gate(transitioned, self.controller_api_enabled[plug].read()[0]) == 1 {
            let mut pending = self.pending_controller_interrupts.read();
            pending[plug] = 1;
            self.pending_controller_interrupts.write(&pending);
        }
        Ok(())
    }

    pub fn complete_controller(
        &mut self,
        plug: usize,
        conditions: u64,
        error: bool,
    ) -> Result<(), Ge225GateError> {
        self.controller_index(plug)?;
        if self.controller_online[plug].read()[0] == 0 {
            return Err(Ge225GateError::ControllerOffline { plug });
        }
        self.controller_conditions[plug].write(&u64_to_bits::<64>(conditions));
        self.controller_error[plug].write(&[u8::from(error)]);
        self.controller_error_conditions[plug].write(&[0; 64]);
        self.set_controller_ready(plug, true)
    }

    pub fn advance_controller_selector(&mut self) -> bool {
        if self.controller_selector_busy.read()[0] == 0 {
            return false;
        }
        self.controller_selector_busy.write(&[0]);
        self.selected_controller.write(&[0; 4]);
        true
    }

    pub fn set_card_reader_api_enabled(&mut self, enabled: bool) {
        self.card_reader_api_enabled.write(&[u8::from(enabled)]);
    }

    pub fn set_card_punch_api_enabled(&mut self, enabled: bool) {
        self.card_punch_api_enabled.write(&[u8::from(enabled)]);
    }

    pub fn clear_controller_selector_alarm(&mut self) {
        self.controller_selector_alarm.write(&[0]);
        let direct_alarm = or_gate(
            self.card_reader_alarm.read()[0],
            self.card_punch_alarm.read()[0],
        );
        self.priority_alarm.write(&[direct_alarm]);
        self.halted.write(&[direct_alarm]);
    }

    pub fn set_aau_ready(&mut self, ready: bool) {
        self.aau_ready.write(&[u8::from(ready)]);
    }

    pub fn clear_aau_alerts(&mut self) {
        self.aau_overflow.write(&[0]);
        self.aau_underflow.write(&[0]);
        self.aau_overflow_hold.write(&[0]);
        self.aau_underflow_hold.write(&[0]);
    }

    pub fn set_program_counter(&mut self, address: i32) -> Result<(), Ge225GateError> {
        self.checked_address(address)?;
        self.pc.write(&i32_to_bits::<15>(address));
        Ok(())
    }

    pub fn step(&mut self) -> Result<StepTrace, Ge225GateError> {
        if self.halted.read()[0] == 1 {
            return Err(Ge225GateError::Halted);
        }
        if self.api_branch_inhibit.read()[0] == 1 {
            self.api_branch_inhibit.write(&[0]);
        } else {
            self.enter_api_interrupt_if_pending()?;
        }
        let reader_ready_before = self.card_reader_ready();
        let punch_ready_before = self.card_punch_ready();
        let pc_before = bits_to_i32(&self.pc.read());
        let instruction = self.read_word(pc_before)?;
        let (mut operation, mut modifier, mut address) =
            decode(instruction, decode_n_device(self.n_device.read())).ok_or(
                Ge225GateError::UnknownInstruction {
                    word: instruction,
                    pc: pc_before,
                },
            )?;
        let mut ir_word = instruction;
        if is_fixed(operation) && modifier != 0 {
            if operation == Operation::Sxg {
                return Err(Ge225GateError::InvalidAutomaticModification { word: instruction });
            }
            let increment = self.x_word(modifier)? & ADDRESS_MASK;
            let modified_operand = (instruction + increment) & ADDRESS_MASK;
            ir_word = (instruction & !ADDRESS_MASK) | modified_operand;
            if is_shift(operation) {
                let modified_count = address + increment;
                if modified_count > 31 {
                    return Err(Ge225GateError::ShiftCountOutOfRange {
                        count: modified_count,
                    });
                }
                address = modified_count;
                modifier = 0;
            } else {
                let modified_word = (0o25 << 15) | modified_operand;
                let (modified_operation, modified_modifier, modified_address) =
                    decode(modified_word, decode_n_device(self.n_device.read()))
                        .filter(|(candidate, _, _)| is_fixed(*candidate))
                        .ok_or(Ge225GateError::InvalidAutomaticModification {
                            word: modified_word,
                        })?;
                operation = modified_operation;
                modifier = modified_modifier;
                address = modified_address;
            }
        }
        let sequential = pc_before + 1;
        if !matches!(operation, Operation::Bru | Operation::Spb) {
            self.checked_address(sequential)?;
        }

        let effective_address = if is_memory_reference(operation) {
            let effective = if operation == Operation::Bru && modifier == 0 {
                let target = (sequential & !ADDRESS_MASK) | address;
                self.checked_address(target)?;
                target
            } else {
                self.effective_address(address, modifier)?
            };
            Some(effective)
        } else {
            None
        };
        if matches!(
            operation,
            Operation::Dld | Operation::Dad | Operation::Dsu | Operation::Dst | Operation::Dcb
        ) {
            let pair_address = effective_address.ok_or(Ge225GateError::UnknownInstruction {
                word: instruction,
                pc: pc_before,
            })?;
            if pair_address & 1 == 0 {
                self.following_address(pair_address)?;
            }
        }
        if matches!(operation, Operation::Ldx | Operation::Stx) {
            self.checked_address(address)?;
        }
        if matches!(
            operation,
            Operation::Bxl
                | Operation::Bxh
                | Operation::Ldx
                | Operation::Spb
                | Operation::Inx
                | Operation::Stx
        ) {
            self.x_address(modifier)?;
        }
        if operation == Operation::Spb {
            self.checked_address((pc_before & !ADDRESS_MASK) | address)?;
        }
        if operation == Operation::Mov {
            let word_count = self.mov_word_count();
            self.checked_range(address, word_count)?;
            self.checked_range(bits_to_i32(&self.a.read()) & X_MASK, word_count)?;
            self.x_address(0)?;
        }
        let skip = self.skip_amount(operation, modifier, effective_address, address)?;
        if skip != 0 {
            self.checked_address(sequential + skip)?;
        }
        self.preflight_decimal(operation, effective_address)?;
        self.preflight_direct_io(operation, effective_address)?;
        self.preflight_controller(operation, address, sequential)?;
        self.preflight_aau(operation, modifier, address, effective_address)?;
        if ir_word == instruction && modifier != 0 {
            if let Some(modified) = effective_address.map(|effective| {
                if is_card_operation(operation) {
                    (instruction & !ADDRESS_MASK)
                        | (effective & ADDRESS_MASK & !(CARD_ADDRESS_ALIGNMENT - 1))
                        | (instruction & 0o17)
                } else {
                    (instruction & !ADDRESS_MASK) | (effective & ADDRESS_MASK)
                }
            }) {
                ir_word = modified;
            }
        }
        self.ir.write(&i32_to_bits::<20>(ir_word));
        self.pc.write(&i32_to_bits::<15>(sequential));
        let a_before = bits_to_i32(&self.a.read());
        let q_before = bits_to_i32(&self.q.read());
        let mnemonic = if operation == Operation::NCommand {
            match decode_n_device(self.n_device.read()) {
                NRegisterDevice::Off => "NIO",
                NRegisterDevice::Typewriter => "TYP",
                NRegisterDevice::PaperTapeReader => "RPT",
                NRegisterDevice::PaperTapePunch => "WPT",
            }
        } else {
            operation_name(operation)
        }
        .to_string();
        let priority_return = and_gate(
            self.priority_mode.read()[0],
            and_gate(
                self.priority_return_armed.read()[0],
                u8::from(operation == Operation::Bru && modifier != 0),
            ),
        );
        self.execute(operation, modifier, effective_address, address, pc_before)?;
        if priority_return == 1 {
            self.priority_mode.write(&[0]);
            self.priority_return_armed.write(&[0]);
            self.selected_x_group
                .write(&self.interrupted_x_group.read());
        }
        if operation == Operation::Bru {
            self.api_branch_inhibit.write(&[1]);
        }
        self.record_direct_ready_transitions(reader_ready_before, punch_ready_before);
        self.checked_address(bits_to_i32(&self.pc.read()))?;

        Ok(StepTrace {
            pc_before,
            pc_after: bits_to_i32(&self.pc.read()),
            instruction,
            mnemonic,
            a_before,
            a_after: bits_to_i32(&self.a.read()),
            q_before,
            q_after: bits_to_i32(&self.q.read()),
            effective_address,
        })
    }

    pub fn run(&mut self, max_steps: usize) -> Result<Vec<StepTrace>, Ge225GateError> {
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted.read()[0] == 1 {
                break;
            }
            traces.push(self.step()?);
        }
        Ok(traces)
    }

    pub fn get_state(&self) -> Ge225GateState {
        Ge225GateState {
            a: bits_to_i32(&self.a.read()),
            q: bits_to_i32(&self.q.read()),
            m: bits_to_i32(&self.m.read()),
            n: bits_to_i32(&self.n.read()),
            pc: bits_to_i32(&self.pc.read()),
            ir: bits_to_i32(&self.ir.read()),
            overflow: self.overflow.read()[0] == 1,
            parity_error: self.parity_error.read()[0] == 1,
            decimal_mode: self.decimal_mode.read()[0] == 1,
            decimal_carry: decode_decimal_carry(self.decimal_carry.read()),
            clock_sixths: bits_to_i32(&self.clock_sixths.read()),
            n_ready: self.n_ready.read()[0] == 1,
            selected_x_group: bits_to_i32(&self.selected_x_group.read()) as usize,
            halted: self.halted.read()[0] == 1,
            card_reader_ready: self.card_reader_ready() == 1,
            card_punch_ready: self.card_punch_ready() == 1,
            card_reader_continuous: decode_continuous(self.card_reader_continuous.read()),
            card_reader_base: bits_to_i32(&self.card_reader_base.read()),
            card_reader_slot: bits_to_i32(&self.card_reader_slot.read()) as usize,
            card_reader_online: self.card_reader_online.read()[0] == 1,
            card_punch_online: self.card_punch_online.read()[0] == 1,
            card_reader_fault: self.card_reader_fault.read()[0] == 1,
            card_punch_fault: self.card_punch_fault.read()[0] == 1,
            card_reader_alarm: self.card_reader_alarm.read()[0] == 1,
            card_punch_alarm: self.card_punch_alarm.read()[0] == 1,
            priority_alarm: self.priority_alarm.read()[0] == 1,
            n_device: decode_n_device(self.n_device.read()),
            typewriter_power: self.typewriter_power.read()[0] == 1,
            paper_tape_reader_running: self.paper_tape_reader_running.read()[0] == 1,
            typewriter_keyboard_enabled: self.typewriter_keyboard_enabled.read()[0] == 1,
            n_overrun: self.n_overrun.read()[0] == 1,
            stop_on_parity_alarm: self.stop_on_parity_alarm.read()[0] == 1,
            control_switches: bits_to_i32(&self.control_switches.read()),
            automatic_interrupt_mode: self.automatic_interrupt_mode.read()[0] == 1,
            priority_mode: self.priority_mode.read()[0] == 1,
            priority_return_armed: self.priority_return_armed.read()[0] == 1,
            pending_controller_interrupts: bits_to_i32(&self.pending_controller_interrupts.read())
                as u8,
            card_reader_api_enabled: self.card_reader_api_enabled.read()[0] == 1,
            card_punch_api_enabled: self.card_punch_api_enabled.read()[0] == 1,
            card_reader_interrupt_pending: self.card_reader_interrupt_pending.read()[0] == 1,
            card_punch_interrupt_pending: self.card_punch_interrupt_pending.read()[0] == 1,
            controller_selector_busy: self.controller_selector_busy.read()[0] == 1,
            controller_selector_alarm: self.controller_selector_alarm.read()[0] == 1,
            selected_controller: decode_selected_controller(self.selected_controller.read()),
            controllers: (0..CONTROLLER_COUNT)
                .map(|plug| ControllerStatus {
                    online: self.controller_online[plug].read()[0] == 1,
                    ready: self.controller_ready[plug].read()[0] == 1,
                    error: self.controller_error[plug].read()[0] == 1,
                    conditions: bits_to_u64(&self.controller_conditions[plug].read()),
                    error_conditions: bits_to_u64(&self.controller_error_conditions[plug].read()),
                    api_enabled: self.controller_api_enabled[plug].read()[0] == 1,
                })
                .collect(),
            aau: AauState {
                mode: decode_aau_mode(self.aau_mode.read()),
                ready: self.aau_ready.read()[0] == 1,
                ax: bits_to_u64(&self.aau_ax.read()),
                bx: bits_to_u64(&self.aau_bx.read()),
                qx: bits_to_u64(&self.aau_qx.read()),
                ix: bits_to_u64(&self.aau_ix.read()),
                overflow: self.aau_overflow.read()[0] == 1,
                underflow: self.aau_underflow.read()[0] == 1,
                overflow_hold: self.aau_overflow_hold.read()[0] == 1,
                underflow_hold: self.aau_underflow_hold.read()[0] == 1,
            },
            memory: self
                .memory
                .iter()
                .map(|word| bits_to_i32(&word.read()))
                .collect(),
        }
    }

    pub fn flip_flop_count(&self) -> usize {
        self.memory.len() * 20 + CENTRAL_FLIP_FLOPS
    }

    pub fn set_clock_sixths(&mut self, value: i32) -> Result<(), Ge225GateError> {
        if !(0..=DATA_MASK).contains(&value) {
            return Err(Ge225GateError::InvalidClock { value });
        }
        self.clock_sixths.write(&i32_to_bits::<19>(value));
        Ok(())
    }

    pub fn advance_clock_sixths(&mut self, ticks: u64) {
        let current = zero_extend::<19, 65>(self.clock_sixths.read());
        let ticks = u64_to_bits::<65>(ticks);
        let day = zero_extend::<20, 65>(i32_to_bits::<20>(CLOCK_DAY_SIXTHS));
        let word_modulus = zero_extend::<20, 65>(i32_to_bits::<20>(1 << 19));

        let normal_sum = gate_add(current, gate_divide_constant(ticks, CLOCK_DAY_SIXTHS).1).0;
        let normal = mux_bits(
            greater_or_equal(&normal_sum, &day),
            normal_sum,
            gate_subtract(normal_sum, day).0,
        );

        let until_word_wrap = gate_subtract(word_modulus, current).0;
        let before_word_wrap = gate_add(current, ticks).0;
        let after_word_wrap =
            gate_divide_constant(gate_subtract(ticks, until_word_wrap).0, CLOCK_DAY_SIXTHS).1;
        let exceptional = mux_bits(
            not_gate(greater_or_equal(&ticks, &until_word_wrap)),
            after_word_wrap,
            before_word_wrap,
        );
        let next = mux_bits(
            not_gate(greater_or_equal(&current, &day)),
            exceptional,
            normal,
        );
        let clock: [u8; 19] = next[..19]
            .try_into()
            .expect("the reduced GE-225 clock fits nineteen bits");
        self.clock_sixths.write(&clock);
    }

    pub fn clear_decimal_carry(&mut self) {
        self.decimal_carry.write(&[0; 2]);
    }

    fn preflight_decimal(
        &self,
        operation: Operation,
        effective_address: Option<i32>,
    ) -> Result<(), Ge225GateError> {
        if self.decimal_mode.read()[0] == 0 {
            return Ok(());
        }
        let carry = self.decimal_carry.read();
        match operation {
            Operation::Add | Operation::Sub => {
                let operand = self.read_word(effective_address.expect("memory operation"))?;
                gate_decimal_word(
                    self.a.read(),
                    i32_to_bits(operand),
                    operation == Operation::Sub,
                    carry,
                )?;
            }
            Operation::Dad | Operation::Dsu => {
                let address = effective_address.expect("memory operation");
                let first = self.read_word(address)?;
                let second = if address & 1 == 0 {
                    self.read_word(self.following_address(address)?)?
                } else {
                    first
                };
                gate_decimal_pair(
                    self.a.read(),
                    self.q.read(),
                    i32_to_bits(first),
                    i32_to_bits(second),
                    operation == Operation::Dsu,
                    carry,
                )?;
            }
            Operation::Ado | Operation::Sbo => {
                gate_decimal_word(
                    self.a.read(),
                    decimal_one_bits(),
                    operation == Operation::Sbo,
                    carry,
                )?;
            }
            _ => {}
        }
        Ok(())
    }

    fn card_reader_ready(&self) -> u8 {
        let online = self.card_reader_online.read()[0];
        let healthy = not_gate(self.card_reader_fault.read()[0]);
        let idle = is_zero(&self.card_reader_continuous.read());
        let queued = u8::from(!self.card_reader_queue.is_empty());
        and_gate(and_gate(online, healthy), and_gate(idle, queued))
    }

    fn controller_index(&self, plug: usize) -> Result<usize, Ge225GateError> {
        (plug < CONTROLLER_COUNT)
            .then_some(plug)
            .ok_or(Ge225GateError::ControllerPlugOutOfRange { plug })
    }

    fn check_controller_condition(&self, condition: u8) -> Result<(), Ge225GateError> {
        if !(CONTROLLER_CONDITION_MIN..=CONTROLLER_CONDITION_MAX).contains(&condition) {
            return Err(Ge225GateError::ControllerConditionOutOfRange { condition });
        }
        Ok(())
    }

    fn set_controller_ready_value(&mut self, plug: usize, ready: bool) {
        self.controller_ready[plug].write(&[u8::from(ready)]);
        let mut conditions = self.controller_conditions[plug].read();
        conditions[CONTROLLER_READY_CONDITION as usize] = u8::from(ready);
        self.controller_conditions[plug].write(&conditions);
    }

    fn api_interrupt_pending(&self) -> u8 {
        let controller = not_gate(is_zero(&self.pending_controller_interrupts.read()));
        or_gate(
            controller,
            or_gate(
                self.card_reader_interrupt_pending.read()[0],
                self.card_punch_interrupt_pending.read()[0],
            ),
        )
    }

    fn enter_api_interrupt_if_pending(&mut self) -> Result<(), Ge225GateError> {
        let enter = and_gate(
            self.automatic_interrupt_mode.read()[0],
            and_gate(
                not_gate(self.priority_mode.read()[0]),
                self.api_interrupt_pending(),
            ),
        );
        if enter == 0 {
            return Ok(());
        }
        self.write_word(API_SAVED_PC_ADDRESS, bits_to_i32(&self.pc.read()))?;
        self.checked_address(API_VECTOR_ADDRESS)?;
        self.interrupted_x_group
            .write(&self.selected_x_group.read());
        self.selected_x_group
            .write(&i32_to_bits::<6>(API_X_GROUP as i32));
        self.pc.write(&i32_to_bits::<15>(API_VECTOR_ADDRESS));
        self.automatic_interrupt_mode.write(&[0]);
        self.priority_mode.write(&[1]);
        self.priority_return_armed.write(&[0]);
        self.pending_controller_interrupts.write(&[0; 8]);
        self.card_reader_interrupt_pending.write(&[0]);
        self.card_punch_interrupt_pending.write(&[0]);
        Ok(())
    }

    fn record_direct_ready_transitions(&mut self, reader_before: u8, punch_before: u8) {
        let reader_transition = and_gate(not_gate(reader_before), self.card_reader_ready());
        if and_gate(reader_transition, self.card_reader_api_enabled.read()[0]) == 1 {
            self.card_reader_interrupt_pending.write(&[1]);
        }
        let punch_transition = and_gate(not_gate(punch_before), self.card_punch_ready());
        if and_gate(punch_transition, self.card_punch_api_enabled.read()[0]) == 1 {
            self.card_punch_interrupt_pending.write(&[1]);
        }
    }

    fn card_punch_ready(&self) -> u8 {
        and_gate(
            and_gate(
                self.card_punch_online.read()[0],
                not_gate(self.card_punch_fault.read()[0]),
            ),
            u8::from(self.card_punch_output.len() < MAX_CARD_PUNCH_DEPTH),
        )
    }

    fn card_address(&self, address: i32) -> Result<i32, Ge225GateError> {
        if !(0..CARD_ADDRESS_LIMIT).contains(&address) || address % CARD_ADDRESS_ALIGNMENT != 0 {
            return Err(Ge225GateError::InvalidCardAddress { address });
        }
        Ok(address)
    }

    fn preflight_direct_io(
        &self,
        operation: Operation,
        effective_address: Option<i32>,
    ) -> Result<(), Ge225GateError> {
        let format = card_operation_format(operation);
        if is_card_operation(operation) {
            let base =
                self.card_address(effective_address.expect("card operations have an address"))?;
            let words = format.unwrap_or(CardFormat::Full12).word_count();
            self.checked_range(base, words)?;
            if is_card_read(operation) {
                let sync_offset = match format {
                    Some(CardFormat::Decimal) => 27,
                    Some(CardFormat::Binary10) => 41,
                    _ => 83,
                };
                self.checked_address(base + sync_offset)?;
            }
        }
        if operation == Operation::NCommand {
            match decode_n_device(self.n_device.read()) {
                NRegisterDevice::Typewriter
                    if self.typewriter_output.len() >= MAX_CHARACTER_QUEUE_DEPTH =>
                {
                    return Err(Ge225GateError::CharacterQueueFull);
                }
                NRegisterDevice::PaperTapePunch
                    if self.paper_tape_output.len() >= MAX_CHARACTER_QUEUE_DEPTH =>
                {
                    return Err(Ge225GateError::CharacterQueueFull);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn preflight_controller(
        &self,
        operation: Operation,
        raw_address: i32,
        sequential: i32,
    ) -> Result<(), Ge225GateError> {
        if operation != Operation::Sel {
            return Ok(());
        }
        let plug = raw_address as usize;
        self.controller_index(plug)?;
        if self.controller_selector_busy.read()[0] == 1
            || self.controller_online[plug].read()[0] == 0
        {
            return Ok(());
        }
        self.checked_range(sequential, 2)?;
        self.checked_address(sequential + 2)?;
        if self.controller_commands.len() >= MAX_CONTROLLER_COMMANDS {
            return Err(Ge225GateError::ControllerCommandQueueFull);
        }
        Ok(())
    }

    fn preflight_aau(
        &self,
        operation: Operation,
        modifier: i32,
        raw_address: i32,
        effective_address: Option<i32>,
    ) -> Result<(), Ge225GateError> {
        if (is_aau_memory(operation) || is_aau_general(operation)) && self.aau_ready.read()[0] == 0
        {
            return Err(Ge225GateError::AauNotReady {
                instruction: operation_name(operation),
            });
        }
        if !is_aau_memory(operation) {
            return Ok(());
        }
        if modifier == 0 && raw_address <= 0o17 {
            return Err(Ge225GateError::InvalidAauAddress {
                instruction: operation_name(operation),
                address: raw_address,
            });
        }
        if matches!(
            operation,
            Operation::Fad | Operation::Fsu | Operation::Fmp | Operation::Fdv
        ) && decode_aau_mode(self.aau_mode.read()).is_none()
        {
            return Err(Ge225GateError::AauModeRequired {
                instruction: operation_name(operation),
            });
        }
        let address = effective_address.expect("AAU memory operations have an effective address");
        if address & 1 == 0 {
            self.following_address(address)?;
        }
        Ok(())
    }

    fn reader_alarm(&mut self) {
        self.card_reader_alarm.write(&[1]);
        self.priority_alarm.write(&[1]);
        self.card_reader_continuous.write(&[0; 2]);
        self.halted.write(&[1]);
    }

    fn punch_alarm(&mut self) {
        self.card_punch_alarm.write(&[1]);
        self.priority_alarm.write(&[1]);
        self.halted.write(&[1]);
    }

    fn transfer_card_input(&mut self, expected: CardFormat) -> Result<(), Ge225GateError> {
        let Some(record) = self.card_reader_queue.front().cloned() else {
            self.card_reader_continuous.write(&[0; 2]);
            return Ok(());
        };
        if record.format != expected {
            self.reader_alarm();
            return Ok(());
        }
        let slot = bits_to_i32(&self.card_reader_slot.read()) as usize;
        let offset = match expected {
            CardFormat::Decimal => slot * 32,
            CardFormat::Binary10 => slot * 64,
            CardFormat::Full12 | CardFormat::MixedDecimal | CardFormat::MixedBinary => 0,
        };
        let destination = bits_to_i32(&self.card_reader_base.read()) + offset as i32;
        let data_range = self.checked_range(destination, record.words.len())?;
        let sync_offset = match expected {
            CardFormat::Decimal => 27,
            CardFormat::Binary10 => 41,
            CardFormat::Full12 | CardFormat::MixedDecimal | CardFormat::MixedBinary => 83,
        };
        let sync_address = self.checked_address(destination + sync_offset)?;
        let hopper_empty = self.card_reader_queue.len() == 1;
        let sync = card_sync_word(record.format, record.status, hopper_empty);

        self.card_reader_queue.pop_front();
        for (address, mut word) in data_range.zip(record.words) {
            if record.format == CardFormat::MixedBinary && address == destination as usize {
                word |= SIGN_BIT;
            }
            self.memory[address].write(&i32_to_bits::<20>(word & WORD_MASK));
        }
        self.memory[sync_address].write(&i32_to_bits::<20>(sync));
        let next_slot = match expected {
            CardFormat::Decimal => (slot + 1) % 4,
            CardFormat::Binary10 => (slot + 1) % 2,
            CardFormat::Full12 | CardFormat::MixedDecimal | CardFormat::MixedBinary => 0,
        };
        self.card_reader_slot
            .write(&i32_to_bits::<2>(next_slot as i32));
        if self.card_reader_queue.is_empty() {
            self.card_reader_continuous.write(&[0; 2]);
        }
        Ok(())
    }

    fn transfer_card_punch(&mut self, format: CardFormat, base: i32) -> Result<(), Ge225GateError> {
        let words = self
            .checked_range(base, format.word_count())?
            .map(|address| bits_to_i32(&self.memory[address].read()))
            .collect();
        self.card_punch_output.push(CardRecord {
            format,
            words,
            status: CardStatus::default(),
        });
        Ok(())
    }

    fn checked_address(&self, address: i32) -> Result<usize, Ge225GateError> {
        if address < 0 || address as usize >= self.memory.len() {
            return Err(Ge225GateError::AddressOutOfRange {
                address,
                capacity: self.memory.len(),
            });
        }
        Ok(address as usize)
    }

    fn effective_address(&self, address: i32, modifier: i32) -> Result<i32, Ge225GateError> {
        let effective = if modifier == 0 {
            address
        } else {
            let group = bits_to_i32(&self.selected_x_group.read()) as usize;
            let x_address = (group * 4 + modifier as usize) as i32;
            (address + (self.read_word(x_address)? & X_MASK)) & X_MASK
        };
        self.checked_address(effective)?;
        Ok(effective)
    }

    fn x_address(&self, modifier: i32) -> Result<i32, Ge225GateError> {
        let group = bits_to_i32(&self.selected_x_group.read());
        let address = group * 4 + modifier;
        self.checked_address(address)?;
        Ok(address)
    }

    fn x_word(&self, modifier: i32) -> Result<i32, Ge225GateError> {
        self.read_word(self.x_address(modifier)?)
    }

    fn set_x_word(&mut self, modifier: i32, value: i32) -> Result<(), Ge225GateError> {
        self.write_word(self.x_address(modifier)?, value)
    }

    fn following_address(&self, address: i32) -> Result<i32, Ge225GateError> {
        let following = address + 1;
        self.checked_address(following)?;
        Ok(following)
    }

    fn checked_range(
        &self,
        start: i32,
        word_count: usize,
    ) -> Result<std::ops::Range<usize>, Ge225GateError> {
        if start < 0 {
            return Err(Ge225GateError::AddressOutOfRange {
                address: start,
                capacity: self.memory.len(),
            });
        }
        let start = start as usize;
        let end = start
            .checked_add(word_count)
            .ok_or(Ge225GateError::AddressOutOfRange {
                address: i32::MAX,
                capacity: self.memory.len(),
            })?;
        if end > self.memory.len() {
            return Err(Ge225GateError::AddressOutOfRange {
                address: i32::try_from(end).unwrap_or(i32::MAX),
                capacity: self.memory.len(),
            });
        }
        Ok(start..end)
    }

    fn mov_word_count(&self) -> usize {
        let q = self.q.read();
        if q[19] == 0 {
            0
        } else {
            bits_to_i32(&gate_absolute(q)) as usize
        }
    }

    fn skip_amount(
        &self,
        operation: Operation,
        modifier: i32,
        effective_address: Option<i32>,
        raw_address: i32,
    ) -> Result<i32, Ge225GateError> {
        let a = self.a.read();
        Ok(match operation {
            Operation::Bxl => i32::from((self.x_word(modifier)? & ADDRESS_MASK) >= raw_address),
            Operation::Bxh => i32::from((self.x_word(modifier)? & ADDRESS_MASK) < raw_address),
            Operation::Cab => {
                let Some(address) = effective_address else {
                    return Ok(0);
                };
                let operand = i32_to_bits::<20>(self.read_word(address)?);
                match signed_compare(operand, a) {
                    0 => 1,
                    ordering if ordering < 0 => 2,
                    _ => 0,
                }
            }
            Operation::Dcb => {
                let Some(address) = effective_address else {
                    return Ok(0);
                };
                let first = self.read_word(address)?;
                let second = if address & 1 == 0 {
                    self.read_word(self.following_address(address)?)?
                } else {
                    first
                };
                let operand = join_double(i32_to_bits::<20>(first), i32_to_bits::<20>(second));
                match signed_compare(operand, join_double(a, self.q.read())) {
                    0 => 1,
                    ordering if ordering < 0 => 2,
                    _ => 0,
                }
            }
            Operation::Bod
            | Operation::Bev
            | Operation::Bmi
            | Operation::Bpl
            | Operation::Bze
            | Operation::Bnz
            | Operation::Bov
            | Operation::Bno => {
                let zero = is_zero(&a);
                let condition = match operation {
                    Operation::Bod => a[0],
                    Operation::Bev => not_gate(a[0]),
                    Operation::Bmi => a[19],
                    Operation::Bpl => not_gate(a[19]),
                    Operation::Bze => zero,
                    Operation::Bnz => not_gate(zero),
                    Operation::Bov => self.overflow.read()[0],
                    Operation::Bno => not_gate(self.overflow.read()[0]),
                    _ => unreachable!("the match only contains fixed branch tests"),
                };
                i32::from(condition == 0)
            }
            Operation::Bpe
            | Operation::Bpc
            | Operation::Bnr
            | Operation::Bnn
            | Operation::Bcr
            | Operation::Bcn
            | Operation::Bpr
            | Operation::Bpn => {
                let condition = match operation {
                    Operation::Bpe => self.parity_error.read()[0],
                    Operation::Bpc => not_gate(self.parity_error.read()[0]),
                    Operation::Bnr => self.n_ready.read()[0],
                    Operation::Bnn => not_gate(self.n_ready.read()[0]),
                    Operation::Bcr => self.card_reader_ready(),
                    Operation::Bcn => not_gate(self.card_reader_ready()),
                    Operation::Bpr => self.card_punch_ready(),
                    Operation::Bpn => not_gate(self.card_punch_ready()),
                    _ => unreachable!("the match contains only direct-I/O branch tests"),
                };
                i32::from(condition == 0)
            }
            Operation::BcsSet | Operation::BcsClear => {
                let plug = ((raw_address >> 6) & 0o7) as usize;
                let condition = (raw_address & CONTROLLER_CONDITION_MASK) as usize;
                let asserted = self.controller_conditions[plug].read()[condition];
                let branch = if operation == Operation::BcsSet {
                    asserted
                } else {
                    not_gate(asserted)
                };
                i32::from(branch == 0)
            }
            operation if is_aau_branch(operation) => {
                i32::from(self.aau_branch_condition(operation) == 0)
            }
            _ => 0,
        })
    }

    fn execute(
        &mut self,
        operation: Operation,
        modifier: i32,
        effective_address: Option<i32>,
        raw_address: i32,
        pc_before: i32,
    ) -> Result<(), Ge225GateError> {
        let address = effective_address.unwrap_or(raw_address);
        match operation {
            operation if is_aau_memory(operation) => {
                self.execute_aau_memory(operation, address)?;
            }
            operation if is_aau_general(operation) => {
                self.execute_aau_general(operation)?;
            }
            operation if is_aau_branch(operation) => {
                self.execute_aau_branch(operation)?;
            }
            Operation::Lda => {
                let operand = self.read_word(address)?;
                self.m.write(&i32_to_bits::<20>(operand));
                self.a.write(&i32_to_bits::<20>(operand));
            }
            Operation::Add | Operation::Sub => {
                let operand = self.read_word(address)?;
                let left = self.a.read();
                let right = i32_to_bits::<20>(operand);
                let (result, carry, overflow) = if self.decimal_mode.read()[0] == 1 {
                    let (result, carry, overflow) = gate_decimal_word(
                        left,
                        right,
                        operation == Operation::Sub,
                        self.decimal_carry.read(),
                    )?;
                    (result, Some(carry), overflow)
                } else if operation == Operation::Add {
                    let (result, overflow) = gate_add(left, right);
                    (result, None, overflow)
                } else {
                    let (result, overflow) = gate_subtract(left, right);
                    (result, None, overflow)
                };
                self.m.write(&right);
                self.a.write(&result);
                if let Some(carry) = carry {
                    self.decimal_carry.write(&carry);
                }
                if overflow == 1 {
                    self.overflow.write(&[1]);
                }
            }
            Operation::Sta => {
                let value = self.a.read();
                let index = self.checked_address(address)?;
                self.memory[index].write(&value);
            }
            Operation::Bxl => {
                if (self.x_word(modifier)? & ADDRESS_MASK) >= raw_address {
                    self.advance_pc(1)?;
                }
            }
            Operation::Bxh => {
                if (self.x_word(modifier)? & ADDRESS_MASK) < raw_address {
                    self.advance_pc(1)?;
                }
            }
            Operation::Ldx => {
                let word = self.read_word(raw_address)?;
                self.set_x_word(modifier, word)?;
            }
            Operation::Spb => {
                let target = (pc_before & !ADDRESS_MASK) | raw_address;
                self.checked_address(target)?;
                self.set_x_word(modifier, pc_before)?;
                self.pc.write(&i32_to_bits::<15>(target));
            }
            Operation::Dld => {
                let first = self.read_word(address)?;
                let second = if address & 1 == 0 {
                    self.read_word(self.following_address(address)?)?
                } else {
                    first
                };
                self.a.write(&i32_to_bits::<20>(first));
                self.q.write(&i32_to_bits::<20>(second));
            }
            Operation::Dad | Operation::Dsu => {
                let first = self.read_word(address)?;
                let second = if address & 1 == 0 {
                    self.read_word(self.following_address(address)?)?
                } else {
                    first
                };
                if self.decimal_mode.read()[0] == 1 {
                    let (a, q, carry, overflow) = gate_decimal_pair(
                        self.a.read(),
                        self.q.read(),
                        i32_to_bits(first),
                        i32_to_bits(second),
                        operation == Operation::Dsu,
                        self.decimal_carry.read(),
                    )?;
                    self.a.write(&a);
                    self.q.write(&q);
                    self.decimal_carry.write(&carry);
                    if overflow == 1 {
                        self.overflow.write(&[1]);
                    }
                } else {
                    let left = join_double(self.a.read(), self.q.read());
                    let right = join_double(i32_to_bits::<20>(first), i32_to_bits::<20>(second));
                    let (result, overflow) = if operation == Operation::Dad {
                        gate_add(left, right)
                    } else {
                        gate_subtract(left, right)
                    };
                    let (a, q) = split_double(result);
                    self.a.write(&a);
                    self.q.write(&q);
                    if overflow == 1 {
                        self.overflow.write(&[1]);
                    }
                }
            }
            Operation::Dst => {
                if address & 1 == 0 {
                    let following = self.following_address(address)?;
                    self.write_word(address, bits_to_i32(&self.a.read()))?;
                    self.write_word(following, bits_to_i32(&self.q.read()))?;
                } else {
                    self.write_word(address, bits_to_i32(&self.q.read()))?;
                }
            }
            Operation::Inx => {
                let current = i32_to_bits::<15>(self.x_word(modifier)? & X_MASK);
                let increment = i32_to_bits::<15>(raw_address);
                let (updated, _) = gate_add(current, increment);
                let mut word = i32_to_bits::<20>(self.x_word(modifier)?);
                word[..15].copy_from_slice(&updated);
                self.set_x_word(modifier, bits_to_i32(&word))?;
            }
            Operation::Mpy => {
                let operand = self.read_word(address)?;
                self.m.write(&i32_to_bits::<20>(operand));
                let (product, overflow) =
                    gate_multiply_add(self.q.read(), i32_to_bits::<20>(operand), self.a.read());
                let (a, q) = split_double(product);
                self.a.write(&a);
                self.q.write(&q);
                self.overflow.write(&[overflow]);
            }
            Operation::Dvd => {
                let operand = self.read_word(address)?;
                let divisor = i32_to_bits::<20>(operand);
                self.m.write(&divisor);
                self.overflow.write(&[0]);
                let dividend = join_double(self.a.read(), self.q.read());
                if let Some((quotient, remainder)) = gate_divide(dividend, divisor) {
                    self.a.write(&quotient);
                    self.q.write(&remainder);
                } else {
                    self.overflow.write(&[1]);
                }
            }
            Operation::Stx => {
                self.write_word(raw_address, self.x_word(modifier)?)?;
            }
            Operation::Ext => {
                let operand = i32_to_bits::<20>(self.read_word(address)?);
                let result =
                    std::array::from_fn(|bit| and_gate(self.a.read()[bit], not_gate(operand[bit])));
                self.m.write(&operand);
                self.a.write(&result);
            }
            Operation::Cab => {
                let operand = i32_to_bits::<20>(self.read_word(address)?);
                self.m.write(&operand);
                let ordering = signed_compare(operand, self.a.read());
                if ordering == 0 {
                    self.advance_pc(1)?;
                } else if ordering < 0 {
                    self.advance_pc(2)?;
                }
            }
            Operation::Dcb => {
                let first = self.read_word(address)?;
                let second = if address & 1 == 0 {
                    self.read_word(self.following_address(address)?)?
                } else {
                    first
                };
                let operand = join_double(i32_to_bits::<20>(first), i32_to_bits::<20>(second));
                let accumulator = join_double(self.a.read(), self.q.read());
                let ordering = signed_compare(operand, accumulator);
                if ordering == 0 {
                    self.advance_pc(1)?;
                } else if ordering < 0 {
                    self.advance_pc(2)?;
                }
            }
            Operation::Ory => {
                let index = self.checked_address(address)?;
                let existing = self.memory[index].read();
                let a = self.a.read();
                self.memory[index]
                    .write(&std::array::from_fn(|bit| or_gate(existing[bit], a[bit])));
            }
            Operation::Mov => {
                let word_count = self.mov_word_count();
                let source = self.checked_range(raw_address, word_count)?;
                let destination = bits_to_i32(&self.a.read()) & X_MASK;
                let destination = self.checked_range(destination, word_count)?;
                let moved: Vec<[u8; 20]> = source.map(|index| self.memory[index].read()).collect();
                for (index, word) in destination.zip(moved) {
                    self.memory[index].write(&word);
                }
                self.set_x_word(0, bits_to_i32(&self.pc.read()))?;
                self.a.write(&[0; 20]);
            }
            Operation::Bru => self.pc.write(&i32_to_bits::<15>(address)),
            Operation::Sto => {
                let index = self.checked_address(address)?;
                let existing = self.memory[index].read();
                let a = self.a.read();
                self.memory[index].write(&std::array::from_fn(|bit| {
                    if bit < 13 {
                        a[bit]
                    } else {
                        existing[bit]
                    }
                }));
            }
            Operation::Rcd | Operation::Rcb | Operation::Rcf => {
                if self.card_reader_ready() == 0 {
                    self.reader_alarm();
                } else {
                    let format =
                        card_operation_format(operation).expect("a typed card read has a format");
                    self.card_reader_base.write(&i32_to_bits::<15>(address));
                    self.card_reader_slot.write(&[0; 2]);
                    self.card_reader_continuous.write(&match operation {
                        Operation::Rcd => [1, 0],
                        Operation::Rcb => [0, 1],
                        Operation::Rcf => [0, 0],
                        _ => unreachable!("the match contains typed card reads"),
                    });
                    self.transfer_card_input(format)?;
                    if operation == Operation::Rcf
                        && and_gate(
                            self.card_reader_ready(),
                            self.card_reader_api_enabled.read()[0],
                        ) == 1
                    {
                        self.card_reader_interrupt_pending.write(&[1]);
                    }
                }
            }
            Operation::Rcm => {
                if self.card_reader_ready() == 0 {
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
                        self.card_reader_base.write(&i32_to_bits::<15>(address));
                        self.card_reader_slot.write(&[0; 2]);
                        self.card_reader_continuous.write(&[0; 2]);
                        self.transfer_card_input(format)?;
                        if and_gate(
                            self.card_reader_ready(),
                            self.card_reader_api_enabled.read()[0],
                        ) == 1
                        {
                            self.card_reader_interrupt_pending.write(&[1]);
                        }
                    }
                }
            }
            Operation::Wcd | Operation::Wcb | Operation::Wcf => {
                if self.card_punch_ready() == 0 {
                    self.punch_alarm();
                } else {
                    self.transfer_card_punch(
                        card_operation_format(operation).expect("a typed card punch has a format"),
                        address,
                    )?;
                    if and_gate(
                        self.card_punch_ready(),
                        self.card_punch_api_enabled.read()[0],
                    ) == 1
                    {
                        self.card_punch_interrupt_pending.write(&[1]);
                    }
                }
            }
            Operation::Hcr => self.card_reader_continuous.write(&[0; 2]),
            Operation::Off => {
                self.typewriter_power.write(&[0]);
                self.n_device.write(&[0; 2]);
                self.paper_tape_reader_running.write(&[0]);
                self.typewriter_keyboard_enabled.write(&[0]);
                self.n_ready.write(&[0]);
            }
            Operation::NCommand => match decode_n_device(self.n_device.read()) {
                NRegisterDevice::Typewriter => {
                    if self.typewriter_power.read()[0] == 0 {
                        self.n_ready.write(&[0]);
                    } else {
                        let code = bits_to_i32(&self.n.read()) & N_MASK;
                        let output = match code {
                            0o37 => Some("\r"),
                            0o76 => Some("\t"),
                            0o72 | 0o75 => None,
                            _ => typewriter_char(code),
                        };
                        if let Some(output) = output {
                            self.typewriter_output.push(output.into());
                            self.n_ready.write(&[1]);
                        } else if matches!(code, 0o72 | 0o75) {
                            self.n_ready.write(&[1]);
                        } else {
                            self.n_ready.write(&[0]);
                        }
                    }
                }
                NRegisterDevice::PaperTapeReader => {
                    self.paper_tape_reader_running.write(&[1]);
                    self.n_ready.write(&[0]);
                    self.advance_paper_tape_reader()?;
                }
                NRegisterDevice::PaperTapePunch => {
                    self.paper_tape_output
                        .push(bits_to_i32(&self.n.read()) & N_MASK);
                    self.n_ready.write(&[1]);
                }
                NRegisterDevice::Off => self.n_ready.write(&[0]),
            },
            Operation::Ton => {
                self.n_device
                    .write(&encode_n_device(NRegisterDevice::Typewriter));
                self.typewriter_power.write(&[1]);
                self.paper_tape_reader_running.write(&[0]);
                self.typewriter_keyboard_enabled.write(&[0]);
                self.n_ready.write(&[1]);
            }
            Operation::Ron => {
                self.n_device
                    .write(&encode_n_device(NRegisterDevice::PaperTapeReader));
                self.typewriter_power.write(&[0]);
                self.paper_tape_reader_running.write(&[0]);
                self.typewriter_keyboard_enabled.write(&[0]);
                self.n.write(&[0; 6]);
                self.n_ready.write(&[0]);
            }
            Operation::Pon => {
                self.n_device
                    .write(&encode_n_device(NRegisterDevice::PaperTapePunch));
                self.typewriter_power.write(&[0]);
                self.paper_tape_reader_running.write(&[0]);
                self.typewriter_keyboard_enabled.write(&[0]);
                self.n_ready.write(&[1]);
            }
            Operation::Rcs => {
                let controls = self.control_switches.read();
                let a = self.a.read();
                self.a
                    .write(&std::array::from_fn(|bit| or_gate(a[bit], controls[bit])));
            }
            Operation::Hpt => match decode_n_device(self.n_device.read()) {
                NRegisterDevice::PaperTapeReader => self.paper_tape_reader_running.write(&[0]),
                NRegisterDevice::Typewriter => {
                    self.typewriter_keyboard_enabled.write(&[1]);
                    self.n_ready.write(&[0]);
                }
                NRegisterDevice::Off | NRegisterDevice::PaperTapePunch => {}
            },
            Operation::Sel => {
                let plug = raw_address as usize;
                if self.controller_selector_busy.read()[0] == 1
                    || self.controller_online[plug].read()[0] == 0
                {
                    self.controller_selector_alarm.write(&[1]);
                    self.priority_alarm.write(&[1]);
                    self.halted.write(&[1]);
                } else {
                    let command_word = self.read_word(bits_to_i32(&self.pc.read()))?;
                    let address_word =
                        self.read_word(self.following_address(bits_to_i32(&self.pc.read()))?)?;
                    let errors = self.controller_error_conditions[plug].read();
                    let conditions = self.controller_conditions[plug].read();
                    self.controller_conditions[plug].write(&std::array::from_fn(|bit| {
                        and_gate(conditions[bit], not_gate(errors[bit]))
                    }));
                    self.controller_error_conditions[plug].write(&[0; 64]);
                    self.controller_error[plug].write(&[0]);
                    self.set_controller_ready_value(plug, false);
                    self.controller_commands.push(ControllerCommand {
                        plug: plug as u8,
                        select_word: bits_to_i32(&self.ir.read()),
                        command_word,
                        address_word,
                    });
                    self.controller_selector_busy.write(&[1]);
                    self.selected_controller
                        .write(&encode_selected_controller(Some(plug as u8)));
                    self.advance_pc(2)?;
                }
            }
            Operation::BcsSet | Operation::BcsClear => {
                let plug = ((raw_address >> 6) & 0o7) as usize;
                let condition = (raw_address & CONTROLLER_CONDITION_MASK) as usize;
                let asserted = self.controller_conditions[plug].read()[condition];
                let branch = if operation == Operation::BcsSet {
                    asserted
                } else {
                    not_gate(asserted)
                };
                if branch == 0 {
                    self.advance_pc(1)?;
                }
            }
            Operation::SetPst => {
                self.automatic_interrupt_mode.write(&[1]);
                if self.priority_mode.read()[0] == 1 {
                    self.priority_return_armed.write(&[1]);
                }
            }
            Operation::SetPbk => self.automatic_interrupt_mode.write(&[0]),
            Operation::Ldz => self.a.write(&[0; 20]),
            Operation::Ldo => self.a.write(&i32_to_bits::<20>(1)),
            Operation::Lmo => self.a.write(&[1; 20]),
            Operation::Cpl => self
                .a
                .write(&std::array::from_fn(|bit| not_gate(self.a.read()[bit]))),
            Operation::Neg => {
                let before = self.a.read();
                let (result, overflow) = gate_subtract([0; 20], before);
                self.a.write(&result);
                if overflow == 1 {
                    self.overflow.write(&[1]);
                }
            }
            Operation::Chs => {
                let mut result = self.a.read();
                result[19] = not_gate(result[19]);
                self.a.write(&result);
            }
            Operation::Nop => {}
            Operation::Laq => self.a.write(&self.q.read()),
            Operation::Lqa => self.q.write(&self.a.read()),
            Operation::Xaq => {
                let a = self.a.read();
                let q = self.q.read();
                self.a.write(&q);
                self.q.write(&a);
            }
            Operation::Maq => {
                self.q.write(&self.a.read());
                self.a.write(&[0; 20]);
            }
            Operation::Ado | Operation::Sbo => {
                let before = self.a.read();
                let (result, carry, overflow) = if self.decimal_mode.read()[0] == 1 {
                    let (result, carry, overflow) = gate_decimal_word(
                        before,
                        decimal_one_bits(),
                        operation == Operation::Sbo,
                        self.decimal_carry.read(),
                    )?;
                    (result, Some(carry), overflow)
                } else if operation == Operation::Ado {
                    let (result, overflow) = gate_add(before, i32_to_bits::<20>(1));
                    (result, None, overflow)
                } else {
                    let (result, overflow) = gate_subtract(before, i32_to_bits::<20>(1));
                    (result, None, overflow)
                };
                self.a.write(&result);
                if let Some(carry) = carry {
                    self.decimal_carry.write(&carry);
                }
                if overflow == 1 {
                    self.overflow.write(&[1]);
                }
            }
            Operation::Lac => self.a.write(&with_sign_bits(self.clock_sixths.read(), 0)),
            Operation::Lca => {
                let clock: [u8; 19] = self.a.read()[..19]
                    .try_into()
                    .expect("the GE-225 clock receives A's nineteen data bits");
                self.clock_sixths.write(&clock);
            }
            Operation::SetDecimalMode => self.decimal_mode.write(&[1]),
            Operation::SetBinaryMode => self.decimal_mode.write(&[0]),
            Operation::Sra
            | Operation::Sna
            | Operation::Sca
            | Operation::San
            | Operation::Srd
            | Operation::Naq
            | Operation::Scd
            | Operation::Anq
            | Operation::Sla
            | Operation::Sld
            | Operation::Nor
            | Operation::Dno => self.execute_shift(operation, raw_address)?,
            Operation::Sxg => self.selected_x_group.write(&i32_to_bits::<6>(raw_address)),
            Operation::Bod
            | Operation::Bev
            | Operation::Bmi
            | Operation::Bpl
            | Operation::Bze
            | Operation::Bnz
            | Operation::Bov
            | Operation::Bno => {
                let a = self.a.read();
                let zero = is_zero(&a);
                let condition = match operation {
                    Operation::Bod => a[0],
                    Operation::Bev => not_gate(a[0]),
                    Operation::Bmi => a[19],
                    Operation::Bpl => not_gate(a[19]),
                    Operation::Bze => zero,
                    Operation::Bnz => not_gate(zero),
                    Operation::Bov => self.overflow.read()[0],
                    Operation::Bno => not_gate(self.overflow.read()[0]),
                    _ => 0,
                };
                if condition == 0 {
                    self.advance_pc(1)?;
                }
                if matches!(operation, Operation::Bov | Operation::Bno) {
                    self.overflow.write(&[0]);
                }
            }
            Operation::Bpe
            | Operation::Bpc
            | Operation::Bnr
            | Operation::Bnn
            | Operation::Bcr
            | Operation::Bcn
            | Operation::Bpr
            | Operation::Bpn => {
                let condition = match operation {
                    Operation::Bpe => self.parity_error.read()[0],
                    Operation::Bpc => not_gate(self.parity_error.read()[0]),
                    Operation::Bnr => self.n_ready.read()[0],
                    Operation::Bnn => not_gate(self.n_ready.read()[0]),
                    Operation::Bcr => self.card_reader_ready(),
                    Operation::Bcn => not_gate(self.card_reader_ready()),
                    Operation::Bpr => self.card_punch_ready(),
                    Operation::Bpn => not_gate(self.card_punch_ready()),
                    _ => unreachable!("the match contains only direct-I/O branches"),
                };
                if condition == 0 {
                    self.advance_pc(1)?;
                }
                if matches!(operation, Operation::Bpe | Operation::Bpc) {
                    self.parity_error.write(&[0]);
                }
            }
            _ => unreachable!("the AAU operation guards cover every remaining operation"),
        }
        Ok(())
    }

    fn read_aau_operand(&mut self, address: i32) -> Result<[u8; 40], Ge225GateError> {
        let first = i32_to_bits::<20>(self.read_word(address)?);
        let second = if address & 1 == 0 {
            i32_to_bits::<20>(self.read_word(self.following_address(address)?)?)
        } else {
            first
        };
        self.m.write(&second);
        Ok(join_aau_words(first, second))
    }

    fn write_aau_operand(&mut self, address: i32, value: [u8; 40]) -> Result<(), Ge225GateError> {
        let (first, second) = split_aau_words(value);
        let index = self.checked_address(address)?;
        if address & 1 == 0 {
            let following = self.following_address(address)?;
            let following_index = self.checked_address(following)?;
            self.memory[index].write(&first);
            self.memory[following_index].write(&second);
        } else {
            self.memory[index].write(&second);
        }
        self.m.write(&second);
        Ok(())
    }

    fn accept_aau_instruction(&mut self) {
        self.aau_overflow.write(&[0]);
        self.aau_underflow.write(&[0]);
    }

    fn set_aau_overflow(&mut self) {
        self.aau_overflow.write(&[1]);
        self.aau_overflow_hold.write(&[1]);
    }

    fn set_aau_underflow(&mut self) {
        self.aau_underflow.write(&[1]);
        self.aau_underflow_hold.write(&[1]);
    }

    fn capture_aau_ix(&mut self) {
        self.aau_ix.write(&zero_extend::<20, 40>(self.ir.read()));
    }

    fn execute_aau_general(&mut self, operation: Operation) -> Result<(), Ge225GateError> {
        self.capture_aau_ix();
        self.accept_aau_instruction();
        match operation {
            Operation::AauSetFixpoint => self.aau_mode.write(&encode_aau_mode(AauMode::FixedPoint)),
            Operation::AauSetNflpoint => self
                .aau_mode
                .write(&encode_aau_mode(AauMode::NormalizedFloatingPoint)),
            Operation::AauSetUflpoint => self
                .aau_mode
                .write(&encode_aau_mode(AauMode::UnnormalizedFloatingPoint)),
            Operation::AauLaq => self.aau_ax.write(&self.aau_qx.read()),
            Operation::AauLqa => self.aau_qx.write(&self.aau_ax.read()),
            Operation::AauMaq => {
                self.aau_qx.write(&self.aau_ax.read());
                self.aau_ax.write(&[0; 40]);
            }
            Operation::AauXaq => {
                let ax = self.aau_ax.read();
                let qx = self.aau_qx.read();
                self.aau_ax.write(&qx);
                self.aau_qx.write(&ax);
            }
            Operation::AauRov => self.aau_overflow_hold.write(&[0]),
            Operation::AauRun => self.aau_underflow_hold.write(&[0]),
            Operation::AauRin => {
                self.aau_overflow_hold.write(&[0]);
                self.aau_underflow_hold.write(&[0]);
            }
            Operation::AauNox => {
                let (exponent, mantissa) =
                    aau_float_pair_parts(self.aau_ax.read(), self.aau_qx.read());
                self.finish_aau_float_pair(exponent, sign_extend::<61, 64>(mantissa), true);
            }
            _ => unreachable!("the caller accepts only AAU general operations"),
        }
        Ok(())
    }

    fn aau_branch_condition(&self, operation: Operation) -> u8 {
        let mode = decode_aau_mode(self.aau_mode.read());
        let ax = self.aau_ax.read();
        let floating = u8::from(matches!(
            mode,
            Some(AauMode::NormalizedFloatingPoint | AauMode::UnnormalizedFloatingPoint)
        ));
        let minus = mux_bit(floating, ax[39], ax[19]);
        let zero = is_zero(&ax);
        let overflow = self.aau_overflow.read()[0];
        let underflow = self.aau_underflow.read()[0];
        let overflow_hold = self.aau_overflow_hold.read()[0];
        let underflow_hold = self.aau_underflow_hold.read()[0];
        match operation {
            Operation::AauBar => self.aau_ready.read()[0],
            Operation::AauBan => not_gate(self.aau_ready.read()[0]),
            Operation::AauBmi => minus,
            Operation::AauBpl => not_gate(minus),
            Operation::AauBze => zero,
            Operation::AauBnz => not_gate(zero),
            Operation::AauBov => overflow,
            Operation::AauBno => not_gate(overflow),
            Operation::AauBuf => underflow,
            Operation::AauBnu => not_gate(underflow),
            Operation::AauBoo => overflow_hold,
            Operation::AauBon => not_gate(overflow_hold),
            Operation::AauBuo => underflow_hold,
            Operation::AauBun => not_gate(underflow_hold),
            Operation::AauBer => or_gate(overflow, underflow),
            Operation::AauBne => not_gate(or_gate(overflow, underflow)),
            _ => unreachable!("the caller accepts only AAU status branches"),
        }
    }

    fn execute_aau_branch(&mut self, operation: Operation) -> Result<(), Ge225GateError> {
        self.capture_aau_ix();
        let condition = self.aau_branch_condition(operation);
        if matches!(operation, Operation::AauBoo | Operation::AauBon)
            && self.aau_overflow_hold.read()[0] == 1
        {
            self.aau_overflow_hold.write(&[0]);
        }
        if matches!(operation, Operation::AauBuo | Operation::AauBun)
            && self.aau_underflow_hold.read()[0] == 1
        {
            self.aau_underflow_hold.write(&[0]);
        }
        if condition == 0 {
            self.advance_pc(1)?;
        }
        Ok(())
    }

    fn execute_aau_memory(
        &mut self,
        operation: Operation,
        address: i32,
    ) -> Result<(), Ge225GateError> {
        self.capture_aau_ix();
        self.accept_aau_instruction();
        match operation {
            Operation::Fld => {
                let operand = self.read_aau_operand(address)?;
                self.aau_ax.write(&operand);
            }
            Operation::Fst => self.write_aau_operand(address, self.aau_ax.read())?,
            Operation::Fad | Operation::Fsu
                if decode_aau_mode(self.aau_mode.read()) == Some(AauMode::FixedPoint) =>
            {
                let operand = self.read_aau_operand(address)?;
                self.aau_bx.write(&operand);
                let left = aau_fixed_bits(self.aau_ax.read());
                let right = aau_fixed_bits(operand);
                let (result, alert) = if operation == Operation::Fad {
                    gate_add(left, right)
                } else {
                    gate_subtract(left, right)
                };
                if alert == 1 {
                    if left[AAU_FIXED_WORD_BITS - 1] == 0 {
                        self.set_aau_overflow();
                    } else {
                        self.set_aau_underflow();
                    }
                }
                self.aau_ax.write(&aau_fixed_raw(result));
            }
            Operation::Fmp
                if decode_aau_mode(self.aau_mode.read()) == Some(AauMode::FixedPoint) =>
            {
                let operand = self.read_aau_operand(address)?;
                self.aau_bx.write(&operand);
                let product = gate_signed_multiply::<39, 77>(
                    aau_fixed_bits(self.aau_qx.read()),
                    aau_fixed_bits(operand),
                );
                let (ax, qx) = split_aau_fixed_pair(product);
                self.aau_ax.write(&ax);
                self.aau_qx.write(&qx);
            }
            Operation::Fdv
                if decode_aau_mode(self.aau_mode.read()) == Some(AauMode::FixedPoint) =>
            {
                let operand = self.read_aau_operand(address)?;
                self.aau_bx.write(&operand);
                let divisor = aau_fixed_bits(operand);
                let dividend = join_aau_fixed_pair(self.aau_ax.read(), self.aau_qx.read());
                let high = aau_fixed_bits(self.aau_ax.read());
                if is_zero(&divisor) == 1
                    || greater_or_equal(&gate_absolute(high), &gate_absolute(divisor)) == 1
                {
                    if dividend[76] == 1 {
                        let magnitude = gate_twos_complement(dividend);
                        let (ax, qx) = split_aau_fixed_pair(magnitude);
                        self.aau_ax.write(&ax);
                        self.aau_qx.write(&qx);
                    }
                    self.set_aau_overflow();
                } else {
                    let (quotient, remainder) = gate_aau_fixed_divide(dividend, divisor)
                        .expect("AAU fixed divide preflight rejects zero divisors");
                    self.aau_ax.write(&aau_fixed_raw(quotient));
                    self.aau_qx.write(&aau_fixed_raw(remainder));
                }
            }
            Operation::Fad | Operation::Fsu | Operation::Fmp | Operation::Fdv => {
                self.execute_aau_floating(operation, address)?;
            }
            _ => unreachable!("the caller accepts only AAU memory operations"),
        }
        Ok(())
    }

    fn finish_aau_float_pair(
        &mut self,
        mut exponent: i32,
        mut mantissa: [u8; 64],
        normalize: bool,
    ) {
        if is_zero(&mantissa) == 1 {
            self.aau_ax.write(&[0; 40]);
            self.aau_qx.write(&[0; 40]);
            return;
        }
        while !fits_signed_width(&mantissa, 61) {
            mantissa = arithmetic_shift_right_bits(mantissa, 1);
            exponent = gate_i32_add(exponent, 1);
        }
        if normalize {
            while is_zero(&mantissa) == 0
                && gate_absolute(mantissa)[59..].iter().all(|bit| *bit == 0)
            {
                mantissa = shift_left_bits(mantissa, 1);
                exponent = gate_i32_subtract(exponent, 1);
            }
        }
        if exponent > 255 {
            self.set_aau_overflow();
        } else if exponent < -256 {
            self.set_aau_underflow();
            self.aau_ax.write(&[0; 40]);
            self.aau_qx.write(&[0; 40]);
            return;
        }
        let pair: [u8; 61] = mantissa[..61]
            .try_into()
            .expect("the bounded AAU floating pair is sixty-one bits");
        let (ax, qx) = aau_float_pair_raw(exponent, pair);
        self.aau_ax.write(&ax);
        self.aau_qx.write(&qx);
    }

    fn execute_aau_floating(
        &mut self,
        operation: Operation,
        address: i32,
    ) -> Result<(), Ge225GateError> {
        let normalized =
            decode_aau_mode(self.aau_mode.read()) == Some(AauMode::NormalizedFloatingPoint);
        let operand = self.read_aau_operand(address)?;
        self.aau_bx.write(&operand);
        let (bx_exponent, bx_mantissa) = aau_float_parts(operand);
        match operation {
            Operation::Fad | Operation::Fsu => {
                let (ax_exponent, ax_mantissa) = aau_float_parts(self.aau_ax.read());
                let target_exponent = ax_exponent.max(bx_exponent);
                let left = arithmetic_shift_right_bits(
                    shift_left_bits(sign_extend::<31, 64>(ax_mantissa), 30),
                    (target_exponent - ax_exponent).max(0) as usize,
                );
                let right = arithmetic_shift_right_bits(
                    shift_left_bits(sign_extend::<31, 64>(bx_mantissa), 30),
                    (target_exponent - bx_exponent).max(0) as usize,
                );
                let result = if operation == Operation::Fad {
                    gate_add(left, right).0
                } else {
                    gate_subtract(left, right).0
                };
                self.finish_aau_float_pair(target_exponent, result, normalized);
            }
            Operation::Fmp => {
                let (qx_exponent, qx_mantissa) = aau_float_parts(self.aau_qx.read());
                let product = gate_signed_multiply::<31, 64>(qx_mantissa, bx_mantissa);
                self.finish_aau_float_pair(
                    gate_i32_add(qx_exponent, bx_exponent),
                    product,
                    normalized,
                );
            }
            Operation::Fdv => self.execute_aau_float_divide(bx_exponent, bx_mantissa, normalized),
            _ => unreachable!("the caller accepts only AAU floating arithmetic"),
        }
        Ok(())
    }

    fn execute_aau_float_divide(
        &mut self,
        bx_exponent: i32,
        bx_mantissa: [u8; 31],
        normalized: bool,
    ) {
        let (mut ax_exponent, dividend_pair) =
            aau_float_pair_parts(self.aau_ax.read(), self.aau_qx.read());
        let dividend = sign_extend::<61, 64>(dividend_pair);
        let divisor = sign_extend::<31, 64>(bx_mantissa);
        if is_zero(&divisor) == 1 {
            if dividend[63] == 1 {
                let (ax, qx) = aau_float_pair_raw(
                    ax_exponent,
                    gate_absolute(dividend)[..61]
                        .try_into()
                        .expect("an AAU pair has sixty-one data bits"),
                );
                self.aau_ax.write(&ax);
                self.aau_qx.write(&qx);
            }
            self.set_aau_overflow();
            return;
        }
        if is_zero(&dividend) == 1 {
            self.aau_ax.write(&[0; 40]);
            self.aau_qx.write(&[0; 40]);
            return;
        }
        let dividend_negative = dividend[63];
        let divisor_negative = divisor[63];
        let mut dividend_magnitude = gate_absolute(dividend);
        let divisor_magnitude = gate_absolute(divisor);
        if greater_or_equal(
            &shift_right_bits(dividend_magnitude, 30),
            &divisor_magnitude,
        ) == 1
        {
            dividend_magnitude = shift_right_bits(dividend_magnitude, 1);
            ax_exponent = gate_i32_add(ax_exponent, 1);
            if greater_or_equal(
                &shift_right_bits(dividend_magnitude, 30),
                &divisor_magnitude,
            ) == 1
            {
                let pair: [u8; 61] = dividend_magnitude[..61]
                    .try_into()
                    .expect("the AAU dividend pair is sixty-one bits");
                let (ax, qx) = aau_float_pair_raw(ax_exponent, pair);
                self.aau_ax.write(&ax);
                self.aau_qx.write(&qx);
                self.set_aau_overflow();
                return;
            }
        }
        let mut quotient_exponent = gate_i32_subtract(ax_exponent, bx_exponent);
        let (quotient_magnitude, remainder_magnitude) =
            gate_unsigned_divide_64(dividend_magnitude, divisor_magnitude)
                .expect("AAU floating divide rejects zero divisors");
        let quotient_sign = xor_gate(dividend_negative, divisor_negative);
        let mut quotient = apply_sign(quotient_magnitude, quotient_sign);
        while !fits_signed_width(&quotient, 31) {
            quotient = arithmetic_shift_right_bits(quotient, 1);
            quotient_exponent = gate_i32_add(quotient_exponent, 1);
        }
        if normalized {
            while is_zero(&quotient) == 0
                && gate_absolute(quotient)[29..].iter().all(|bit| *bit == 0)
            {
                quotient = shift_left_bits(quotient, 1);
                quotient_exponent = gate_i32_subtract(quotient_exponent, 1);
            }
        }
        if quotient_exponent > 255 {
            self.set_aau_overflow();
        } else if quotient_exponent < -256 {
            self.set_aau_underflow();
            self.aau_ax.write(&[0; 40]);
            self.aau_qx.write(&[0; 40]);
        } else {
            let remainder = apply_sign(remainder_magnitude, dividend_negative);
            let quotient: [u8; 31] = quotient[..31]
                .try_into()
                .expect("the bounded AAU quotient is thirty-one bits");
            let remainder: [u8; 31] = remainder[..31]
                .try_into()
                .expect("the bounded AAU remainder is thirty-one bits");
            self.aau_ax
                .write(&aau_float_raw(quotient_exponent, quotient));
            self.aau_qx.write(&aau_float_raw(
                gate_i32_subtract(quotient_exponent, 30),
                remainder,
            ));
        }
    }

    fn execute_shift(&mut self, operation: Operation, count: i32) -> Result<(), Ge225GateError> {
        let count = count as usize;
        let a_before = self.a.read();
        let q_before = self.q.read();
        let a_sign = a_before[19];
        let q_sign = q_before[19];
        let mut a_data: [u8; 19] = a_before[..19]
            .try_into()
            .expect("the GE-225 A data field is nineteen bits");
        let mut q_data: [u8; 19] = q_before[..19]
            .try_into()
            .expect("the GE-225 Q data field is nineteen bits");
        match operation {
            Operation::Sra => {
                let mut shifted = a_before;
                for _ in 0..count.min(19) {
                    shifted = std::array::from_fn(|bit| {
                        if bit == 19 {
                            shifted[19]
                        } else {
                            shifted[bit + 1]
                        }
                    });
                }
                self.a.write(&shifted);
            }
            Operation::Sla => {
                for _ in 0..count {
                    if a_data[18] == 1 {
                        self.overflow.write(&[1]);
                    }
                    a_data = std::array::from_fn(|bit| if bit == 0 { 0 } else { a_data[bit - 1] });
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
            }
            Operation::Sca => {
                for _ in 0..(count % 19) {
                    let low = a_data[0];
                    a_data =
                        std::array::from_fn(|bit| if bit == 18 { low } else { a_data[bit + 1] });
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
            }
            Operation::San => {
                let mut n = self.n.read();
                for _ in 0..count {
                    let a_low = a_data[0];
                    a_data =
                        std::array::from_fn(|bit| if bit == 18 { a_sign } else { a_data[bit + 1] });
                    n = std::array::from_fn(|bit| if bit == 5 { a_low } else { n[bit + 1] });
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
                self.n.write(&n);
            }
            Operation::Sna => {
                let mut n = self.n.read();
                for _ in 0..count {
                    let n_low = n[0];
                    a_data =
                        std::array::from_fn(|bit| if bit == 18 { n_low } else { a_data[bit + 1] });
                    n = std::array::from_fn(|bit| if bit == 5 { 0 } else { n[bit + 1] });
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
                self.n.write(&n);
                self.n_ready.write(&[0]);
            }
            Operation::Srd => {
                for _ in 0..count {
                    let a_low = a_data[0];
                    a_data =
                        std::array::from_fn(|bit| if bit == 18 { a_sign } else { a_data[bit + 1] });
                    q_data =
                        std::array::from_fn(|bit| if bit == 18 { a_low } else { q_data[bit + 1] });
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
                self.q.write(&with_sign_bits(q_data, a_sign));
            }
            Operation::Naq => {
                let mut n = self.n.read();
                for _ in 0..count {
                    let n_low = n[0];
                    let a_low = a_data[0];
                    a_data =
                        std::array::from_fn(|bit| if bit == 18 { n_low } else { a_data[bit + 1] });
                    q_data =
                        std::array::from_fn(|bit| if bit == 18 { a_low } else { q_data[bit + 1] });
                    n = std::array::from_fn(|bit| if bit == 5 { 0 } else { n[bit + 1] });
                }
                self.n.write(&n);
                self.a.write(&with_sign_bits(a_data, a_sign));
                self.q.write(&with_sign_bits(q_data, a_sign));
                self.n_ready.write(&[0]);
            }
            Operation::Scd => {
                for _ in 0..(count % 38) {
                    let low = q_data[0];
                    let a_low = a_data[0];
                    a_data =
                        std::array::from_fn(|bit| if bit == 18 { low } else { a_data[bit + 1] });
                    q_data =
                        std::array::from_fn(|bit| if bit == 18 { a_low } else { q_data[bit + 1] });
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
                self.q.write(&with_sign_bits(q_data, a_sign));
            }
            Operation::Anq => {
                let mut a = a_before;
                let mut n = self.n.read();
                for _ in 0..count {
                    let bit = a[0];
                    a = std::array::from_fn(|position| {
                        if position == 19 {
                            a[19]
                        } else {
                            a[position + 1]
                        }
                    });
                    q_data = std::array::from_fn(|position| {
                        if position == 18 {
                            bit
                        } else {
                            q_data[position + 1]
                        }
                    });
                    n = std::array::from_fn(
                        |position| {
                            if position == 5 {
                                bit
                            } else {
                                n[position + 1]
                            }
                        },
                    );
                }
                self.a.write(&a);
                self.q.write(&with_sign_bits(q_data, a_sign));
                self.n.write(&n);
                self.n_ready.write(&[0]);
            }
            Operation::Sld => {
                for _ in 0..count {
                    if a_data[18] == 1 {
                        self.overflow.write(&[1]);
                    }
                    let q_high = q_data[18];
                    a_data =
                        std::array::from_fn(|bit| if bit == 0 { q_high } else { a_data[bit - 1] });
                    q_data = std::array::from_fn(|bit| if bit == 0 { 0 } else { q_data[bit - 1] });
                }
                self.a.write(&with_sign_bits(a_data, q_sign));
                self.q.write(&with_sign_bits(q_data, q_sign));
            }
            Operation::Nor => {
                let mut shifts = 0;
                while shifts < count && a_data[18] == a_sign {
                    if a_data[18] == 1 {
                        self.overflow.write(&[1]);
                    }
                    a_data = std::array::from_fn(|bit| if bit == 0 { 0 } else { a_data[bit - 1] });
                    shifts += 1;
                }
                self.a.write(&with_sign_bits(a_data, a_sign));
                self.write_word(0, (count - shifts) as i32)?;
            }
            Operation::Dno => {
                let mut shifts = 0;
                while shifts < count && a_data[18] == a_sign {
                    if a_data[18] == 1 {
                        self.overflow.write(&[1]);
                    }
                    let q_high = q_data[18];
                    a_data =
                        std::array::from_fn(|bit| if bit == 0 { q_high } else { a_data[bit - 1] });
                    q_data = std::array::from_fn(|bit| if bit == 0 { 0 } else { q_data[bit - 1] });
                    shifts += 1;
                }
                self.a.write(&with_sign_bits(a_data, q_sign));
                self.q.write(&with_sign_bits(q_data, q_sign));
                self.write_word(0, (count - shifts) as i32)?;
            }
            _ => unreachable!("only shift operations enter the gate shift network"),
        }
        Ok(())
    }

    fn advance_pc(&mut self, amount: i32) -> Result<(), Ge225GateError> {
        let next = bits_to_i32(&self.pc.read()) + amount;
        self.checked_address(next)?;
        self.pc.write(&i32_to_bits::<15>(next));
        Ok(())
    }
}

fn encode_n_device(device: NRegisterDevice) -> [u8; 2] {
    match device {
        NRegisterDevice::Off => [0, 0],
        NRegisterDevice::Typewriter => [1, 0],
        NRegisterDevice::PaperTapeReader => [0, 1],
        NRegisterDevice::PaperTapePunch => [1, 1],
    }
}

fn decode_n_device(bits: [u8; 2]) -> NRegisterDevice {
    match bits {
        [1, 0] => NRegisterDevice::Typewriter,
        [0, 1] => NRegisterDevice::PaperTapeReader,
        [1, 1] => NRegisterDevice::PaperTapePunch,
        _ => NRegisterDevice::Off,
    }
}

fn encode_selected_controller(plug: Option<u8>) -> [u8; 4] {
    plug.map_or([0; 4], |plug| {
        let mut bits = i32_to_bits::<4>(i32::from(plug));
        bits[3] = 1;
        bits
    })
}

fn decode_selected_controller(bits: [u8; 4]) -> Option<u8> {
    (bits[3] == 1).then_some(bits_to_i32(&bits[..3]) as u8)
}

fn decode_continuous(bits: [u8; 2]) -> Option<CardFormat> {
    match bits {
        [1, 0] => Some(CardFormat::Decimal),
        [0, 1] => Some(CardFormat::Binary10),
        _ => None,
    }
}

fn is_card_operation(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::Rcd
            | Operation::Rcb
            | Operation::Wcd
            | Operation::Wcb
            | Operation::Rcf
            | Operation::Rcm
            | Operation::Wcf
    )
}

fn is_card_read(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::Rcd | Operation::Rcb | Operation::Rcf | Operation::Rcm
    )
}

fn card_operation_format(operation: Operation) -> Option<CardFormat> {
    match operation {
        Operation::Rcd | Operation::Wcd => Some(CardFormat::Decimal),
        Operation::Rcb | Operation::Wcb => Some(CardFormat::Binary10),
        Operation::Rcf | Operation::Wcf => Some(CardFormat::Full12),
        _ => None,
    }
}

fn card_sync_word(format: CardFormat, status: CardStatus, hopper_empty: bool) -> i32 {
    let initial = match format {
        CardFormat::Decimal => CARD_DECIMAL_SYNC,
        CardFormat::Binary10 => CARD_BINARY_SYNC,
        CardFormat::Full12 | CardFormat::MixedDecimal | CardFormat::MixedBinary => CARD_FULL_SYNC,
    };
    let mut bits = i32_to_bits::<20>(initial);
    bits[18] = u8::from(hopper_empty);
    bits[3] = not_gate(u8::from(status.output_stacker_full));
    bits[2] = not_gate(u8::from(status.reader_malfunction));
    bits[1] = not_gate(u8::from(status.invalid_character));
    bits[0] = not_gate(and_gate(
        u8::from(hopper_empty),
        u8::from(status.end_of_file),
    ));
    bits_to_i32(&bits)
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

fn decode(word: i32, _n_device: NRegisterDevice) -> Option<(Operation, i32, i32)> {
    let normalized = word & WORD_MASK;
    if let Some(operation) = aau_exact_operation(normalized) {
        return Some((operation, 0, 0));
    }
    let modifier = (normalized >> 13) & 0x03;
    let canonical = normalized & !(0x03 << 13);
    let canonical_bits = i32_to_bits::<20>(canonical);
    if (canonical & !CONTROLLER_PLUG_MASK) == CONTROLLER_SELECT_BASE {
        return Some((
            Operation::Sel,
            modifier,
            (canonical & CONTROLLER_PLUG_MASK) >> 6,
        ));
    }
    let controller_status_base = normalized & !(CONTROLLER_PLUG_MASK | CONTROLLER_CONDITION_MASK);
    let controller_condition = (normalized & CONTROLLER_CONDITION_MASK) as u8;
    if matches!(
        controller_status_base,
        CONTROLLER_STATUS_SET_BASE | CONTROLLER_STATUS_CLEAR_BASE
    ) && (CONTROLLER_CONDITION_MIN..=CONTROLLER_CONDITION_MAX).contains(&controller_condition)
    {
        let operation = if controller_status_base == CONTROLLER_STATUS_SET_BASE {
            Operation::BcsSet
        } else {
            Operation::BcsClear
        };
        let plug = (normalized & CONTROLLER_PLUG_MASK) >> 6;
        return Some((operation, 0, (plug << 6) | i32::from(controller_condition)));
    }
    if (canonical >> 15) == 0o25 {
        let address = canonical & ADDRESS_MASK;
        let base = address & !(CARD_ADDRESS_ALIGNMENT - 1);
        let reserved = address & 0o160;
        if base < CARD_ADDRESS_LIMIT && reserved == 0 {
            let operation = match address & 0o17 {
                0o00 => Some(Operation::Rcd),
                0o01 => Some(Operation::Rcb),
                0o02 => Some(Operation::Wcd),
                0o03 => Some(Operation::Wcb),
                0o10 => Some(Operation::Rcf),
                0o12 => Some(Operation::Rcm),
                0o17 => Some(Operation::Wcf),
                _ => None,
            };
            if let Some(operation) = operation {
                return Some((operation, modifier, base));
            }
        }
    }
    let fixed = FIXED_OPERATIONS.iter().find_map(|(code, operation)| {
        (equal_bits(&canonical_bits, &i32_to_bits::<20>(*code)) == 1).then_some(*operation)
    });
    if let Some(operation) = fixed {
        return Some((operation, modifier, 0));
    }
    let sxg_masked = canonical & !(0x1f << 3);
    if equal_bits(
        &i32_to_bits::<20>(sxg_masked),
        &i32_to_bits::<20>(0o2506003),
    ) == 1
    {
        return Some((Operation::Sxg, modifier, (canonical >> 3) & 0x1f));
    }
    let shift = SHIFT_OPERATIONS.iter().find_map(|(base, operation)| {
        let masked = canonical & !0o37;
        (equal_bits(&i32_to_bits::<20>(masked), &i32_to_bits::<20>(*base)) == 1)
            .then_some(*operation)
    });
    if let Some(operation) = shift {
        return Some((operation, modifier, canonical & 0o37));
    }
    let opcode_bits: [u8; 5] = canonical_bits[15..20]
        .try_into()
        .expect("the GE-225 opcode field is five bits");
    let selectors = decode_bits(opcode_bits);
    let operation = MEMORY_OPERATIONS
        .iter()
        .find_map(|(opcode, operation)| (selectors[*opcode] == 1).then_some(*operation))?;
    Some((operation, modifier, normalized & ADDRESS_MASK))
}

fn aau_exact_operation(word: i32) -> Option<Operation> {
    Some(match word {
        0o3500010 => Operation::AauSetFixpoint,
        0o3100010 => Operation::AauSetNflpoint,
        0o3200010 => Operation::AauSetUflpoint,
        0o3600002 => Operation::AauLaq,
        0o3200002 => Operation::AauLqa,
        0o3100002 => Operation::AauMaq,
        0o3500002 => Operation::AauXaq,
        0o3100004 => Operation::AauRov,
        0o3200004 => Operation::AauRun,
        0o3500004 => Operation::AauRin,
        0o3100005 => Operation::AauNox,
        0o2514720 => Operation::AauBar,
        0o2516720 => Operation::AauBan,
        0o2514721 => Operation::AauBmi,
        0o2516721 => Operation::AauBpl,
        0o2514722 => Operation::AauBze,
        0o2516722 => Operation::AauBnz,
        0o2514723 => Operation::AauBov,
        0o2516723 => Operation::AauBno,
        0o2514724 => Operation::AauBuf,
        0o2516724 => Operation::AauBnu,
        0o2514725 => Operation::AauBoo,
        0o2516725 => Operation::AauBon,
        0o2514726 => Operation::AauBuo,
        0o2516726 => Operation::AauBun,
        0o2514727 => Operation::AauBer,
        0o2516727 => Operation::AauBne,
        _ => return None,
    })
}

const FIXED_OPERATIONS: &[(i32, Operation)] = &[
    (0o2500004, Operation::Hcr),
    (0o2500005, Operation::Off),
    (0o2500006, Operation::NCommand),
    (0o2500007, Operation::Ton),
    (0o2500011, Operation::Rcs),
    (0o2500014, Operation::Ron),
    (0o2500015, Operation::Pon),
    (0o2500016, Operation::Hpt),
    (0o2506015, Operation::SetPst),
    (0o2506016, Operation::SetPbk),
    (0o2504002, Operation::Ldz),
    (0o2504022, Operation::Ldo),
    (0o2504102, Operation::Lmo),
    (0o2504502, Operation::Cpl),
    (0o2504522, Operation::Neg),
    (0o2504040, Operation::Chs),
    (0o2504012, Operation::Nop),
    (0o2504001, Operation::Laq),
    (0o2504004, Operation::Lqa),
    (0o2504005, Operation::Xaq),
    (0o2504006, Operation::Maq),
    (0o2504032, Operation::Ado),
    (0o2504112, Operation::Sbo),
    (0o2504202, Operation::Lac),
    (0o2504210, Operation::Lca),
    (0o2506011, Operation::SetDecimalMode),
    (0o2506012, Operation::SetBinaryMode),
    (0o2514000, Operation::Bod),
    (0o2516000, Operation::Bev),
    (0o2514001, Operation::Bmi),
    (0o2516001, Operation::Bpl),
    (0o2514002, Operation::Bze),
    (0o2516002, Operation::Bnz),
    (0o2514003, Operation::Bov),
    (0o2516003, Operation::Bno),
    (0o2514004, Operation::Bpe),
    (0o2516004, Operation::Bpc),
    (0o2514005, Operation::Bnr),
    (0o2516005, Operation::Bnn),
    (0o2514006, Operation::Bcr),
    (0o2516006, Operation::Bcn),
    (0o2514007, Operation::Bpr),
    (0o2516007, Operation::Bpn),
];

const MEMORY_OPERATIONS: &[(usize, Operation)] = &[
    (0o00, Operation::Lda),
    (0o01, Operation::Add),
    (0o02, Operation::Sub),
    (0o03, Operation::Sta),
    (0o04, Operation::Bxl),
    (0o05, Operation::Bxh),
    (0o06, Operation::Ldx),
    (0o07, Operation::Spb),
    (0o10, Operation::Dld),
    (0o11, Operation::Dad),
    (0o12, Operation::Dsu),
    (0o13, Operation::Dst),
    (0o14, Operation::Inx),
    (0o15, Operation::Mpy),
    (0o16, Operation::Dvd),
    (0o17, Operation::Stx),
    (0o20, Operation::Ext),
    (0o21, Operation::Cab),
    (0o22, Operation::Dcb),
    (0o23, Operation::Ory),
    (0o24, Operation::Mov),
    (0o26, Operation::Bru),
    (0o27, Operation::Sto),
    (0o30, Operation::Fld),
    (0o31, Operation::Fad),
    (0o32, Operation::Fsu),
    (0o33, Operation::Fst),
    (0o35, Operation::Fmp),
    (0o36, Operation::Fdv),
];

const SHIFT_OPERATIONS: &[(i32, Operation)] = &[
    (0o2510000, Operation::Sra),
    (0o2510100, Operation::Sna),
    (0o2510040, Operation::Sca),
    (0o2510400, Operation::San),
    (0o2511000, Operation::Srd),
    (0o2511100, Operation::Naq),
    (0o2511200, Operation::Scd),
    (0o2511400, Operation::Anq),
    (0o2512000, Operation::Sla),
    (0o2512200, Operation::Sld),
    (0o2513000, Operation::Nor),
    (0o2513200, Operation::Dno),
];

fn is_fixed(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::Ldz
            | Operation::Ldo
            | Operation::Lmo
            | Operation::Cpl
            | Operation::Neg
            | Operation::Chs
            | Operation::Nop
            | Operation::Laq
            | Operation::Lqa
            | Operation::Xaq
            | Operation::Maq
            | Operation::Ado
            | Operation::Sbo
            | Operation::Lac
            | Operation::Lca
            | Operation::SetDecimalMode
            | Operation::SetBinaryMode
            | Operation::Bod
            | Operation::Bev
            | Operation::Bmi
            | Operation::Bpl
            | Operation::Bze
            | Operation::Bnz
            | Operation::Bov
            | Operation::Bno
            | Operation::Sra
            | Operation::Sna
            | Operation::Sca
            | Operation::San
            | Operation::Srd
            | Operation::Naq
            | Operation::Scd
            | Operation::Anq
            | Operation::Sla
            | Operation::Sld
            | Operation::Nor
            | Operation::Dno
            | Operation::Sxg
            | Operation::Hcr
            | Operation::Off
            | Operation::NCommand
            | Operation::Ton
            | Operation::Rcs
            | Operation::Ron
            | Operation::Pon
            | Operation::Hpt
            | Operation::Bpe
            | Operation::Bpc
            | Operation::Bnr
            | Operation::Bnn
            | Operation::Bcr
            | Operation::Bcn
            | Operation::Bpr
            | Operation::Bpn
            | Operation::Sel
            | Operation::BcsSet
            | Operation::BcsClear
            | Operation::SetPst
            | Operation::SetPbk
            | Operation::AauSetFixpoint
            | Operation::AauSetNflpoint
            | Operation::AauSetUflpoint
            | Operation::AauLaq
            | Operation::AauLqa
            | Operation::AauMaq
            | Operation::AauXaq
            | Operation::AauRov
            | Operation::AauRun
            | Operation::AauRin
            | Operation::AauNox
            | Operation::AauBar
            | Operation::AauBan
            | Operation::AauBmi
            | Operation::AauBpl
            | Operation::AauBze
            | Operation::AauBnz
            | Operation::AauBov
            | Operation::AauBno
            | Operation::AauBuf
            | Operation::AauBnu
            | Operation::AauBoo
            | Operation::AauBon
            | Operation::AauBuo
            | Operation::AauBun
            | Operation::AauBer
            | Operation::AauBne
    )
}

fn is_shift(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::Sra
            | Operation::Sna
            | Operation::Sca
            | Operation::San
            | Operation::Srd
            | Operation::Naq
            | Operation::Scd
            | Operation::Anq
            | Operation::Sla
            | Operation::Sld
            | Operation::Nor
            | Operation::Dno
    )
}

fn is_aau_memory(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::Fld
            | Operation::Fad
            | Operation::Fsu
            | Operation::Fst
            | Operation::Fmp
            | Operation::Fdv
    )
}

fn is_aau_general(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::AauSetFixpoint
            | Operation::AauSetNflpoint
            | Operation::AauSetUflpoint
            | Operation::AauLaq
            | Operation::AauLqa
            | Operation::AauMaq
            | Operation::AauXaq
            | Operation::AauRov
            | Operation::AauRun
            | Operation::AauRin
            | Operation::AauNox
    )
}

fn is_aau_branch(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::AauBar
            | Operation::AauBan
            | Operation::AauBmi
            | Operation::AauBpl
            | Operation::AauBze
            | Operation::AauBnz
            | Operation::AauBov
            | Operation::AauBno
            | Operation::AauBuf
            | Operation::AauBnu
            | Operation::AauBoo
            | Operation::AauBon
            | Operation::AauBuo
            | Operation::AauBun
            | Operation::AauBer
            | Operation::AauBne
    )
}

fn is_memory_reference(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::Lda
            | Operation::Add
            | Operation::Sub
            | Operation::Sta
            | Operation::Dld
            | Operation::Dad
            | Operation::Dsu
            | Operation::Dst
            | Operation::Mpy
            | Operation::Dvd
            | Operation::Ext
            | Operation::Cab
            | Operation::Dcb
            | Operation::Ory
            | Operation::Bru
            | Operation::Sto
            | Operation::Rcd
            | Operation::Rcb
            | Operation::Wcd
            | Operation::Wcb
            | Operation::Rcf
            | Operation::Rcm
            | Operation::Wcf
            | Operation::Fld
            | Operation::Fad
            | Operation::Fsu
            | Operation::Fst
            | Operation::Fmp
            | Operation::Fdv
    )
}

fn operation_name(operation: Operation) -> &'static str {
    match operation {
        Operation::Lda => "LDA",
        Operation::Add => "ADD",
        Operation::Sub => "SUB",
        Operation::Sta => "STA",
        Operation::Bxl => "BXL",
        Operation::Bxh => "BXH",
        Operation::Ldx => "LDX",
        Operation::Spb => "SPB",
        Operation::Dld => "DLD",
        Operation::Dad => "DAD",
        Operation::Dsu => "DSU",
        Operation::Dst => "DST",
        Operation::Inx => "INX",
        Operation::Mpy => "MPY",
        Operation::Dvd => "DVD",
        Operation::Stx => "STX",
        Operation::Ext => "EXT",
        Operation::Cab => "CAB",
        Operation::Dcb => "DCB",
        Operation::Ory => "ORY",
        Operation::Mov => "MOV",
        Operation::Bru => "BRU",
        Operation::Sto => "STO",
        Operation::Ldz => "LDZ",
        Operation::Ldo => "LDO",
        Operation::Lmo => "LMO",
        Operation::Cpl => "CPL",
        Operation::Neg => "NEG",
        Operation::Chs => "CHS",
        Operation::Nop => "NOP",
        Operation::Laq => "LAQ",
        Operation::Lqa => "LQA",
        Operation::Xaq => "XAQ",
        Operation::Maq => "MAQ",
        Operation::Ado => "ADO",
        Operation::Sbo => "SBO",
        Operation::Lac => "LAC",
        Operation::Lca => "LCA",
        Operation::SetDecimalMode => "SET_DECMODE",
        Operation::SetBinaryMode => "SET_BINMODE",
        Operation::Bod => "BOD",
        Operation::Bev => "BEV",
        Operation::Bmi => "BMI",
        Operation::Bpl => "BPL",
        Operation::Bze => "BZE",
        Operation::Bnz => "BNZ",
        Operation::Bov => "BOV",
        Operation::Bno => "BNO",
        Operation::Sra => "SRA",
        Operation::Sna => "SNA",
        Operation::Sca => "SCA",
        Operation::San => "SAN",
        Operation::Srd => "SRD",
        Operation::Naq => "NAQ",
        Operation::Scd => "SCD",
        Operation::Anq => "ANQ",
        Operation::Sla => "SLA",
        Operation::Sld => "SLD",
        Operation::Nor => "NOR",
        Operation::Dno => "DNO",
        Operation::Sxg => "SXG",
        Operation::Rcd => "RCD",
        Operation::Rcb => "RCB",
        Operation::Wcd => "WCD",
        Operation::Wcb => "WCB",
        Operation::Rcf => "RCF",
        Operation::Rcm => "RCM",
        Operation::Wcf => "WCF",
        Operation::Hcr => "HCR",
        Operation::Off => "OFF",
        Operation::NCommand => "NIO",
        Operation::Ton => "TON",
        Operation::Rcs => "RCS",
        Operation::Ron => "RON",
        Operation::Pon => "PON",
        Operation::Hpt => "HPT",
        Operation::Bpe => "BPE",
        Operation::Bpc => "BPC",
        Operation::Bnr => "BNR",
        Operation::Bnn => "BNN",
        Operation::Bcr => "BCR",
        Operation::Bcn => "BCN",
        Operation::Bpr => "BPR",
        Operation::Bpn => "BPN",
        Operation::Sel => "SEL",
        Operation::BcsSet | Operation::BcsClear => "BCS",
        Operation::SetPst => "SET_PST",
        Operation::SetPbk => "SET_PBK",
        Operation::Fld => "FLD",
        Operation::Fad => "FAD",
        Operation::Fsu => "FSU",
        Operation::Fst => "FST",
        Operation::Fmp => "FMP",
        Operation::Fdv => "FDV",
        Operation::AauSetFixpoint => "SET_FIXPOINT",
        Operation::AauSetNflpoint => "SET_NFLPOINT",
        Operation::AauSetUflpoint => "SET_UFLPOINT",
        Operation::AauLaq => "LAQ",
        Operation::AauLqa => "LQA",
        Operation::AauMaq => "MAQ",
        Operation::AauXaq => "XAQ",
        Operation::AauRov => "ROV",
        Operation::AauRun => "RUN",
        Operation::AauRin => "RIN",
        Operation::AauNox => "NOX",
        Operation::AauBar => "BAR",
        Operation::AauBan => "BAN",
        Operation::AauBmi => "BMI",
        Operation::AauBpl => "BPL",
        Operation::AauBze => "BZE",
        Operation::AauBnz => "BNZ",
        Operation::AauBov => "BOV",
        Operation::AauBno => "BNO",
        Operation::AauBuf => "BUF",
        Operation::AauBnu => "BNU",
        Operation::AauBoo => "BOO",
        Operation::AauBon => "BON",
        Operation::AauBuo => "BUO",
        Operation::AauBun => "BUN",
        Operation::AauBer => "BER",
        Operation::AauBne => "BNE",
    }
}

fn decimal_one_bits() -> [u8; 20] {
    let mut word = [0; 20];
    word[0] = 1;
    word
}

fn gate_decimal_digits<const WIDTH: usize>(word: [u8; 20]) -> Result<[u8; WIDTH], Ge225GateError> {
    let ones: [u8; 4] = word[..4].try_into().expect("ones digit width");
    let tens: [u8; 4] = word[6..10].try_into().expect("tens digit width");
    let hundreds: [u8; 4] = word[12..16].try_into().expect("hundreds digit width");
    let ten = i32_to_bits::<4>(10);
    if [ones, tens, hundreds]
        .iter()
        .any(|digit| greater_or_equal(digit, &ten) == 1)
    {
        return Err(Ge225GateError::InvalidBcd {
            word: bits_to_i32(&word),
        });
    }
    let ones = zero_extend::<4, WIDTH>(ones);
    let tens = gate_multiply_constant(zero_extend::<4, WIDTH>(tens), 10);
    let hundreds = gate_multiply_constant(zero_extend::<4, WIDTH>(hundreds), 100);
    Ok(gate_add(gate_add(hundreds, tens).0, ones).0)
}

fn gate_decimal_word(
    accumulator: [u8; 20],
    operand: [u8; 20],
    subtract: bool,
    carry: [u8; 2],
) -> Result<([u8; 20], [u8; 2], u8), Ge225GateError> {
    let left_raw = gate_decimal_digits::<13>(accumulator)?;
    let right_raw = gate_decimal_digits::<13>(operand)?;
    let flagged = accumulator[18];
    if operand[18] == 1 && flagged == 0 {
        return Err(Ge225GateError::FlaggedDecimalOperand { double: false });
    }
    let total = gate_decimal_total(
        left_raw,
        and_gate(accumulator[19], flagged),
        right_raw,
        and_gate(operand[19], flagged),
        subtract,
        carry,
        1_000,
    );
    let (raw, negative, next_carry, overflow) = gate_normalize_decimal(total, flagged, 1_000);
    Ok((
        gate_encode_decimal(raw, negative, flagged),
        next_carry,
        overflow,
    ))
}

fn gate_decimal_pair(
    a: [u8; 20],
    q: [u8; 20],
    high_operand: [u8; 20],
    low_operand: [u8; 20],
    subtract: bool,
    carry: [u8; 2],
) -> Result<DecimalPairResult, Ge225GateError> {
    let a_high = gate_decimal_digits::<23>(a)?;
    let a_low = gate_decimal_digits::<23>(q)?;
    let operand_high = gate_decimal_digits::<23>(high_operand)?;
    let operand_low = gate_decimal_digits::<23>(low_operand)?;
    let flagged = a[18];
    if high_operand[18] == 1 && flagged == 0 {
        return Err(Ge225GateError::FlaggedDecimalOperand { double: true });
    }
    let left_raw = gate_add(gate_multiply_constant(a_high, 1_000), a_low).0;
    let right_raw = gate_add(gate_multiply_constant(operand_high, 1_000), operand_low).0;
    let total = gate_decimal_total(
        left_raw,
        and_gate(a[19], flagged),
        right_raw,
        and_gate(high_operand[19], flagged),
        subtract,
        carry,
        1_000_000,
    );
    let (raw, negative, next_carry, overflow) = gate_normalize_decimal(total, flagged, 1_000_000);
    let (high, low) = gate_divide_constant(raw, 1_000);
    Ok((
        gate_encode_decimal(high, negative, flagged),
        gate_encode_decimal(low, 0, 0),
        next_carry,
        overflow,
    ))
}

fn gate_decimal_total<const WIDTH: usize>(
    left_raw: [u8; WIDTH],
    left_negative: u8,
    right_raw: [u8; WIDTH],
    right_negative: u8,
    subtract: bool,
    carry: [u8; 2],
    modulus: i32,
) -> [u8; WIDTH] {
    let modulus_bits = i32_to_bits::<WIDTH>(modulus);
    let left_signed = mux_bits(
        and_gate(left_negative, not_gate(is_zero(&left_raw))),
        left_raw,
        gate_subtract(left_raw, modulus_bits).0,
    );
    let right_signed = mux_bits(
        and_gate(right_negative, not_gate(is_zero(&right_raw))),
        right_raw,
        gate_subtract(right_raw, modulus_bits).0,
    );
    let combined = if subtract {
        gate_subtract(left_signed, right_signed).0
    } else {
        gate_add(left_signed, right_signed).0
    };
    gate_add(combined, sign_extend::<2, WIDTH>(carry)).0
}

fn gate_normalize_decimal<const WIDTH: usize>(
    total: [u8; WIDTH],
    flagged: u8,
    modulus: i32,
) -> ([u8; WIDTH], u8, [u8; 2], u8) {
    let modulus_bits = i32_to_bits::<WIDTH>(modulus);
    let twice_modulus = i32_to_bits::<WIDTH>(modulus * 2);
    let negative_modulus = gate_subtract([0; WIDTH], modulus_bits).0;
    let at_least_modulus = signed_greater_or_equal(&total, &modulus_bits);
    let at_most_negative_modulus = signed_greater_or_equal(&negative_modulus, &total);
    let negative = total[WIDTH - 1];
    let middle_negative = and_gate(negative, not_gate(at_most_negative_modulus));

    let plus_modulus = gate_add(total, modulus_bits).0;
    let plus_twice_modulus = gate_add(total, twice_modulus).0;
    let minus_modulus = gate_subtract(total, modulus_bits).0;
    let mut raw = mux_bits(middle_negative, total, plus_modulus);
    raw = mux_bits(at_most_negative_modulus, raw, plus_twice_modulus);
    raw = mux_bits(at_least_modulus, raw, minus_modulus);

    let flagged_negative = or_gate(
        at_least_modulus,
        and_gate(negative, not_gate(at_most_negative_modulus)),
    );
    let overflow = and_gate(flagged, or_gate(at_least_modulus, at_most_negative_modulus));
    let unflagged = not_gate(flagged);
    let positive_carry = and_gate(unflagged, at_least_modulus);
    let negative_carry = and_gate(unflagged, negative);
    let carry = [or_gate(positive_carry, negative_carry), negative_carry];
    (raw, and_gate(flagged, flagged_negative), carry, overflow)
}

fn gate_encode_decimal<const WIDTH: usize>(
    raw: [u8; WIDTH],
    negative: u8,
    flagged: u8,
) -> [u8; 20] {
    let (hundreds, remainder) = gate_divide_constant(raw, 100);
    let (tens, ones) = gate_divide_constant(remainder, 10);
    let mut word = [0; 20];
    word[..4].copy_from_slice(&ones[..4]);
    word[6..10].copy_from_slice(&tens[..4]);
    word[12..16].copy_from_slice(&hundreds[..4]);
    word[18] = flagged;
    word[19] = negative;
    word
}

fn gate_divide_constant<const WIDTH: usize>(
    dividend: [u8; WIDTH],
    divisor: i32,
) -> ([u8; WIDTH], [u8; WIDTH]) {
    let divisor = u64_to_bits::<WIDTH>(divisor as u64);
    let mut quotient = [0; WIDTH];
    let mut remainder = [0; WIDTH];
    for bit in (0..WIDTH).rev() {
        remainder = std::array::from_fn(|position| {
            if position == 0 {
                dividend[bit]
            } else {
                remainder[position - 1]
            }
        });
        let subtract = greater_or_equal(&remainder, &divisor);
        remainder = mux_bits(subtract, remainder, gate_subtract(remainder, divisor).0);
        quotient[bit] = subtract;
    }
    (quotient, remainder)
}

fn gate_multiply_constant<const WIDTH: usize>(value: [u8; WIDTH], multiplier: u32) -> [u8; WIDTH] {
    let mut product = [0; WIDTH];
    for bit in 0..32 {
        if (multiplier >> bit) & 1 == 1 {
            let partial = std::array::from_fn(|position| {
                if position >= bit {
                    value[position - bit]
                } else {
                    0
                }
            });
            product = gate_add(product, partial).0;
        }
    }
    product
}

fn mux_bits<const WIDTH: usize>(select: u8, zero: [u8; WIDTH], one: [u8; WIDTH]) -> [u8; WIDTH] {
    std::array::from_fn(|bit| {
        or_gate(
            and_gate(not_gate(select), zero[bit]),
            and_gate(select, one[bit]),
        )
    })
}

fn zero_extend<const FROM: usize, const TO: usize>(value: [u8; FROM]) -> [u8; TO] {
    std::array::from_fn(|bit| if bit < FROM { value[bit] } else { 0 })
}

fn sign_extend<const FROM: usize, const TO: usize>(value: [u8; FROM]) -> [u8; TO] {
    std::array::from_fn(|bit| {
        if bit < FROM {
            value[bit]
        } else {
            value[FROM - 1]
        }
    })
}

fn signed_greater_or_equal<const WIDTH: usize>(left: &[u8; WIDTH], right: &[u8; WIDTH]) -> u8 {
    let signs_differ = xor_gate(left[WIDTH - 1], right[WIDTH - 1]);
    let left_positive = and_gate(not_gate(left[WIDTH - 1]), right[WIDTH - 1]);
    or_gate(
        and_gate(signs_differ, left_positive),
        and_gate(not_gate(signs_differ), greater_or_equal(left, right)),
    )
}

fn decode_decimal_carry(bits: [u8; 2]) -> i32 {
    match bits {
        [1, 1] => -1,
        [1, 0] => 1,
        _ => 0,
    }
}

fn encode_aau_mode(mode: AauMode) -> [u8; 2] {
    match mode {
        AauMode::FixedPoint => [1, 0],
        AauMode::NormalizedFloatingPoint => [0, 1],
        AauMode::UnnormalizedFloatingPoint => [1, 1],
    }
}

fn decode_aau_mode(bits: [u8; 2]) -> Option<AauMode> {
    match bits {
        [1, 0] => Some(AauMode::FixedPoint),
        [0, 1] => Some(AauMode::NormalizedFloatingPoint),
        [1, 1] => Some(AauMode::UnnormalizedFloatingPoint),
        _ => None,
    }
}

fn mux_bit(select: u8, zero: u8, one: u8) -> u8 {
    or_gate(and_gate(not_gate(select), zero), and_gate(select, one))
}

fn join_aau_words(first: [u8; 20], second: [u8; 20]) -> [u8; 40] {
    std::array::from_fn(|bit| {
        if bit < 20 {
            second[bit]
        } else {
            first[bit - 20]
        }
    })
}

fn split_aau_words(value: [u8; 40]) -> ([u8; 20], [u8; 20]) {
    let first = std::array::from_fn(|bit| value[bit + 20]);
    let second = std::array::from_fn(|bit| value[bit]);
    (first, second)
}

fn aau_fixed_bits(raw: [u8; 40]) -> [u8; 39] {
    std::array::from_fn(|bit| if bit < 19 { raw[bit] } else { raw[bit + 1] })
}

fn aau_fixed_raw(value: [u8; 39]) -> [u8; 40] {
    std::array::from_fn(|bit| match bit {
        0..=18 => value[bit],
        19 => value[38],
        _ => value[bit - 1],
    })
}

fn join_aau_fixed_pair(ax: [u8; 40], qx: [u8; 40]) -> [u8; 77] {
    let ax = aau_fixed_bits(ax);
    let qx = aau_fixed_bits(qx);
    std::array::from_fn(|bit| if bit < 38 { qx[bit] } else { ax[bit - 38] })
}

fn split_aau_fixed_pair(value: [u8; 77]) -> ([u8; 40], [u8; 40]) {
    let ax: [u8; 39] = std::array::from_fn(|bit| value[bit + 38]);
    let qx: [u8; 39] = std::array::from_fn(|bit| if bit < 38 { value[bit] } else { ax[38] });
    (aau_fixed_raw(ax), aau_fixed_raw(qx))
}

fn aau_float_parts(raw: [u8; 40]) -> (i32, [u8; 31]) {
    let magnitude = bits_to_i32(&raw[31..39]);
    let exponent = if raw[39] == 0 {
        magnitude
    } else if magnitude == 0 {
        -256
    } else {
        -magnitude
    };
    let mantissa = std::array::from_fn(|bit| match bit {
        0..=18 => raw[bit],
        19..=29 => raw[bit + 1],
        _ => raw[19],
    });
    (exponent, mantissa)
}

fn aau_exponent_bits(exponent: i32) -> [u8; 9] {
    let encoded = if (0..=255).contains(&exponent) {
        exponent
    } else if (-256..=-1).contains(&exponent) {
        0x100 | (-exponent & 0xff)
    } else if exponent < -256 {
        (-exponent) & 0xff
    } else {
        0x100 | ((exponent - 256) & 0xff)
    };
    i32_to_bits(encoded)
}

fn aau_float_raw(exponent: i32, mantissa: [u8; 31]) -> [u8; 40] {
    let exponent = aau_exponent_bits(exponent);
    std::array::from_fn(|bit| match bit {
        0..=18 => mantissa[bit],
        19 => mantissa[30],
        20..=30 => mantissa[bit - 1],
        _ => exponent[bit - 31],
    })
}

fn aau_float_pair_parts(ax: [u8; 40], qx: [u8; 40]) -> (i32, [u8; 61]) {
    let (exponent, ax_mantissa) = aau_float_parts(ax);
    let (_, qx_mantissa) = aau_float_parts(qx);
    let pair = std::array::from_fn(|bit| {
        if bit < 30 {
            qx_mantissa[bit]
        } else {
            ax_mantissa[bit - 30]
        }
    });
    (exponent, pair)
}

fn aau_float_pair_raw(exponent: i32, mantissa: [u8; 61]) -> ([u8; 40], [u8; 40]) {
    let ax_mantissa: [u8; 31] = std::array::from_fn(|bit| mantissa[bit + 30]);
    let qx_mantissa: [u8; 31] = std::array::from_fn(|bit| {
        if bit < 30 {
            mantissa[bit]
        } else {
            mantissa[60]
        }
    });
    (
        aau_float_raw(exponent, ax_mantissa),
        aau_float_raw(gate_i32_subtract(exponent, 30), qx_mantissa),
    )
}

fn shift_left_bits<const WIDTH: usize>(value: [u8; WIDTH], count: usize) -> [u8; WIDTH] {
    if count >= WIDTH {
        return [0; WIDTH];
    }
    std::array::from_fn(|bit| if bit >= count { value[bit - count] } else { 0 })
}

fn shift_right_bits<const WIDTH: usize>(value: [u8; WIDTH], count: usize) -> [u8; WIDTH] {
    if count >= WIDTH {
        return [0; WIDTH];
    }
    std::array::from_fn(|bit| {
        if bit + count < WIDTH {
            value[bit + count]
        } else {
            0
        }
    })
}

fn arithmetic_shift_right_bits<const WIDTH: usize>(
    value: [u8; WIDTH],
    count: usize,
) -> [u8; WIDTH] {
    let sign = value[WIDTH - 1];
    if count >= WIDTH {
        return [sign; WIDTH];
    }
    std::array::from_fn(|bit| {
        if bit + count < WIDTH {
            value[bit + count]
        } else {
            sign
        }
    })
}

fn fits_signed_width<const WIDTH: usize>(value: &[u8; WIDTH], bits: usize) -> bool {
    value[bits..].iter().all(|bit| *bit == value[bits - 1])
}

fn apply_sign<const WIDTH: usize>(magnitude: [u8; WIDTH], sign: u8) -> [u8; WIDTH] {
    mux_bits(sign, magnitude, gate_twos_complement(magnitude))
}

fn gate_signed_multiply<const WIDTH: usize, const OUT: usize>(
    left: [u8; WIDTH],
    right: [u8; WIDTH],
) -> [u8; OUT] {
    let left_magnitude = gate_absolute(left);
    let right_magnitude = gate_absolute(right);
    let mut product = [0; OUT];
    for multiplier_bit in 0..WIDTH {
        let partial = std::array::from_fn(|bit| {
            if bit >= multiplier_bit && bit - multiplier_bit < WIDTH {
                and_gate(
                    left_magnitude[bit - multiplier_bit],
                    right_magnitude[multiplier_bit],
                )
            } else {
                0
            }
        });
        product = gate_add(product, partial).0;
    }
    apply_sign(product, xor_gate(left[WIDTH - 1], right[WIDTH - 1]))
}

fn gate_unsigned_divide_64(dividend: [u8; 64], divisor: [u8; 64]) -> Option<([u8; 64], [u8; 64])> {
    if is_zero(&divisor) == 1 {
        return None;
    }
    let divisor_wide = zero_extend::<64, 65>(divisor);
    let mut quotient = [0; 64];
    let mut remainder = [0; 65];
    for dividend_bit in (0..64).rev() {
        remainder = shift_left_bits(remainder, 1);
        remainder[0] = dividend[dividend_bit];
        let subtract = greater_or_equal(&remainder, &divisor_wide);
        remainder = mux_bits(
            subtract,
            remainder,
            gate_subtract(remainder, divisor_wide).0,
        );
        quotient[dividend_bit] = subtract;
    }
    Some((
        quotient,
        remainder[..64]
            .try_into()
            .expect("a divide remainder is smaller than its 64-bit divisor"),
    ))
}

fn gate_aau_fixed_divide(dividend: [u8; 77], divisor: [u8; 39]) -> Option<([u8; 39], [u8; 39])> {
    if is_zero(&divisor) == 1 {
        return None;
    }
    let dividend_sign = dividend[76];
    let divisor_sign = divisor[38];
    let dividend_magnitude = gate_absolute(dividend);
    let divisor_wide = zero_extend::<39, 78>(gate_absolute(divisor));
    let mut quotient_wide = [0; 77];
    let mut remainder = [0; 78];
    for dividend_bit in (0..77).rev() {
        remainder = shift_left_bits(remainder, 1);
        remainder[0] = dividend_magnitude[dividend_bit];
        let subtract = greater_or_equal(&remainder, &divisor_wide);
        remainder = mux_bits(
            subtract,
            remainder,
            gate_subtract(remainder, divisor_wide).0,
        );
        quotient_wide[dividend_bit] = subtract;
    }
    let quotient_magnitude: [u8; 39] = quotient_wide[..39]
        .try_into()
        .expect("the preflight-bounded AAU quotient fits its register");
    let remainder_magnitude: [u8; 39] = remainder[..39]
        .try_into()
        .expect("the AAU remainder fits its divisor width");
    Some((
        apply_sign(quotient_magnitude, xor_gate(dividend_sign, divisor_sign)),
        apply_sign(remainder_magnitude, dividend_sign),
    ))
}

fn gate_i32_add(left: i32, right: i32) -> i32 {
    bits_to_i32(&gate_add(i32_to_bits::<32>(left), i32_to_bits::<32>(right)).0)
}

fn gate_i32_subtract(left: i32, right: i32) -> i32 {
    bits_to_i32(&gate_subtract(i32_to_bits::<32>(left), i32_to_bits::<32>(right)).0)
}

fn gate_add<const WIDTH: usize>(left: [u8; WIDTH], right: [u8; WIDTH]) -> ([u8; WIDTH], u8) {
    let result = ripple_carry_adder_with_carry(&left, &right, 0);
    let sum: [u8; WIDTH] = result.sum.try_into().expect("ripple adder preserves width");
    let overflow = and_gate(
        not_gate(xor_gate(left[WIDTH - 1], right[WIDTH - 1])),
        xor_gate(left[WIDTH - 1], sum[WIDTH - 1]),
    );
    (sum, overflow)
}

fn gate_subtract<const WIDTH: usize>(left: [u8; WIDTH], right: [u8; WIDTH]) -> ([u8; WIDTH], u8) {
    let inverted: [u8; WIDTH] = std::array::from_fn(|bit| not_gate(right[bit]));
    let result = ripple_carry_adder_with_carry(&left, &inverted, 1);
    let difference: [u8; WIDTH] = result.sum.try_into().expect("ripple adder preserves width");
    let overflow = and_gate(
        xor_gate(left[WIDTH - 1], right[WIDTH - 1]),
        xor_gate(left[WIDTH - 1], difference[WIDTH - 1]),
    );
    (difference, overflow)
}

fn join_double(a: [u8; 20], q: [u8; 20]) -> [u8; 39] {
    std::array::from_fn(|bit| if bit < 19 { q[bit] } else { a[bit - 19] })
}

fn split_double(value: [u8; 39]) -> ([u8; 20], [u8; 20]) {
    let a = std::array::from_fn(|bit| value[bit + 19]);
    let mut q = [0; 20];
    q[..19].copy_from_slice(&value[..19]);
    q[19] = a[19];
    (a, q)
}

fn with_sign_bits(data: [u8; 19], sign: u8) -> [u8; 20] {
    std::array::from_fn(|bit| if bit == 19 { sign } else { data[bit] })
}

fn gate_twos_complement<const WIDTH: usize>(value: [u8; WIDTH]) -> [u8; WIDTH] {
    gate_subtract([0; WIDTH], value).0
}

fn gate_absolute<const WIDTH: usize>(value: [u8; WIDTH]) -> [u8; WIDTH] {
    let negative = value[WIDTH - 1];
    let complemented = gate_twos_complement(value);
    std::array::from_fn(|bit| {
        or_gate(
            and_gate(not_gate(negative), value[bit]),
            and_gate(negative, complemented[bit]),
        )
    })
}

fn gate_multiply_add(q: [u8; 20], operand: [u8; 20], a: [u8; 20]) -> ([u8; 39], u8) {
    let q_magnitude = gate_absolute(q);
    let operand_magnitude = gate_absolute(operand);
    let mut product = [0; 39];
    for multiplier_bit in 0..19 {
        let partial = std::array::from_fn(|bit| {
            if bit >= multiplier_bit && bit - multiplier_bit < 19 {
                and_gate(
                    q_magnitude[bit - multiplier_bit],
                    operand_magnitude[multiplier_bit],
                )
            } else {
                0
            }
        });
        product = gate_add(product, partial).0;
    }
    let product_sign = xor_gate(q[19], operand[19]);
    let negative_product = gate_twos_complement(product);
    product = std::array::from_fn(|bit| {
        or_gate(
            and_gate(not_gate(product_sign), product[bit]),
            and_gate(product_sign, negative_product[bit]),
        )
    });
    let a_extended = std::array::from_fn(|bit| if bit < 20 { a[bit] } else { a[19] });
    gate_add(product, a_extended)
}

fn gate_divide(dividend: [u8; 39], divisor: [u8; 20]) -> Option<([u8; 20], [u8; 20])> {
    let divisor_magnitude = gate_absolute(divisor);
    if is_zero(&divisor_magnitude) == 1 {
        return None;
    }
    let high: [u8; 20] = dividend[19..39]
        .try_into()
        .expect("the GE-225 double high half is 20 bits");
    if greater_or_equal(&gate_absolute(high), &divisor_magnitude) == 1 {
        return None;
    }

    let dividend_sign = dividend[38];
    let divisor_sign = divisor[19];
    let dividend_magnitude = gate_absolute(dividend);
    let divisor_wide: [u8; 21] =
        std::array::from_fn(|bit| if bit < 20 { divisor_magnitude[bit] } else { 0 });
    let mut remainder = [0; 21];
    let mut quotient_wide = [0; 39];
    for dividend_bit in (0..39).rev() {
        for bit in (1..21).rev() {
            remainder[bit] = remainder[bit - 1];
        }
        remainder[0] = dividend_magnitude[dividend_bit];
        let subtract = greater_or_equal(&remainder, &divisor_wide);
        let difference = gate_subtract(remainder, divisor_wide).0;
        remainder = std::array::from_fn(|bit| {
            or_gate(
                and_gate(not_gate(subtract), remainder[bit]),
                and_gate(subtract, difference[bit]),
            )
        });
        quotient_wide[dividend_bit] = subtract;
    }

    let quotient_magnitude: [u8; 20] = quotient_wide[..20]
        .try_into()
        .expect("the preflight-bounded GE-225 quotient is 20 bits");
    let remainder_magnitude: [u8; 20] = remainder[..20]
        .try_into()
        .expect("the GE-225 divide remainder is 20 bits");
    let result_sign = xor_gate(dividend_sign, divisor_sign);
    let negative_quotient = gate_twos_complement(quotient_magnitude);
    let negative_remainder = gate_twos_complement(remainder_magnitude);
    let quotient = std::array::from_fn(|bit| {
        or_gate(
            and_gate(not_gate(result_sign), quotient_magnitude[bit]),
            and_gate(result_sign, negative_quotient[bit]),
        )
    });
    let remainder = std::array::from_fn(|bit| {
        or_gate(
            and_gate(not_gate(result_sign), remainder_magnitude[bit]),
            and_gate(result_sign, negative_remainder[bit]),
        )
    });
    Some((quotient, remainder))
}

fn greater_or_equal<const WIDTH: usize>(left: &[u8; WIDTH], right: &[u8; WIDTH]) -> u8 {
    let mut greater = 0;
    let mut equal = 1;
    for bit in (0..WIDTH).rev() {
        greater = or_gate(
            greater,
            and_gate(equal, and_gate(left[bit], not_gate(right[bit]))),
        );
        equal = and_gate(equal, not_gate(xor_gate(left[bit], right[bit])));
    }
    or_gate(greater, equal)
}

fn signed_compare<const WIDTH: usize>(left: [u8; WIDTH], right: [u8; WIDTH]) -> i8 {
    if left == right {
        return 0;
    }
    let left_sign = left[WIDTH - 1];
    let right_sign = right[WIDTH - 1];
    if left_sign != right_sign {
        return if left_sign == 1 { -1 } else { 1 };
    }
    for bit in (0..WIDTH - 1).rev() {
        if left[bit] != right[bit] {
            let greater = if left[bit] == 1 { 1 } else { -1 };
            return if left_sign == 1 { -greater } else { greater };
        }
    }
    0
}

fn is_zero<const WIDTH: usize>(bits: &[u8; WIDTH]) -> u8 {
    bits.iter()
        .fold(1, |zero, bit| and_gate(zero, not_gate(*bit)))
}

fn equal_bits<const WIDTH: usize>(left: &[u8; WIDTH], right: &[u8; WIDTH]) -> u8 {
    left.iter().zip(right).fold(1, |equal, (a, b)| {
        and_gate(equal, not_gate(xor_gate(*a, *b)))
    })
}

fn decode_bits<const WIDTH: usize>(bits: [u8; WIDTH]) -> Vec<u8> {
    (0..(1usize << WIDTH))
        .map(|value| equal_bits(&bits, &i32_to_bits::<WIDTH>(value as i32)))
        .collect()
}

fn i32_to_bits<const WIDTH: usize>(value: i32) -> [u8; WIDTH] {
    std::array::from_fn(|bit| ((value >> bit) & 1) as u8)
}

fn u64_to_bits<const WIDTH: usize>(value: u64) -> [u8; WIDTH] {
    std::array::from_fn(|bit| {
        if bit < 64 {
            ((value >> bit) & 1) as u8
        } else {
            0
        }
    })
}

fn bits_to_i32(bits: &[u8]) -> i32 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, input)| value | i32::from(*input) << bit)
}

fn bits_to_u64(bits: &[u8]) -> u64 {
    bits.iter()
        .enumerate()
        .fold(0, |value, (bit, input)| value | u64::from(*input) << bit)
}

pub fn encode_instruction(opcode: i32, modifier: i32, address: i32) -> Option<i32> {
    if !(0..=0o37).contains(&opcode)
        || !(0..=3).contains(&modifier)
        || !(0..=ADDRESS_MASK).contains(&address)
    {
        return None;
    }
    Some((opcode << 15) | (modifier << 13) | address)
}

pub fn assemble_aau_general(mnemonic: &str) -> Result<i32, String> {
    Ok(match mnemonic {
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
        _ => {
            return Err(format!(
                "unknown GE-225 AAU general instruction: {mnemonic}"
            ))
        }
    })
}

pub fn assemble_aau_memory(mnemonic: &str, address: i32, modifier: i32) -> Result<i32, String> {
    let opcode = match mnemonic {
        "FLD" => 0o30,
        "FAD" => 0o31,
        "FSU" => 0o32,
        "FST" => 0o33,
        "FMP" => 0o35,
        "FDV" => 0o36,
        _ => return Err(format!("unknown GE-225 AAU memory instruction: {mnemonic}")),
    };
    encode_instruction(opcode, modifier, address)
        .ok_or_else(|| format!("invalid GE-225 AAU memory operand: {address:o}/{modifier}"))
}

pub fn assemble_aau_branch(mnemonic: &str) -> Result<i32, String> {
    Ok(match mnemonic {
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
        _ => return Err(format!("unknown GE-225 AAU branch instruction: {mnemonic}")),
    })
}

pub fn pack_aau_words(first: i32, second: i32) -> u64 {
    ((first as u64 & WORD_MASK as u64) << 20) | (second as u64 & WORD_MASK as u64)
}

pub fn unpack_aau_words(value: u64) -> (i32, i32) {
    (
        ((value >> 20) & WORD_MASK as u64) as i32,
        (value & WORD_MASK as u64) as i32,
    )
}
