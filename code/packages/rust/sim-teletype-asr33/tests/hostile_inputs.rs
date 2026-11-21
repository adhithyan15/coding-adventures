//! Hostile-input and deterministic fuzz test suite for `sim-teletype-asr33`.

#![forbid(unsafe_code)]

use sim_teletype_asr33::{
    compute_even_parity, new_asr33, new_asr33_gate_level, parity_generator_7xor,
    render_paper_roll_ascii, to_paint_scene, Asr33FrameState, Asr33Functional, Asr33GateLevel,
};

struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_u8(&mut self) -> u8 {
        (self.next_u64() & 0xFF) as u8
    }

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
}

#[test]
fn hostile_parity_and_mutated_gate_flip_flops() {
    for v in 0..=255u8 {
        let p = parity_generator_7xor(v, v.wrapping_add(1), 255, 2, 3, 128, 254);
        assert!(p <= 1);
        assert!(compute_even_parity(v) <= 1);
    }

    // Mutate every FlipFlopState field in Asr33GateLevel to 255 across all methods
    let mut gate = new_asr33_gate_level();
    for ff in &mut gate.tx_shift_ff {
        ff.master_q = 255;
        ff.master_q_bar = 255;
        ff.slave_q = 255;
        ff.slave_q_bar = 255;
    }
    for ff in &mut gate.krb_ff {
        ff.master_q = 255;
        ff.master_q_bar = 255;
        ff.slave_q = 255;
        ff.slave_q_bar = 255;
    }
    for ff in &mut gate.tto_ff {
        ff.master_q = 255;
        ff.master_q_bar = 255;
        ff.slave_q = 255;
        ff.slave_q_bar = 255;
    }
    for ff in &mut gate.rx_shift_ff {
        ff.master_q = 255;
        ff.master_q_bar = 255;
        ff.slave_q = 255;
        ff.slave_q_bar = 255;
    }
    gate.kb_flag_ff.master_q = 255;
    gate.kb_flag_ff.slave_q = 255;
    gate.tp_flag_ff.master_q = 255;
    gate.tp_flag_ff.slave_q = 255;

    gate.tx_load(b'A');
    for _ in 0..15 {
        let _ = gate.step_serial_bit();
    }
    gate.rx_active = true;
    gate.rx_bits_received = 255;
    let _ = gate.rx_feed_bit(1);
    assert!(!gate.rx_active);
    gate.kbd_push_byte(0xFF);
    let _ = gate.kbd_read_clear();
    gate.tp_clear_flag();
}

#[test]
fn hostile_rx_tx_platen_and_non_utf8_paper() {
    let mut frame = Asr33FrameState::new();
    frame.rx_active = true;
    frame.rx_bits_received = 255;
    let out = frame.rx_feed_bit(1);
    assert!(out.is_some());
    assert!(!frame.rx_active);

    frame.tx_bits_remaining = 255;
    for _ in 0..260 {
        let _ = frame.step_serial_bit();
    }

    frame.platen_col = 255;
    frame.platen_row = u32::MAX;
    frame.print_byte(b'z');
    frame.print_byte(0x07);
    frame.print_byte(0x0D);
    frame.print_byte(0x0A);

    let mut func = new_asr33();
    func.state.platen_col = 255;
    func.state.platen_row = u32::MAX;
    func.paper[0].fill(0xFF);
    func.line_lengths[0] = 255;
    func.print_char(b'a');
    let _ = func.paper_roll_text();
    let _ = to_paint_scene(&func);
    let _ = render_paper_roll_ascii(&func);
}

#[test]
fn hostile_fuzz_campaign_1500_cases() {
    let mut rng = SplitMix64(0xA533_1963_0000_0001);
    let mut func = Asr33Functional::new();
    let mut gate = Asr33GateLevel::new();

    for i in 0..1_500 {
        let b = rng.next_u8();
        let _ = parity_generator_7xor(
            b,
            rng.next_u8(),
            rng.next_u8(),
            rng.next_u8(),
            rng.next_u8(),
            rng.next_u8(),
            rng.next_u8(),
        );
        let _ = compute_even_parity(b);

        if i % 17 == 0 {
            func.state.rx_active = true;
            func.state.rx_bits_received = rng.next_u8();
            gate.rx_active = true;
            gate.rx_bits_received = rng.next_u8();
        }
        if i % 19 == 0 {
            func.state.tx_bits_remaining = rng.next_u8();
            gate.tx_bits_remaining = rng.next_u8();
        }
        if i % 23 == 0 {
            func.state.platen_col = rng.next_u8();
            func.state.platen_row = if i % 46 == 0 {
                u32::MAX
            } else {
                rng.next_u32()
            };
            let idx = (rng.next_u8() as usize) % 64;
            func.paper[idx].fill(rng.next_u8());
            func.line_lengths[idx] = rng.next_u8();
        }
        if i % 29 == 0 {
            let slot = (rng.next_u8() as usize) % 8;
            gate.rx_shift_ff[slot].slave_q = rng.next_u8();
            gate.tx_shift_ff[slot].master_q = rng.next_u8();
            gate.krb_ff[slot].slave_q_bar = rng.next_u8();
            gate.kb_flag_ff.master_q = rng.next_u8();
            gate.tp_flag_ff.slave_q = rng.next_u8();
        }

        match i % 6 {
            0 => {
                func.tx_load(b);
                gate.tx_load(b);
            }
            1 => {
                let _ = func.step_serial_bit();
                let _ = gate.step_serial_bit();
            }
            2 => {
                let _ = func.rx_feed_bit(b);
                let _ = gate.rx_feed_bit(b);
            }
            3 => {
                func.kbd_push_byte(b);
                gate.kbd_push_byte(b);
            }
            4 => {
                let _ = func.kbd_read_clear();
                let _ = gate.kbd_read_clear();
            }
            _ => {
                func.print_char(b);
                gate.tp_clear_flag();
                func.tp_clear_flag();
            }
        }

        if i % 50 == 0 {
            let _ = func.paper_roll_text();
            let _ = to_paint_scene(&func);
        }
    }
}
