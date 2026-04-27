const MASK_20: i32 = (1 << 20) - 1;
const DATA_MASK: i32 = (1 << 19) - 1;
const SIGN_BIT: i32 = 1 << 19;
const ADDR_MASK: i32 = 0x1fff;
const X_MASK: i32 = 0x7fff;
const N_MASK: i32 = 0x3f;
const WORD_BYTES: usize = 3;
const MAX_X_GROUPS: usize = 32;

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
const OP_MOY: i32 = 0o24;
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
    pub automatic_interrupt_mode: bool,
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
        OP_MOY => "MOY",
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
        "SET_DECMODE" => 0o2506011,
        "SET_BINMODE" => 0o2506012,
        "SXG" => 0o2506013,
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
        0o2506011 => "SET_DECMODE",
        0o2506012 => "SET_BINMODE",
        0o2506013 => "SXG",
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
        0o00 => "0", 0o01 => "1", 0o02 => "2", 0o03 => "3", 0o04 => "4", 0o05 => "5",
        0o06 => "6", 0o07 => "7", 0o10 => "8", 0o11 => "9", 0o13 => "/", 0o21 => "A",
        0o22 => "B", 0o23 => "C", 0o24 => "D", 0o25 => "E", 0o26 => "F", 0o27 => "G",
        0o30 => "H", 0o31 => "I", 0o33 => "-", 0o40 => ".", 0o41 => "J", 0o42 => "K",
        0o43 => "L", 0o44 => "M", 0o45 => "N", 0o46 => "O", 0o47 => "P", 0o50 => "Q",
        0o51 => "R", 0o53 => "$", 0o60 => " ", 0o62 => "S", 0o63 => "T", 0o64 => "U",
        0o65 => "V", 0o66 => "W", 0o67 => "X", 0o70 => "Y", 0o71 => "Z",
        _ => return None,
    })
}

fn to_signed20(value: i32) -> i32 {
    let word = value & MASK_20;
    if (word & SIGN_BIT) != 0 { word - (1 << 20) } else { word }
}

fn from_signed20(value: i32) -> i32 { value & MASK_20 }
fn sign_of(word: i32) -> i32 { if (word & SIGN_BIT) != 0 { 1 } else { 0 } }
fn with_sign(word: i32, sign: i32) -> i32 { ((sign & 1) << 19) | (word & DATA_MASK) }

fn combine_words(high: i32, low: i32) -> i64 {
    ((high & MASK_20) as i64) << 20 | ((low & MASK_20) as i64)
}

fn to_signed40(value: i64) -> i64 {
    (value << 24) >> 24
}

fn split_signed40(value: i64) -> (i32, i32) {
    let raw = value & ((1_i64 << 40) - 1);
    (((raw >> 20) & MASK_20 as i64) as i32, (raw & MASK_20 as i64) as i32)
}

fn arith_compare(left: i32, right: i32) -> i32 {
    match to_signed20(left).cmp(&to_signed20(right)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 0,
    }
}

fn arith_compare_double(left_high: i32, left_low: i32, right_high: i32, right_low: i32) -> i32 {
    match to_signed40(combine_words(left_high, left_low)).cmp(&to_signed40(combine_words(right_high, right_low))) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 0,
    }
}

pub fn encode_instruction(opcode: i32, modifier: i32, address: i32) -> Result<i32, String> {
    if !(0..=0o37).contains(&opcode) { return Err(format!("opcode out of range: {opcode}")); }
    if !(0..=0o3).contains(&modifier) { return Err(format!("modifier out of range: {modifier}")); }
    if !(0..=ADDR_MASK).contains(&address) { return Err(format!("address out of range: {address}")); }
    Ok(((opcode & 0x1f) << 15) | ((modifier & 0x03) << 13) | (address & ADDR_MASK))
}

pub fn decode_instruction(word: i32) -> (i32, i32, i32) {
    let normalized = word & MASK_20;
    ((normalized >> 15) & 0x1f, (normalized >> 13) & 0x03, normalized & ADDR_MASK)
}

pub fn assemble_fixed(mnemonic: &str) -> Result<i32, String> {
    fixed_word(mnemonic).ok_or_else(|| format!("unknown fixed GE-225 instruction: {mnemonic}"))
}

pub fn assemble_shift(mnemonic: &str, count: i32) -> Result<i32, String> {
    if !(0..=0o37).contains(&count) { return Err(format!("shift count out of range: {count}")); }
    shift_base(mnemonic)
        .map(|base| base | count)
        .ok_or_else(|| format!("unknown GE-225 shift instruction: {mnemonic}"))
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
    if program.len() % WORD_BYTES != 0 {
        return Err(format!("GE-225 byte stream must be a multiple of {WORD_BYTES} bytes, got {}", program.len()));
    }
    Ok(program
        .chunks_exact(WORD_BYTES)
        .map(|chunk| (((chunk[0] as i32) << 16) | ((chunk[1] as i32) << 8) | chunk[2] as i32) & MASK_20)
        .collect())
}

pub struct Simulator {
    memory_size: i32,
    memory: Vec<i32>,
    card_reader_queue: Vec<Vec<i32>>,
    a: i32,
    q: i32,
    m: i32,
    n: i32,
    pc: i32,
    ir: i32,
    overflow: bool,
    parity_error: bool,
    decimal_mode: bool,
    automatic_interrupt_mode: bool,
    selected_x_group: usize,
    n_ready: bool,
    typewriter_power: bool,
    typewriter_output: Vec<String>,
    control_switches: i32,
    halted: bool,
    x_groups: [[i32; 4]; MAX_X_GROUPS],
}

impl Simulator {
    pub fn new(memory_words: i32) -> Self {
        assert!(memory_words > 0, "memory_words must be positive");
        Self {
            memory_size: memory_words,
            memory: vec![0; memory_words as usize],
            card_reader_queue: vec![],
            a: 0, q: 0, m: 0, n: 0, pc: 0, ir: 0,
            overflow: false, parity_error: false, decimal_mode: false, automatic_interrupt_mode: false,
            selected_x_group: 0, n_ready: true, typewriter_power: false, typewriter_output: vec![],
            control_switches: 0, halted: false, x_groups: [[0; 4]; MAX_X_GROUPS],
        }
    }

    pub fn reset(&mut self) {
        self.a = 0; self.q = 0; self.m = 0; self.n = 0; self.pc = 0; self.ir = 0;
        self.overflow = false; self.parity_error = false; self.decimal_mode = false; self.automatic_interrupt_mode = false;
        self.selected_x_group = 0; self.n_ready = true; self.typewriter_power = false; self.typewriter_output.clear();
        self.control_switches = 0; self.halted = false; self.x_groups = [[0; 4]; MAX_X_GROUPS];
    }

    pub fn get_state(&self) -> State {
        State {
            a: self.a, q: self.q, m: self.m, n: self.n, pc: self.pc, ir: self.ir,
            indicators: Indicators {
                carry: self.overflow, zero: self.a == 0, negative: (self.a & SIGN_BIT) != 0,
                overflow: self.overflow, parity_error: self.parity_error,
            },
            overflow: self.overflow, parity_error: self.parity_error, decimal_mode: self.decimal_mode,
            automatic_interrupt_mode: self.automatic_interrupt_mode, selected_x_group: self.selected_x_group,
            n_ready: self.n_ready, typewriter_power: self.typewriter_power, control_switches: self.control_switches,
            x_words: self.x_groups[self.selected_x_group].to_vec(), halted: self.halted, memory: self.memory.clone(),
        }
    }

    pub fn set_control_switches(&mut self, value: i32) { self.control_switches = value & MASK_20; }
    pub fn queue_card_reader_record(&mut self, words: &[i32]) { self.card_reader_queue.push(words.iter().map(|w| w & MASK_20).collect()); }
    pub fn get_typewriter_output(&self) -> String { self.typewriter_output.join("") }
    pub fn load_words(&mut self, words: &[i32], start_address: i32) -> Result<(), String> {
        for (offset, word) in words.iter().enumerate() { self.write_word(start_address + offset as i32, *word)?; }
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
            return Ok(match decoded.count { Some(count) => format!("{} {count}", decoded.mnemonic), None => decoded.mnemonic.to_string() });
        }
        Ok(format!("{} 0x{:03X},X{}", decoded.mnemonic, decoded.address.unwrap(), decoded.modifier.unwrap()))
    }

    pub fn step(&mut self) -> Result<Trace, String> {
        if self.halted { return Err("cannot step a halted GE-225 simulator".into()); }
        let pc_before = self.pc;
        self.ir = self.read_word(self.pc)?;
        self.pc = (self.pc + 1) % self.memory_size;
        let decoded = self.decode_word(self.ir)?;
        let a_before = self.a;
        let q_before = self.q;
        let mut effective_address = None;
        if !decoded.fixed_word {
            let address = decoded.address.unwrap();
            if !matches!(decoded.mnemonic, "BXL" | "BXH" | "LDX" | "SPB" | "INX" | "STX" | "MOY") {
                effective_address = Some(self.resolve_effective_address(address, decoded.modifier.unwrap()));
            }
            self.execute_memory_reference(decoded.mnemonic, decoded.modifier.unwrap(), effective_address.unwrap_or(address), address, pc_before)?;
        } else {
            self.execute_fixed(&decoded)?;
        }
        Ok(Trace {
            address: pc_before,
            instruction_word: self.ir,
            mnemonic: self.disassemble_word(self.ir)?,
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
            if self.halted { break; }
            traces.push(self.step()?);
        }
        Ok(traces)
    }

    fn get_x_word(&self, slot: usize) -> i32 { self.x_groups[self.selected_x_group][slot] & X_MASK }
    fn set_x_word(&mut self, slot: usize, value: i32) { self.x_groups[self.selected_x_group][slot] = value & X_MASK; }

    fn execute_memory_reference(&mut self, mnemonic: &str, modifier: i32, effective_or_raw_address: i32, raw_address: i32, pc_before: i32) -> Result<(), String> {
        let effective_address = effective_or_raw_address % self.memory_size;
        match mnemonic {
            "LDA" => { self.m = self.read_word(effective_address)?; self.a = self.m; }
            "ADD" => { self.m = self.read_word(effective_address)?; let total = to_signed20(self.a) + to_signed20(self.m); self.a = from_signed20(total); self.overflow = !(-(1 << 19)..=(1 << 19) - 1).contains(&total); }
            "SUB" => { self.m = self.read_word(effective_address)?; let total = to_signed20(self.a) - to_signed20(self.m); self.a = from_signed20(total); self.overflow = !(-(1 << 19)..=(1 << 19) - 1).contains(&total); }
            "STA" => self.write_word(effective_address, self.a)?,
            "BXL" => if (self.get_x_word(modifier as usize) & ADDR_MASK) >= raw_address { self.pc = (self.pc + 1) % self.memory_size; },
            "BXH" => if (self.get_x_word(modifier as usize) & ADDR_MASK) < raw_address { self.pc = (self.pc + 1) % self.memory_size; },
            "LDX" => { let word = self.read_word(raw_address % self.memory_size)?; self.set_x_word(modifier as usize, word); }
            "SPB" => { self.set_x_word(modifier as usize, pc_before); self.pc = raw_address % self.memory_size; }
            "DLD" => {
                let first = self.read_word(effective_address)?;
                if (effective_address & 1) != 0 { self.a = first; self.q = first; }
                else { self.a = first; self.q = self.read_word((effective_address + 1) % self.memory_size)?; }
            }
            "DAD" => {
                let left = to_signed40(combine_words(self.a, self.q));
                let first = self.read_word(effective_address)?;
                let second = if (effective_address & 1) != 0 { first } else { self.read_word((effective_address + 1) % self.memory_size)? };
                let total = left + to_signed40(combine_words(first, second));
                (self.a, self.q) = split_signed40(total);
                self.overflow = !(-(1_i64 << 39)..=((1_i64 << 39) - 1)).contains(&total);
            }
            "DSU" => {
                let left = to_signed40(combine_words(self.a, self.q));
                let first = self.read_word(effective_address)?;
                let second = if (effective_address & 1) != 0 { first } else { self.read_word((effective_address + 1) % self.memory_size)? };
                let total = left - to_signed40(combine_words(first, second));
                (self.a, self.q) = split_signed40(total);
                self.overflow = !(-(1_i64 << 39)..=((1_i64 << 39) - 1)).contains(&total);
            }
            "DST" => {
                if (effective_address & 1) != 0 { self.write_word(effective_address, self.q)?; }
                else { self.write_word(effective_address, self.a)?; self.write_word((effective_address + 1) % self.memory_size, self.q)?; }
            }
            "INX" => self.set_x_word(modifier as usize, (self.get_x_word(modifier as usize) + raw_address) & X_MASK),
            "MPY" => {
                self.m = self.read_word(effective_address)?;
                let product = i64::from(to_signed20(self.q)) * i64::from(to_signed20(self.m)) + i64::from(to_signed20(self.a));
                (self.a, self.q) = split_signed40(product);
                self.overflow = !(-(1_i64 << 39)..=((1_i64 << 39) - 1)).contains(&product);
            }
            "DVD" => {
                self.m = self.read_word(effective_address)?;
                let divisor = i64::from(to_signed20(self.m));
                if divisor == 0 { return Err("GE-225 divide by zero".into()); }
                if i64::from(to_signed20(self.a).abs()) >= divisor.abs() { self.overflow = true; return Ok(()); }
                let dividend = to_signed40(combine_words(self.a, self.q));
                let quotient_mag = dividend.abs() / divisor.abs();
                let remainder_mag = dividend.abs() % divisor.abs();
                let quotient = if (dividend < 0) ^ (divisor < 0) { -quotient_mag } else { quotient_mag };
                let remainder = if quotient < 0 { -remainder_mag } else { remainder_mag };
                self.a = from_signed20(quotient as i32);
                self.q = from_signed20(remainder as i32);
                self.overflow = !(-(1_i64 << 19)..=((1_i64 << 19) - 1)).contains(&quotient);
            }
            "STX" => self.write_word(raw_address % self.memory_size, self.get_x_word(modifier as usize))?,
            "EXT" => { self.m = self.read_word(effective_address)?; self.a &= (!self.m) & MASK_20; }
            "CAB" => {
                self.m = self.read_word(effective_address)?;
                match arith_compare(self.m, self.a) {
                    0 => self.pc = (self.pc + 1) % self.memory_size,
                    x if x < 0 => self.pc = (self.pc + 2) % self.memory_size,
                    _ => {}
                }
            }
            "DCB" => {
                let first = self.read_word(effective_address)?;
                let second = if (effective_address & 1) != 0 { first } else { self.read_word((effective_address + 1) % self.memory_size)? };
                match arith_compare_double(first, second, self.a, self.q) {
                    0 => self.pc = (self.pc + 1) % self.memory_size,
                    x if x < 0 => self.pc = (self.pc + 2) % self.memory_size,
                    _ => {}
                }
            }
            "ORY" => {
                let word = self.read_word(effective_address)?;
                self.write_word(effective_address, word | self.a)?;
            }
            "MOY" => {
                let word_count = (-to_signed20(self.q)).max(0);
                let destination = self.a & X_MASK;
                for offset in 0..word_count { let word = self.read_word((raw_address + offset) % self.memory_size)?; self.write_word((destination + offset) % self.memory_size, word)?; }
                self.set_x_word(0, self.pc);
                self.a = 0;
            }
            "RCD" => {
                if self.card_reader_queue.is_empty() { return Err("RCD executed with no queued card-reader record".into()); }
                let record = self.card_reader_queue.remove(0);
                for (offset, word) in record.iter().enumerate() { self.write_word((effective_address + offset as i32) % self.memory_size, *word)?; }
            }
            "BRU" => self.pc = effective_address,
            "STO" => {
                let existing = self.read_word(effective_address)?;
                self.write_word(effective_address, (existing & !ADDR_MASK) | (self.a & ADDR_MASK))?;
            }
            _ => return Err(format!("unimplemented GE-225 memory-reference instruction: {mnemonic}")),
        }
        Ok(())
    }

    fn execute_fixed(&mut self, decoded: &DecodedInstruction) -> Result<(), String> {
        let mnemonic = decoded.mnemonic;
        let count = decoded.count.unwrap_or(0);
        match mnemonic {
            "OFF" => { self.typewriter_power = false; self.n_ready = true; }
            "TYP" => {
                if !self.typewriter_power { self.n_ready = false; return Ok(()); }
                let code = self.n & N_MASK;
                if code == 0o37 { self.typewriter_output.push("\r".into()); }
                else if code == 0o76 { self.typewriter_output.push("\t".into()); }
                else if code != 0o72 && code != 0o75 {
                    let ch = typewriter_char(code).ok_or_else(|| "invalid typewriter code".to_string())?;
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
            "NEG" => { let before = to_signed20(self.a); self.a = from_signed20(-before); self.overflow = before == -(1 << 19); }
            "CHS" => self.a ^= SIGN_BIT,
            "NOP" => {}
            "LAQ" => self.a = self.q,
            "LQA" => self.q = self.a,
            "XAQ" => std::mem::swap(&mut self.a, &mut self.q),
            "MAQ" => { self.q = self.a; self.a = 0; }
            "ADO" => { let total = to_signed20(self.a) + 1; self.a = from_signed20(total); self.overflow = !(-(1 << 19)..=(1 << 19) - 1).contains(&total); }
            "SBO" => { let total = to_signed20(self.a) - 1; self.a = from_signed20(total); self.overflow = !(-(1 << 19)..=(1 << 19) - 1).contains(&total); }
            "SET_DECMODE" => self.decimal_mode = true,
            "SET_BINMODE" => self.decimal_mode = false,
            "SXG" => self.selected_x_group = (self.a & 0x1f) as usize,
            "SET_PST" => self.automatic_interrupt_mode = true,
            "SET_PBK" => self.automatic_interrupt_mode = false,
            "BOD" | "BEV" | "BMI" | "BPL" | "BZE" | "BNZ" | "BOV" | "BNO" | "BPE" | "BPC" | "BNR" | "BNN" => self.execute_branch_test(mnemonic),
            _ => {
                if shift_base(mnemonic).is_some() { self.execute_shift(mnemonic, count); }
                else { return Err(format!("unimplemented GE-225 fixed instruction: {mnemonic}")); }
            }
        }
        Ok(())
    }

    fn execute_branch_test(&mut self, mnemonic: &str) {
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
        if matches!(mnemonic, "BOV" | "BNO") { self.overflow = false; }
        if matches!(mnemonic, "BPE" | "BPC") { self.parity_error = false; }
        if !cond { self.pc = (self.pc + 1) % self.memory_size; }
    }

    fn execute_shift(&mut self, mnemonic: &str, count: i32) {
        if count == 0 {
            if mnemonic == "SRD" { self.q = with_sign(self.q, sign_of(self.a)); }
            else if mnemonic == "SLD" { self.a = with_sign(self.a, sign_of(self.q)); }
            return;
        }
        let a_sign = sign_of(self.a);
        let mut a_data = self.a & DATA_MASK;
        let q_sign = sign_of(self.q);
        let mut q_data = self.q & DATA_MASK;
        match mnemonic {
            "SRA" => self.a = from_signed20(to_signed20(self.a) >> count.min(19)),
            "SLA" => { self.overflow = (a_data >> (19 - count).max(0)) != 0; self.a = with_sign((a_data << count) & DATA_MASK, a_sign); }
            "SCA" => { let rotation = count % 19; if rotation != 0 { a_data = ((a_data >> rotation) | (a_data << (19 - rotation))) & DATA_MASK; } self.a = with_sign(a_data, a_sign); }
            "SAN" => {
                let fill = if a_sign == 1 { (1 << count) - 1 } else { 0 };
                let mut combined = ((a_data & DATA_MASK) << 6) | (self.n & N_MASK);
                combined = ((fill << 25) | combined) >> count;
                self.a = with_sign((combined >> 6) & DATA_MASK, a_sign);
                self.n = combined & N_MASK;
            }
            "SNA" => {
                let combined = (((self.n & N_MASK) << 19) | a_data) >> count;
                self.n = (combined >> 19) & N_MASK;
                self.a = with_sign(combined & DATA_MASK, a_sign);
            }
            "SRD" => {
                let value = combine_words(self.a, self.q) >> count;
                self.a = with_sign(((value >> 20) as i32) & DATA_MASK, a_sign);
                self.q = with_sign((value as i32) & DATA_MASK, a_sign);
            }
            "NAQ" => {
                let combined = ((((self.n & N_MASK) as i64) << 38) | (((a_data & DATA_MASK) as i64) << 19) | (q_data as i64)) >> count;
                self.n = ((combined >> 38) as i32) & N_MASK;
                self.a = with_sign(((combined >> 19) as i32) & DATA_MASK, a_sign);
                self.q = with_sign((combined as i32) & DATA_MASK, a_sign);
            }
            "SCD" => {
                let rotation = count % 38;
                let mut combined = (((a_data & DATA_MASK) as i64) << 19) | (q_data as i64);
                if rotation != 0 { combined = ((combined >> rotation) | (combined << (38 - rotation))) & ((1_i64 << 38) - 1); }
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
            }
            "SLD" => {
                let mut combined = (((a_data & DATA_MASK) as i64) << 19) | (q_data as i64);
                self.overflow = (combined >> (38 - count).max(0)) != 0;
                combined = (combined << count) & ((1_i64 << 38) - 1);
                self.a = with_sign(((combined >> 19) as i32) & DATA_MASK, q_sign);
                self.q = with_sign((combined as i32) & DATA_MASK, q_sign);
            }
            "NOR" => {
                let mut shifts = 0;
                let target_bit = if a_sign == 0 { 0 } else { 1 };
                while shifts < count {
                    let lead = (a_data >> 18) & 1;
                    if lead != target_bit { break; }
                    self.overflow |= lead == 1;
                    a_data = (a_data << 1) & DATA_MASK;
                    shifts += 1;
                }
                self.a = with_sign(a_data, a_sign);
                self.set_x_word(0, count - shifts);
            }
            "DNO" => {
                let mut shifts = 0;
                let target_bit = if a_sign == 0 { 0 } else { 1 };
                let mut combined = (((a_data & DATA_MASK) as i64) << 19) | (q_data as i64);
                while shifts < count {
                    let lead = (combined >> 37) & 1_i64;
                    if lead != target_bit { break; }
                    self.overflow |= lead == 1;
                    combined = (combined << 1) & ((1_i64 << 38) - 1);
                    shifts += 1;
                }
                self.a = with_sign(((combined >> 19) as i32) & DATA_MASK, q_sign);
                self.q = with_sign((combined as i32) & DATA_MASK, q_sign);
                self.set_x_word(0, count - shifts);
            }
            _ => {}
        }
    }

    fn decode_word(&self, word: i32) -> Result<DecodedInstruction, String> {
        let normalized = word & MASK_20;
        if let Some(name) = fixed_name(normalized) {
            return Ok(DecodedInstruction { mnemonic: name, modifier: None, address: None, count: None, fixed_word: true });
        }
        for name in ["SRA", "SNA", "SCA", "SAN", "SRD", "NAQ", "SCD", "ANQ", "SLA", "SLD", "NOR", "DNO"] {
            if let Some(base) = shift_base(name) {
                if (normalized & !0o37) == base {
                    return Ok(DecodedInstruction { mnemonic: name, modifier: None, address: None, count: Some(normalized & 0o37), fixed_word: true });
                }
            }
        }
        let (opcode, modifier, address) = decode_instruction(normalized);
        let mnemonic = base_opcode_name(opcode).ok_or_else(|| format!("unknown GE-225 opcode field {opcode:o}"))?;
        Ok(DecodedInstruction { mnemonic, modifier: Some(modifier), address: Some(address), count: None, fixed_word: false })
    }

    fn resolve_effective_address(&self, address: i32, modifier: i32) -> i32 {
        let base = address % self.memory_size;
        if modifier == 0 { return base; }
        (base + (self.get_x_word(modifier as usize) % self.memory_size)) % self.memory_size
    }

    fn check_address(&self, address: i32) -> Result<(), String> {
        if address < 0 || address >= self.memory_size { Err(format!("address out of range: {address}")) } else { Ok(()) }
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
        assert_eq!(unpack_words(&pack_words(&[word, assemble_fixed("NOP").unwrap()])).unwrap(), vec![word, assemble_fixed("NOP").unwrap()]);
    }

    #[test]
    fn lda_add_sta_program() {
        let mut sim = Simulator::new(4096);
        sim.load_words(&[ins(0o00, 10, 0), ins(0o01, 11, 0), ins(0o03, 12, 0), assemble_fixed("NOP").unwrap(), 0, 0, 0, 0, 0, 0, 1, 2, 0], 0).unwrap();
        sim.run(4).unwrap();
        let state = sim.get_state();
        assert_eq!(state.a, 3);
        assert_eq!(state.memory[12], 3);
    }

    #[test]
    fn spb_stores_p() {
        let mut sim = Simulator::new(4096);
        sim.load_words(&[ins(0o07, 4, 2), assemble_fixed("NOP").unwrap(), assemble_fixed("NOP").unwrap(), assemble_fixed("NOP").unwrap(), ins(0o00, 10, 0), assemble_fixed("NOP").unwrap(), 0, 0, 0, 0, 0x12345], 0).unwrap();
        sim.run(3).unwrap();
        let state = sim.get_state();
        assert_eq!(state.x_words[2], 0);
        assert_eq!(state.a, 0x12345);
    }

    #[test]
    fn odd_address_double_ops() {
        let mut sim = Simulator::new(4096);
        sim.write_word(11, 0x13579).unwrap();
        sim.load_words(&[ins(0o10, 11, 0), ins(0o13, 13, 0), assemble_fixed("NOP").unwrap()], 0).unwrap();
        sim.run(3).unwrap();
        let state = sim.get_state();
        assert_eq!(state.a, 0x13579);
        assert_eq!(state.q, 0x13579);
        assert_eq!(state.memory[13], 0x13579);
    }

    #[test]
    fn moy_moves_blocks() {
        let mut sim = Simulator::new(4096);
        sim.write_word(20, 0x11111).unwrap();
        sim.write_word(21, 0x22222).unwrap();
        sim.write_word(30, 40).unwrap();
        sim.write_word(31, (1 << 20) - 2).unwrap();
        sim.load_words(&[ins(0o00, 30, 0), assemble_fixed("LQA").unwrap(), ins(0o00, 31, 0), assemble_fixed("XAQ").unwrap(), ins(0o24, 20, 0), assemble_fixed("NOP").unwrap()], 0).unwrap();
        sim.run(6).unwrap();
        let state = sim.get_state();
        assert_eq!(state.a, 0);
        assert_eq!(state.memory[40], 0x11111);
        assert_eq!(state.memory[41], 0x22222);
    }

    #[test]
    fn console_typewriter_path() {
        let mut sim = Simulator::new(4096);
        sim.set_control_switches(0o1633);
        sim.load_words(&[assemble_fixed("RCS").unwrap(), assemble_fixed("TON").unwrap(), assemble_shift("SAN", 6).unwrap(), assemble_fixed("TYP").unwrap(), assemble_fixed("NOP").unwrap()], 0).unwrap();
        sim.run(5).unwrap();
        assert_eq!(sim.get_typewriter_output(), "-");
        assert!(sim.get_state().typewriter_power);
    }

    #[test]
    fn rcd_loads_queued_record() {
        let mut sim = Simulator::new(4096);
        sim.queue_card_reader_record(&[0x11111, 0x22222]);
        sim.load_words(&[ins(0o25, 10, 0), assemble_fixed("NOP").unwrap()], 0).unwrap();
        sim.run(2).unwrap();
        let state = sim.get_state();
        assert_eq!(state.memory[10], 0x11111);
        assert_eq!(state.memory[11], 0x22222);
    }
}
