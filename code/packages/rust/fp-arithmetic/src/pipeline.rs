//! Pipelined floating-point arithmetic — the bridge to GPU architecture.
//!
//! # Why Pipelining?
//!
//! Imagine a car factory with a single worker who does everything: welds the
//! frame, installs the engine, paints the body, mounts the wheels, inspects
//! the result. One car takes 5 hours. Want 100 cars? That's 500 hours.
//!
//! Now imagine a factory with 5 stations, each doing one step. The first car
//! still takes 5 hours to pass through all 5 stations. But while it moves to
//! station 2, a NEW car enters station 1. After the initial 5-hour fill-up
//! time, a finished car rolls off the line every HOUR — 5x throughput!
//!
//! This is pipelining, and it's exactly how GPUs achieve massive throughput.
//!
//! # Latency vs Throughput
//!
//! ```text
//! Latency:     Time for ONE operation to complete start-to-finish.
//! Throughput:  How many operations complete per unit time.
//! ```
//!
//! For a 5-stage pipeline:
//!
//! ```text
//! Latency = 5 clock cycles (one operation still takes 5 cycles)
//! Throughput = 1 result per clock cycle (after pipeline fills up)
//! ```
//!
//! # Pipeline Timing Diagram
//!
//! ```text
//! Clock:  1    2    3    4    5    6    7    8
//! --------------------------------------------
//! Stage1: [A1] [B1] [C1] [D1]  -    -    -    -
//! Stage2:  -   [A2] [B2] [C2] [D2]  -    -    -
//! Stage3:  -    -   [A3] [B3] [C3] [D3]  -    -
//! Stage4:  -    -    -   [A4] [B4] [C4] [D4]  -
//! Stage5:  -    -    -    -   [A5] [B5] [C5] [D5]
//! ```
//!
//! # How This Connects to GPUs
//!
//! A modern GPU has thousands of CUDA cores, each containing pipelined FP units.
//! With 5000 cores each running pipelined FP at 1.5 GHz:
//!
//! ```text
//! 5000 cores x 1 result/cycle x 1.5 GHz = 7.5 TFLOPS
//! ```
//!
//! # Rust Implementation
//!
//! Unlike Go's goroutine-based approach, we use a simple vector of pipeline
//! stages advanced on each `tick()` call. Each pipeline stage is an `Option`
//! containing intermediate computation state. On each tick, stages shift
//! forward — just like hardware pipeline registers capturing data on each
//! clock edge.

use crate::formats::{FloatFormat, FloatBits, make_nan, make_inf, make_zero};
use crate::ieee754::{bits_msb_to_int, int_to_bits_msb, is_nan, is_inf, is_zero, bit_length};

// =========================================================================
// StageData — intermediate pipeline data passed between stages
// =========================================================================

/// Holds the intermediate computation state as it flows through pipeline stages.
/// In hardware, this data lives in pipeline registers — banks of D flip-flops
/// that capture values on each clock edge.
///
/// The `special` field handles bypass cases (NaN, Inf, Zero) that skip the
/// normal computation stages. When `special` is `Some`, stages simply pass
/// the data through without processing.
#[derive(Debug, Clone)]
struct StageData {
    /// If `Some`, this is a pre-computed result (NaN, Inf, zero) that bypasses
    /// the normal pipeline stages. In hardware, this is a multiplexer that
    /// selects between the normal computation path and the special-value bypass.
    special: Option<FloatBits>,

    // Normal computation fields. Different stages use different subsets.
    sign_a: u8,
    sign_b: u8,
    exp_a: i32,
    exp_b: i32,
    mant_a: u64,
    mant_b: u64,
    guard_bits: u32,
    result_sign: u8,
    result_mant: u64,
    result_exp: i32,
    product: u64,
    product_sign: u8,
    product_leading: u32,
    c_sign: u8,
    exp_c: i32,
    mant_c: u64,
    product_exp: i32,
    c_aligned: u64,
}

impl Default for StageData {
    fn default() -> Self {
        StageData {
            special: None,
            sign_a: 0, sign_b: 0,
            exp_a: 0, exp_b: 0,
            mant_a: 0, mant_b: 0,
            guard_bits: 0,
            result_sign: 0,
            result_mant: 0,
            result_exp: 0,
            product: 0,
            product_sign: 0,
            product_leading: 0,
            c_sign: 0,
            exp_c: 0,
            mant_c: 0,
            product_exp: 0,
            c_aligned: 0,
        }
    }
}

// =========================================================================
// PipelinedFPAdder — 5-stage pipelined floating-point adder
// =========================================================================

/// A 5-stage pipelined floating-point adder.
///
/// In real GPU hardware, the FP adder is pipelined so that while one
/// addition is being normalized (stage 4), a newer addition is being
/// aligned (stage 2), and an even newer one is being unpacked (stage 1).
///
/// # Pipeline Stages
///
/// ```text
/// Stage 1: UNPACK    — Extract sign, exponent, mantissa. Handle specials.
/// Stage 2: ALIGN     — Compare exponents, shift smaller mantissa right.
/// Stage 3: ADD/SUB   — Add or subtract aligned mantissas.
/// Stage 4: NORMALIZE — Shift result to get leading 1 in correct position.
/// Stage 5: ROUND     — Apply round-to-nearest-even, pack into FloatBits.
/// ```
#[derive(Debug)]
pub struct PipelinedFPAdder {
    pub fmt: FloatFormat,
    pub results: Vec<FloatBits>,
    pub cycle_count: usize,
    stages: [Option<StageData>; 5],
    inputs_pending: Vec<(FloatBits, FloatBits)>,
}

impl PipelinedFPAdder {
    /// Creates a new 5-stage pipelined adder.
    pub fn new(fmt: FloatFormat) -> Self {
        PipelinedFPAdder {
            fmt,
            results: Vec::new(),
            cycle_count: 0,
            stages: Default::default(),
            inputs_pending: Vec::new(),
        }
    }

    /// Queues a new addition `(a + b)` to enter the pipeline on the next tick.
    /// In hardware, this is the dispatch unit loading operands into the
    /// pipeline's input register.
    pub fn submit(&mut self, a: FloatBits, b: FloatBits) {
        self.inputs_pending.push((a, b));
    }

    /// Advances the pipeline by one clock cycle.
    ///
    /// This is the heart of the pipeline simulation. On every tick:
    ///  1. Shift all stages forward: `stage[i] = process(stage[i-1])`
    ///  2. Load new input into stage 0 (if pending)
    ///  3. Collect output from the last stage (if any)
    pub fn tick(&mut self) {
        self.cycle_count += 1;

        // Shift pipeline forward (from end to avoid overwriting)
        for i in (1..5).rev() {
            let input = self.stages[i - 1].take();
            self.stages[i] = self.adder_process_stage(i, input);
        }

        // Load new input
        if !self.inputs_pending.is_empty() {
            let (a, b) = self.inputs_pending.remove(0);
            self.stages[0] = Some(self.adder_stage_unpack(&a, &b));
        } else {
            self.stages[0] = None;
        }

        // Collect output from last stage
        if let Some(ref data) = self.stages[4] {
            if let Some(ref result) = data.special {
                self.results.push(result.clone());
            }
        }
        self.stages[4] = None;
    }

    fn adder_process_stage(&self, stage_num: usize, input: Option<StageData>) -> Option<StageData> {
        let data = input?;
        Some(match stage_num {
            1 => self.adder_stage_align(data),
            2 => self.adder_stage_add(data),
            3 => self.adder_stage_normalize(data),
            4 => self.adder_stage_round_pack(data),
            _ => return None,
        })
    }

    /// Stage 0: UNPACK — extract fields and detect special values.
    fn adder_stage_unpack(&self, a: &FloatBits, b: &FloatBits) -> StageData {
        let f = self.fmt;

        if is_nan(a) || is_nan(b) {
            return StageData { special: Some(make_nan(f)), ..Default::default() };
        }
        let (a_inf, b_inf) = (is_inf(a), is_inf(b));
        if a_inf && b_inf {
            if a.sign == b.sign {
                return StageData { special: Some(make_inf(a.sign, f)), ..Default::default() };
            }
            return StageData { special: Some(make_nan(f)), ..Default::default() };
        }
        if a_inf { return StageData { special: Some(a.clone()), ..Default::default() }; }
        if b_inf { return StageData { special: Some(b.clone()), ..Default::default() }; }

        let (a_zero, b_zero) = (is_zero(a), is_zero(b));
        if a_zero && b_zero {
            return StageData { special: Some(make_zero(a.sign & b.sign, f)), ..Default::default() };
        }
        if a_zero { return StageData { special: Some(b.clone()), ..Default::default() }; }
        if b_zero { return StageData { special: Some(a.clone()), ..Default::default() }; }

        let mut exp_a = bits_msb_to_int(&a.exponent) as i32;
        let mut exp_b = bits_msb_to_int(&b.exponent) as i32;
        let mut mant_a = bits_msb_to_int(&a.mantissa) as u64;
        let mut mant_b = bits_msb_to_int(&b.mantissa) as u64;

        if exp_a != 0 { mant_a = (1u64 << f.mantissa_bits) | mant_a; } else { exp_a = 1; }
        if exp_b != 0 { mant_b = (1u64 << f.mantissa_bits) | mant_b; } else { exp_b = 1; }

        let guard_bits = 3u32;
        mant_a <<= guard_bits;
        mant_b <<= guard_bits;

        StageData {
            sign_a: a.sign, sign_b: b.sign,
            exp_a, exp_b, mant_a, mant_b, guard_bits,
            ..Default::default()
        }
    }

    /// Stage 1: ALIGN — shift the smaller mantissa right.
    fn adder_stage_align(&self, data: StageData) -> StageData {
        if data.special.is_some() { return data; }
        let f = self.fmt;
        let (mut mant_a, mut mant_b) = (data.mant_a, data.mant_b);
        let guard_bits = data.guard_bits;

        let result_exp;
        if data.exp_a >= data.exp_b {
            let exp_diff = (data.exp_a - data.exp_b) as u32;
            if exp_diff > 0 {
                if exp_diff < (f.mantissa_bits + 1 + guard_bits) {
                    let shifted_out = mant_b & ((1u64 << exp_diff) - 1);
                    mant_b >>= exp_diff;
                    if shifted_out != 0 { mant_b |= 1; }
                } else {
                    let sticky = if mant_b != 0 { 1u64 } else { 0 };
                    mant_b >>= exp_diff;
                    if sticky != 0 { mant_b |= 1; }
                }
            }
            result_exp = data.exp_a;
        } else {
            let exp_diff = (data.exp_b - data.exp_a) as u32;
            if exp_diff > 0 {
                if exp_diff < (f.mantissa_bits + 1 + guard_bits) {
                    let shifted_out = mant_a & ((1u64 << exp_diff) - 1);
                    mant_a >>= exp_diff;
                    if shifted_out != 0 { mant_a |= 1; }
                } else {
                    let sticky = if mant_a != 0 { 1u64 } else { 0 };
                    mant_a >>= exp_diff;
                    if sticky != 0 { mant_a |= 1; }
                }
            }
            result_exp = data.exp_b;
        }

        StageData {
            sign_a: data.sign_a, sign_b: data.sign_b,
            mant_a, mant_b, result_exp, guard_bits,
            ..Default::default()
        }
    }

    /// Stage 2: ADD/SUB — add or subtract aligned mantissas.
    fn adder_stage_add(&self, data: StageData) -> StageData {
        if data.special.is_some() { return data; }

        let (result_mant, result_sign) = if data.sign_a == data.sign_b {
            (data.mant_a + data.mant_b, data.sign_a)
        } else if data.mant_a >= data.mant_b {
            (data.mant_a - data.mant_b, data.sign_a)
        } else {
            (data.mant_b - data.mant_a, data.sign_b)
        };

        if result_mant == 0 {
            return StageData { special: Some(make_zero(0, self.fmt)), ..Default::default() };
        }

        StageData {
            result_sign, result_mant, result_exp: data.result_exp, guard_bits: data.guard_bits,
            ..Default::default()
        }
    }

    /// Stage 3: NORMALIZE — shift result to correct position.
    fn adder_stage_normalize(&self, data: StageData) -> StageData {
        if data.special.is_some() { return data; }

        let f = self.fmt;
        let mut result_mant = data.result_mant;
        let mut result_exp = data.result_exp;
        let guard_bits = data.guard_bits;
        let normal_pos = f.mantissa_bits + guard_bits;
        let leading_pos = bit_length(result_mant) - 1;

        if leading_pos > normal_pos {
            let shift_amount = leading_pos - normal_pos;
            let lost_bits = result_mant & ((1u64 << shift_amount) - 1);
            result_mant >>= shift_amount;
            if lost_bits != 0 { result_mant |= 1; }
            result_exp += shift_amount as i32;
        } else if leading_pos < normal_pos {
            let shift_amount = normal_pos - leading_pos;
            if result_exp - shift_amount as i32 >= 1 {
                result_mant <<= shift_amount;
                result_exp -= shift_amount as i32;
            } else {
                let actual_shift = result_exp - 1;
                if actual_shift > 0 { result_mant <<= actual_shift as u32; }
                result_exp = 0;
            }
        }

        StageData {
            result_sign: data.result_sign, result_mant, result_exp, guard_bits,
            ..Default::default()
        }
    }

    /// Stage 4: ROUND & PACK — apply rounding and produce FloatBits.
    fn adder_stage_round_pack(&self, data: StageData) -> StageData {
        if data.special.is_some() { return data; }

        let f = self.fmt;
        let mut result_mant = data.result_mant;
        let mut result_exp = data.result_exp;
        let result_sign = data.result_sign;
        let guard_bits = data.guard_bits;

        let guard = (result_mant >> (guard_bits - 1)) & 1;
        let round_bit = (result_mant >> (guard_bits - 2)) & 1;
        let mut sticky_bit = result_mant & ((1u64 << (guard_bits - 2)) - 1);
        if sticky_bit != 0 { sticky_bit = 1; }

        result_mant >>= guard_bits;

        if guard == 1 {
            if round_bit == 1 || sticky_bit == 1 {
                result_mant += 1;
            } else if (result_mant & 1) == 1 {
                result_mant += 1;
            }
        }

        if result_mant >= (1u64 << (f.mantissa_bits + 1)) {
            result_mant >>= 1;
            result_exp += 1;
        }

        let max_exp = (1i32 << f.exponent_bits) - 1;
        if result_exp >= max_exp {
            return StageData { special: Some(make_inf(result_sign, f)), ..Default::default() };
        }
        if result_exp <= 0 {
            if result_exp < -(f.mantissa_bits as i32) {
                return StageData { special: Some(make_zero(result_sign, f)), ..Default::default() };
            }
            let shift = (1 - result_exp) as u32;
            result_mant >>= shift;
            result_exp = 0;
        }

        if result_exp > 0 {
            result_mant &= (1u64 << f.mantissa_bits) - 1;
        }

        let result = FloatBits {
            sign: result_sign,
            exponent: int_to_bits_msb(result_exp as u64, f.exponent_bits),
            mantissa: int_to_bits_msb(result_mant, f.mantissa_bits),
            fmt: f,
        };
        StageData { special: Some(result), ..Default::default() }
    }
}

// =========================================================================
// PipelinedFPMultiplier — 4-stage pipelined floating-point multiplier
// =========================================================================

/// A 4-stage pipelined floating-point multiplier.
///
/// Multiplication is simpler than addition because there's no alignment step.
///
/// ```text
/// Stage 1: UNPACK + SIGN + EXPONENT
/// Stage 2: MULTIPLY MANTISSAS
/// Stage 3: NORMALIZE
/// Stage 4: ROUND & PACK
/// ```
#[derive(Debug)]
pub struct PipelinedFPMultiplier {
    pub fmt: FloatFormat,
    pub results: Vec<FloatBits>,
    pub cycle_count: usize,
    stages: [Option<StageData>; 4],
    inputs_pending: Vec<(FloatBits, FloatBits)>,
}

impl PipelinedFPMultiplier {
    pub fn new(fmt: FloatFormat) -> Self {
        PipelinedFPMultiplier {
            fmt,
            results: Vec::new(),
            cycle_count: 0,
            stages: Default::default(),
            inputs_pending: Vec::new(),
        }
    }

    pub fn submit(&mut self, a: FloatBits, b: FloatBits) {
        self.inputs_pending.push((a, b));
    }

    pub fn tick(&mut self) {
        self.cycle_count += 1;

        for i in (1..4).rev() {
            let input = self.stages[i - 1].take();
            self.stages[i] = self.mul_process_stage(i, input);
        }

        if !self.inputs_pending.is_empty() {
            let (a, b) = self.inputs_pending.remove(0);
            self.stages[0] = Some(self.mul_stage_unpack_exp(&a, &b));
        } else {
            self.stages[0] = None;
        }

        if let Some(ref data) = self.stages[3] {
            if let Some(ref result) = data.special {
                self.results.push(result.clone());
            }
        }
        self.stages[3] = None;
    }

    fn mul_process_stage(&self, stage_num: usize, input: Option<StageData>) -> Option<StageData> {
        let data = input?;
        Some(match stage_num {
            1 => self.mul_stage_multiply(data),
            2 => self.mul_stage_normalize(data),
            3 => self.mul_stage_round_pack(data),
            _ => return None,
        })
    }

    /// Stage 0: UNPACK + SIGN + EXPONENT
    fn mul_stage_unpack_exp(&self, a: &FloatBits, b: &FloatBits) -> StageData {
        let f = self.fmt;
        let result_sign = a.sign ^ b.sign;

        if is_nan(a) || is_nan(b) {
            return StageData { special: Some(make_nan(f)), ..Default::default() };
        }
        let (a_inf, b_inf) = (is_inf(a), is_inf(b));
        let (a_zero, b_zero) = (is_zero(a), is_zero(b));

        if (a_inf && b_zero) || (b_inf && a_zero) {
            return StageData { special: Some(make_nan(f)), ..Default::default() };
        }
        if a_inf || b_inf {
            return StageData { special: Some(make_inf(result_sign, f)), ..Default::default() };
        }
        if a_zero || b_zero {
            return StageData { special: Some(make_zero(result_sign, f)), ..Default::default() };
        }

        let mut exp_a = bits_msb_to_int(&a.exponent) as i32;
        let mut exp_b = bits_msb_to_int(&b.exponent) as i32;
        let mut mant_a = bits_msb_to_int(&a.mantissa) as u64;
        let mut mant_b = bits_msb_to_int(&b.mantissa) as u64;

        if exp_a != 0 { mant_a = (1u64 << f.mantissa_bits) | mant_a; } else { exp_a = 1; }
        if exp_b != 0 { mant_b = (1u64 << f.mantissa_bits) | mant_b; } else { exp_b = 1; }

        StageData {
            result_sign, result_exp: exp_a + exp_b - f.bias,
            mant_a, mant_b,
            ..Default::default()
        }
    }

    /// Stage 1: MULTIPLY MANTISSAS
    fn mul_stage_multiply(&self, data: StageData) -> StageData {
        if data.special.is_some() { return data; }
        StageData {
            result_sign: data.result_sign,
            result_exp: data.result_exp,
            product: data.mant_a * data.mant_b,
            ..Default::default()
        }
    }

    /// Stage 2: NORMALIZE
    fn mul_stage_normalize(&self, data: StageData) -> StageData {
        if data.special.is_some() { return data; }

        let f = self.fmt;
        let product = data.product;
        let mut result_exp = data.result_exp;

        let product_leading = bit_length(product) - 1;
        let normal_pos = 2 * f.mantissa_bits;

        if product_leading > normal_pos {
            result_exp += (product_leading - normal_pos) as i32;
        } else if product_leading < normal_pos {
            result_exp -= (normal_pos - product_leading) as i32;
        }

        StageData {
            result_sign: data.result_sign,
            result_exp, product, product_leading,
            ..Default::default()
        }
    }

    /// Stage 3: ROUND & PACK
    fn mul_stage_round_pack(&self, data: StageData) -> StageData {
        if data.special.is_some() { return data; }

        let f = self.fmt;
        let result_sign = data.result_sign;
        let mut result_exp = data.result_exp;
        let product = data.product;
        let product_leading = data.product_leading;

        let round_pos = product_leading as i32 - f.mantissa_bits as i32;

        let mut result_mant: u64;
        if round_pos > 0 {
            let rp = round_pos as u32;
            let guard = (product >> (rp - 1)) & 1;
            let mut round_bit = 0u64;
            let mut sticky = 0u64;
            if rp >= 2 {
                round_bit = (product >> (rp - 2)) & 1;
                if product & ((1u64 << (rp - 2)) - 1) != 0 { sticky = 1; }
            }
            result_mant = product >> rp;
            if guard == 1 {
                if round_bit == 1 || sticky == 1 { result_mant += 1; }
                else if (result_mant & 1) == 1 { result_mant += 1; }
            }
            if result_mant >= (1u64 << (f.mantissa_bits + 1)) {
                result_mant >>= 1;
                result_exp += 1;
            }
        } else if round_pos == 0 {
            result_mant = product;
        } else {
            result_mant = product << ((-round_pos) as u32);
        }

        let max_exp = (1i32 << f.exponent_bits) - 1;
        if result_exp >= max_exp {
            return StageData { special: Some(make_inf(result_sign, f)), ..Default::default() };
        }
        if result_exp <= 0 {
            if result_exp < -(f.mantissa_bits as i32) {
                return StageData { special: Some(make_zero(result_sign, f)), ..Default::default() };
            }
            let shift = (1 - result_exp) as u32;
            result_mant >>= shift;
            result_exp = 0;
        }
        if result_exp > 0 {
            result_mant &= (1u64 << f.mantissa_bits) - 1;
        }

        let result = FloatBits {
            sign: result_sign,
            exponent: int_to_bits_msb(result_exp as u64, f.exponent_bits),
            mantissa: int_to_bits_msb(result_mant, f.mantissa_bits),
            fmt: f,
        };
        StageData { special: Some(result), ..Default::default() }
    }
}

// =========================================================================
// PipelinedFMA — 6-stage pipelined fused multiply-add
// =========================================================================

/// A 6-stage pipelined fused multiply-add unit.
///
/// FMA computes `a * b + c` with a single rounding step. It's the most
/// important operation in machine learning because the dot product is
/// just a chain of FMAs.
///
/// ```text
/// Stage 1: UNPACK all three operands
/// Stage 2: MULTIPLY a * b mantissas (full precision!)
/// Stage 3: ALIGN product with c
/// Stage 4: ADD product + c
/// Stage 5: NORMALIZE
/// Stage 6: ROUND & PACK (single rounding step!)
/// ```
#[derive(Debug)]
pub struct PipelinedFMA {
    pub fmt: FloatFormat,
    pub results: Vec<FloatBits>,
    pub cycle_count: usize,
    stages: [Option<StageData>; 6],
    inputs_pending: Vec<(FloatBits, FloatBits, FloatBits)>,
}

impl PipelinedFMA {
    pub fn new(fmt: FloatFormat) -> Self {
        PipelinedFMA {
            fmt,
            results: Vec::new(),
            cycle_count: 0,
            stages: Default::default(),
            inputs_pending: Vec::new(),
        }
    }

    pub fn submit(&mut self, a: FloatBits, b: FloatBits, c: FloatBits) {
        self.inputs_pending.push((a, b, c));
    }

    pub fn tick(&mut self) {
        self.cycle_count += 1;

        for i in (1..6).rev() {
            let input = self.stages[i - 1].take();
            self.stages[i] = self.fma_process_stage(i, input);
        }

        if !self.inputs_pending.is_empty() {
            let (a, b, c) = self.inputs_pending.remove(0);
            self.stages[0] = Some(self.fma_stage_unpack(&a, &b, &c));
        } else {
            self.stages[0] = None;
        }

        if let Some(ref data) = self.stages[5] {
            if let Some(ref result) = data.special {
                self.results.push(result.clone());
            }
        }
        self.stages[5] = None;
    }

    fn fma_process_stage(&self, stage_num: usize, input: Option<StageData>) -> Option<StageData> {
        let data = input?;
        Some(match stage_num {
            1 => self.fma_stage_multiply(data),
            2 => self.fma_stage_align(data),
            3 => self.fma_stage_add(data),
            4 => self.fma_stage_normalize(data),
            5 => self.fma_stage_round_pack(data),
            _ => return None,
        })
    }

    /// Stage 0: UNPACK all three operands
    fn fma_stage_unpack(&self, a: &FloatBits, b: &FloatBits, c: &FloatBits) -> StageData {
        let f = self.fmt;

        if is_nan(a) || is_nan(b) || is_nan(c) {
            return StageData { special: Some(make_nan(f)), ..Default::default() };
        }

        let (a_inf, b_inf, c_inf) = (is_inf(a), is_inf(b), is_inf(c));
        let (a_zero, b_zero) = (is_zero(a), is_zero(b));
        let product_sign = a.sign ^ b.sign;

        if (a_inf && b_zero) || (b_inf && a_zero) {
            return StageData { special: Some(make_nan(f)), ..Default::default() };
        }
        if a_inf || b_inf {
            if c_inf && product_sign != c.sign {
                return StageData { special: Some(make_nan(f)), ..Default::default() };
            }
            return StageData { special: Some(make_inf(product_sign, f)), ..Default::default() };
        }
        if a_zero || b_zero {
            if is_zero(c) {
                return StageData { special: Some(make_zero(product_sign & c.sign, f)), ..Default::default() };
            }
            return StageData { special: Some(c.clone()), ..Default::default() };
        }
        if c_inf {
            return StageData { special: Some(c.clone()), ..Default::default() };
        }

        let mut exp_a = bits_msb_to_int(&a.exponent) as i32;
        let mut exp_b = bits_msb_to_int(&b.exponent) as i32;
        let mut mant_a = bits_msb_to_int(&a.mantissa) as u64;
        let mut mant_b = bits_msb_to_int(&b.mantissa) as u64;
        let mut exp_c = bits_msb_to_int(&c.exponent) as i32;
        let mut mant_c = bits_msb_to_int(&c.mantissa) as u64;

        if exp_a != 0 { mant_a = (1u64 << f.mantissa_bits) | mant_a; } else { exp_a = 1; }
        if exp_b != 0 { mant_b = (1u64 << f.mantissa_bits) | mant_b; } else { exp_b = 1; }
        if exp_c != 0 { mant_c = (1u64 << f.mantissa_bits) | mant_c; } else { exp_c = 1; }

        StageData {
            product_sign, c_sign: c.sign,
            exp_a, exp_b, mant_a, mant_b,
            exp_c, mant_c,
            ..Default::default()
        }
    }

    /// Stage 1: MULTIPLY a * b (full precision)
    fn fma_stage_multiply(&self, data: StageData) -> StageData {
        if data.special.is_some() { return data; }

        let f = self.fmt;
        let product = data.mant_a * data.mant_b;
        let mut product_exp = data.exp_a + data.exp_b - f.bias;

        let product_leading = bit_length(product) - 1;
        let normal_product_pos = 2 * f.mantissa_bits;

        if product_leading > normal_product_pos {
            product_exp += (product_leading - normal_product_pos) as i32;
        } else if product_leading < normal_product_pos {
            product_exp -= (normal_product_pos - product_leading) as i32;
        }

        StageData {
            product_sign: data.product_sign, c_sign: data.c_sign,
            product, product_exp, product_leading,
            exp_c: data.exp_c, mant_c: data.mant_c,
            ..Default::default()
        }
    }

    /// Stage 2: ALIGN product with c
    fn fma_stage_align(&self, data: StageData) -> StageData {
        if data.special.is_some() { return data; }

        let f = self.fmt;
        let mut product = data.product;
        let product_exp = data.product_exp;
        let product_leading = data.product_leading;
        let mant_c = data.mant_c;

        let exp_diff = product_exp - data.exp_c;

        let c_scale_shift = product_leading as i32 - f.mantissa_bits as i32;
        let mut c_aligned: u64;
        if c_scale_shift >= 0 {
            c_aligned = mant_c << (c_scale_shift as u32);
        } else {
            c_aligned = mant_c >> ((-c_scale_shift) as u32);
        }

        let result_exp;
        if exp_diff >= 0 {
            // Clamp shift to avoid overflow — if exp_diff >= 64, the value
            // is shifted entirely to zero (it's too small to contribute).
            let shift = (exp_diff as u32).min(63);
            c_aligned >>= shift;
            result_exp = product_exp;
        } else {
            let shift = ((-exp_diff) as u32).min(63);
            product >>= shift;
            result_exp = data.exp_c;
        }

        StageData {
            product_sign: data.product_sign, c_sign: data.c_sign,
            product, c_aligned, result_exp, product_leading,
            ..Default::default()
        }
    }

    /// Stage 3: ADD product + c
    fn fma_stage_add(&self, data: StageData) -> StageData {
        if data.special.is_some() { return data; }

        let product = data.product;
        let c_aligned = data.c_aligned;

        let (result_mant, result_sign) = if data.product_sign == data.c_sign {
            (product + c_aligned, data.product_sign)
        } else if product >= c_aligned {
            (product - c_aligned, data.product_sign)
        } else {
            (c_aligned - product, data.c_sign)
        };

        if result_mant == 0 {
            return StageData { special: Some(make_zero(0, self.fmt)), ..Default::default() };
        }

        StageData {
            result_sign, result_mant, result_exp: data.result_exp,
            product_leading: data.product_leading,
            ..Default::default()
        }
    }

    /// Stage 4: NORMALIZE
    fn fma_stage_normalize(&self, data: StageData) -> StageData {
        if data.special.is_some() { return data; }

        let f = self.fmt;
        let result_mant = data.result_mant;
        let mut result_exp = data.result_exp;
        let product_leading = data.product_leading;
        let mut target_pos = product_leading;
        if target_pos < f.mantissa_bits {
            target_pos = f.mantissa_bits;
        }

        let result_leading = bit_length(result_mant) - 1;
        if result_leading > target_pos {
            let shift = result_leading - target_pos;
            result_exp += shift as i32;
        } else if result_leading < target_pos {
            let shift_needed = target_pos - result_leading;
            result_exp -= shift_needed as i32;
        }

        StageData {
            result_sign: data.result_sign, result_mant, result_exp,
            ..Default::default()
        }
    }

    /// Stage 5: ROUND & PACK (single rounding step)
    fn fma_stage_round_pack(&self, data: StageData) -> StageData {
        if data.special.is_some() { return data; }

        let f = self.fmt;
        let result_sign = data.result_sign;
        let mut result_exp = data.result_exp;
        let mut result_mant = data.result_mant;

        let result_leading = bit_length(result_mant) - 1;
        let round_pos = result_leading as i32 - f.mantissa_bits as i32;

        if round_pos > 0 {
            let rp = round_pos as u32;
            let guard = (result_mant >> (rp - 1)) & 1;
            let mut round_bit = 0u64;
            let mut sticky = 0u64;
            if rp >= 2 {
                round_bit = (result_mant >> (rp - 2)) & 1;
                if result_mant & ((1u64 << (rp - 2)) - 1) != 0 { sticky = 1; }
            }
            result_mant >>= rp;
            if guard == 1 {
                if round_bit == 1 || sticky == 1 { result_mant += 1; }
                else if (result_mant & 1) == 1 { result_mant += 1; }
            }
            if result_mant >= (1u64 << (f.mantissa_bits + 1)) {
                result_mant >>= 1;
                result_exp += 1;
            }
        } else if round_pos < 0 {
            result_mant <<= (-round_pos) as u32;
        }

        let max_exp = (1i32 << f.exponent_bits) - 1;
        if result_exp >= max_exp {
            return StageData { special: Some(make_inf(result_sign, f)), ..Default::default() };
        }
        if result_exp <= 0 {
            if result_exp < -(f.mantissa_bits as i32) {
                return StageData { special: Some(make_zero(result_sign, f)), ..Default::default() };
            }
            let shift = (1 - result_exp) as u32;
            result_mant >>= shift;
            result_exp = 0;
        }
        if result_exp > 0 {
            result_mant &= (1u64 << f.mantissa_bits) - 1;
        }

        let result = FloatBits {
            sign: result_sign,
            exponent: int_to_bits_msb(result_exp as u64, f.exponent_bits),
            mantissa: int_to_bits_msb(result_mant, f.mantissa_bits),
            fmt: f,
        };
        StageData { special: Some(result), ..Default::default() }
    }
}

// =========================================================================
// FPUnit — a complete floating-point unit with all three pipelines
// =========================================================================

/// A complete floating-point unit with pipelined adder, multiplier, and FMA.
/// This is what sits inside every GPU core.
///
/// ```text
/// +--------------------------------------------------+
/// |                    FP Unit                        |
/// |                                                  |
/// |   +-----------------------------+                |
/// |   |  Pipelined FP Adder (5)     |                |
/// |   +-----------------------------+                |
/// |                                                  |
/// |   +-----------------------------+                |
/// |   |  Pipelined FP Multiplier (4)|                |
/// |   +-----------------------------+                |
/// |                                                  |
/// |   +-----------------------------+                |
/// |   |  Pipelined FMA Unit (6)     |                |
/// |   +-----------------------------+                |
/// |                                                  |
/// |   All three share the same clock signal          |
/// +--------------------------------------------------+
/// ```
///
/// A modern GPU like the NVIDIA RTX 4090 has 16,384 CUDA cores, each
/// containing an FP unit like this. Running at ~2.5 GHz:
///
/// ```text
/// 16,384 cores x 2 FLOPs/cycle (FMA) x 2.52 GHz = 82.6 TFLOPS
/// ```
#[derive(Debug)]
pub struct FPUnit {
    pub fmt: FloatFormat,
    pub adder: PipelinedFPAdder,
    pub multiplier: PipelinedFPMultiplier,
    pub fma: PipelinedFMA,
}

impl FPUnit {
    /// Creates a complete floating-point unit with all three pipelines.
    pub fn new(fmt: FloatFormat) -> Self {
        FPUnit {
            fmt,
            adder: PipelinedFPAdder::new(fmt),
            multiplier: PipelinedFPMultiplier::new(fmt),
            fma: PipelinedFMA::new(fmt),
        }
    }

    /// Runs all three pipelines for `n` complete cycles.
    pub fn tick(&mut self, n: usize) {
        for _ in 0..n {
            self.adder.tick();
            self.multiplier.tick();
            self.fma.tick();
        }
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ieee754::{float_to_bits, bits_to_float};
    use crate::formats::FP32;

    #[test]
    fn test_pipelined_fp_adder_single() {
        let mut adder = PipelinedFPAdder::new(FP32);
        adder.submit(float_to_bits(1.5, FP32), float_to_bits(2.5, FP32));

        for _ in 0..5 {
            adder.tick();
        }

        assert_eq!(adder.results.len(), 1);
        assert_eq!(bits_to_float(&adder.results[0]) as f32, 4.0);
    }

    #[test]
    fn test_pipelined_fp_adder_multiple() {
        let mut adder = PipelinedFPAdder::new(FP32);
        adder.submit(float_to_bits(1.0, FP32), float_to_bits(2.0, FP32)); // 3.0
        adder.submit(float_to_bits(3.0, FP32), float_to_bits(4.0, FP32)); // 7.0
        adder.submit(float_to_bits(0.5, FP32), float_to_bits(0.5, FP32)); // 1.0

        for _ in 0..8 {
            adder.tick();
        }

        assert_eq!(adder.results.len(), 3);
        let expected = [3.0_f64, 7.0, 1.0];
        for (i, &want) in expected.iter().enumerate() {
            assert_eq!(bits_to_float(&adder.results[i]) as f32, want as f32);
        }
    }

    #[test]
    fn test_pipelined_fp_adder_nan() {
        let mut adder = PipelinedFPAdder::new(FP32);
        adder.submit(float_to_bits(f64::NAN, FP32), float_to_bits(1.0, FP32));

        for _ in 0..5 {
            adder.tick();
        }

        assert_eq!(adder.results.len(), 1);
        assert!(is_nan(&adder.results[0]));
    }

    #[test]
    fn test_pipelined_fp_adder_inf() {
        let mut adder = PipelinedFPAdder::new(FP32);
        adder.submit(float_to_bits(f64::INFINITY, FP32), float_to_bits(1.0, FP32));

        for _ in 0..5 {
            adder.tick();
        }

        assert_eq!(adder.results.len(), 1);
        assert!(is_inf(&adder.results[0]));
    }

    #[test]
    fn test_pipelined_fp_adder_zero() {
        let mut adder = PipelinedFPAdder::new(FP32);
        adder.submit(float_to_bits(0.0, FP32), float_to_bits(5.0, FP32));

        for _ in 0..5 {
            adder.tick();
        }

        assert_eq!(adder.results.len(), 1);
        assert_eq!(bits_to_float(&adder.results[0]) as f32, 5.0);
    }

    #[test]
    fn test_pipelined_fp_adder_cycle_count() {
        let mut adder = PipelinedFPAdder::new(FP32);
        adder.tick();
        adder.tick();
        assert_eq!(adder.cycle_count, 2);
    }

    #[test]
    fn test_pipelined_fp_multiplier_single() {
        let mut mul = PipelinedFPMultiplier::new(FP32);
        mul.submit(float_to_bits(3.0, FP32), float_to_bits(4.0, FP32));

        for _ in 0..4 {
            mul.tick();
        }

        assert_eq!(mul.results.len(), 1);
        assert_eq!(bits_to_float(&mul.results[0]) as f32, 12.0);
    }

    #[test]
    fn test_pipelined_fp_multiplier_multiple() {
        let mut mul = PipelinedFPMultiplier::new(FP32);
        mul.submit(float_to_bits(2.0, FP32), float_to_bits(3.0, FP32)); // 6.0
        mul.submit(float_to_bits(5.0, FP32), float_to_bits(5.0, FP32)); // 25.0

        for _ in 0..6 {
            mul.tick();
        }

        assert_eq!(mul.results.len(), 2);
        assert_eq!(bits_to_float(&mul.results[0]) as f32, 6.0);
        assert_eq!(bits_to_float(&mul.results[1]) as f32, 25.0);
    }

    #[test]
    fn test_pipelined_fp_multiplier_special() {
        let mut mul = PipelinedFPMultiplier::new(FP32);

        mul.submit(float_to_bits(f64::INFINITY, FP32), float_to_bits(0.0, FP32)); // NaN
        mul.submit(float_to_bits(f64::NAN, FP32), float_to_bits(1.0, FP32));       // NaN
        mul.submit(float_to_bits(f64::INFINITY, FP32), float_to_bits(2.0, FP32));  // Inf
        mul.submit(float_to_bits(0.0, FP32), float_to_bits(5.0, FP32));            // 0

        for _ in 0..8 {
            mul.tick();
        }

        assert!(mul.results.len() >= 4);
        assert!(is_nan(&mul.results[0]));
        assert!(is_nan(&mul.results[1]));
        assert!(is_inf(&mul.results[2]));
        assert!(is_zero(&mul.results[3]));
    }

    #[test]
    fn test_pipelined_fma_single() {
        let mut fma = PipelinedFMA::new(FP32);
        fma.submit(
            float_to_bits(2.0, FP32),
            float_to_bits(3.0, FP32),
            float_to_bits(1.0, FP32),
        );

        for _ in 0..6 {
            fma.tick();
        }

        assert_eq!(fma.results.len(), 1);
        assert_eq!(bits_to_float(&fma.results[0]) as f32, 7.0);
    }

    #[test]
    fn test_pipelined_fma_multiple() {
        let mut fma = PipelinedFMA::new(FP32);
        fma.submit(
            float_to_bits(1.0, FP32),
            float_to_bits(2.0, FP32),
            float_to_bits(3.0, FP32),
        ); // 5.0
        fma.submit(
            float_to_bits(4.0, FP32),
            float_to_bits(5.0, FP32),
            float_to_bits(0.0, FP32),
        ); // 20.0

        for _ in 0..8 {
            fma.tick();
        }

        assert_eq!(fma.results.len(), 2);
        assert_eq!(bits_to_float(&fma.results[0]) as f32, 5.0);
        assert_eq!(bits_to_float(&fma.results[1]) as f32, 20.0);
    }

    #[test]
    fn test_pipelined_fma_nan() {
        let mut fma = PipelinedFMA::new(FP32);
        fma.submit(
            float_to_bits(f64::NAN, FP32),
            float_to_bits(1.0, FP32),
            float_to_bits(1.0, FP32),
        );

        for _ in 0..6 {
            fma.tick();
        }

        assert_eq!(fma.results.len(), 1);
        assert!(is_nan(&fma.results[0]));
    }

    #[test]
    fn test_pipelined_fma_inf_times_zero() {
        let mut fma = PipelinedFMA::new(FP32);
        fma.submit(
            float_to_bits(f64::INFINITY, FP32),
            float_to_bits(0.0, FP32),
            float_to_bits(1.0, FP32),
        );

        for _ in 0..6 {
            fma.tick();
        }

        assert_eq!(fma.results.len(), 1);
        assert!(is_nan(&fma.results[0]));
    }

    #[test]
    fn test_pipelined_fma_zero_times_finite_plus_c() {
        let mut fma = PipelinedFMA::new(FP32);
        fma.submit(
            float_to_bits(0.0, FP32),
            float_to_bits(5.0, FP32),
            float_to_bits(3.0, FP32),
        );

        for _ in 0..6 {
            fma.tick();
        }

        assert_eq!(fma.results.len(), 1);
        assert_eq!(bits_to_float(&fma.results[0]) as f32, 3.0);
    }

    #[test]
    fn test_fp_unit() {
        let mut unit = FPUnit::new(FP32);

        unit.adder.submit(float_to_bits(1.0, FP32), float_to_bits(2.0, FP32));       // 3.0
        unit.multiplier.submit(float_to_bits(3.0, FP32), float_to_bits(4.0, FP32));  // 12.0
        unit.fma.submit(
            float_to_bits(2.0, FP32),
            float_to_bits(3.0, FP32),
            float_to_bits(1.0, FP32),
        ); // 7.0

        unit.tick(10);

        assert_eq!(unit.adder.results.len(), 1);
        assert_eq!(bits_to_float(&unit.adder.results[0]) as f32, 3.0);

        assert_eq!(unit.multiplier.results.len(), 1);
        assert_eq!(bits_to_float(&unit.multiplier.results[0]) as f32, 12.0);

        assert_eq!(unit.fma.results.len(), 1);
        assert_eq!(bits_to_float(&unit.fma.results[0]) as f32, 7.0);
    }

    #[test]
    fn test_pipeline_throughput() {
        let mut adder = PipelinedFPAdder::new(FP32);

        for i in 0..10 {
            adder.submit(float_to_bits(i as f64, FP32), float_to_bits(1.0, FP32));
        }

        for _ in 0..14 {
            adder.tick();
        }

        assert_eq!(adder.results.len(), 10);
        for i in 0..10 {
            let got = bits_to_float(&adder.results[i]);
            let want = (i as f64) + 1.0;
            assert_eq!(got as f32, want as f32, "result[{i}] = {got}, want {want}");
        }
    }

    #[test]
    fn test_pipelined_adder_subtraction() {
        let mut adder = PipelinedFPAdder::new(FP32);
        adder.submit(float_to_bits(5.0, FP32), float_to_bits(-3.0, FP32));

        for _ in 0..5 {
            adder.tick();
        }

        assert_eq!(adder.results.len(), 1);
        assert_eq!(bits_to_float(&adder.results[0]) as f32, 2.0);
    }

    #[test]
    fn test_empty_pipeline() {
        let mut adder = PipelinedFPAdder::new(FP32);

        for _ in 0..10 {
            adder.tick();
        }

        assert_eq!(adder.results.len(), 0);
    }
}
