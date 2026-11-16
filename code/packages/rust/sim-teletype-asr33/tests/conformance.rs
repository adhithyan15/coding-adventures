//! # Conformance and Differential Verification Suite for ASR-33 Teletype
//!
//! Validates:
//! - Exhaustive 256-byte differential equivalence between `Asr33Functional` and
//!   `Asr33GateLevel` across all 11 serial bit clocks, even parity generator tree,
//!   and `tp_flag` / `kb_flag` sequencing.
//! - Keyboard reader buffer (KRB) and serial receiver (`rx_feed_bit`) equivalence.
//! - 72-column platen mechanics: column indexing 0..=71, auto-wrap at column 72,
//!   CR carriage return, LF line feed, ASCII 0x07 BEL electromechanical ring counter,
//!   and physical typewheel uppercase drum folding ('a'..='z' -> 'A'..='Z').
//! - Declarative 2D paper-roll scene generation (`to_paint_scene`) and ASCII rendering
//!   via `paint-vm-ascii` (`P2D02`) with carriage print-head cursor.
//! - CWE adversarial boundary immunization: CWE-190, CWE-125, CWE-787, and CWE-20.

#![deny(unsafe_code)]

use sim_teletype_asr33::{
    compute_even_parity, new_asr33, new_asr33_gate_level, parity_generator_7xor,
    render_paper_roll_ascii, to_paint_scene, Asr33FrameState, Asr33Functional, Asr33GateLevel,
    ASCII_BEL, ASCII_CR, ASCII_LF, BITS_PER_FRAME, PLATEN_WIDTH, SERIAL_MARK, SERIAL_SPACE,
};

#[test]
fn test_constructor_helpers() {
    let func = new_asr33();
    let gate = new_asr33_gate_level();
    assert!(func.tp_flag());
    assert!(gate.tp_flag());
}

#[test]
fn test_7xor_parity_tree_exhaustion() {
    for b in 0u8..=127u8 {
        let b0 = b & 1;
        let b1 = (b >> 1) & 1;
        let b2 = (b >> 2) & 1;
        let b3 = (b >> 3) & 1;
        let b4 = (b >> 4) & 1;
        let b5 = (b >> 5) & 1;
        let b6 = (b >> 6) & 1;

        let tree_parity = parity_generator_7xor(b0, b1, b2, b3, b4, b5, b6);
        let ones_count = b.count_ones();
        let expected_parity = (ones_count % 2) as u8;

        assert_eq!(
            tree_parity, expected_parity,
            "parity mismatch for 7-bit byte 0x{b:02X}"
        );

        // Verify total number of 1-bits is even
        let total_ones = ones_count + (tree_parity as u32);
        assert_eq!(total_ones % 2, 0, "total 1-bits must be even");
    }
}

#[test]
fn test_exhaustive_256_byte_differential_equivalence() {
    for byte in 0u8..=255u8 {
        let mut func = Asr33Functional::new();
        let mut gate = Asr33GateLevel::new();

        // Initially both teleprinter flags are true (ready)
        assert!(func.tp_flag());
        assert!(gate.tp_flag());
        assert!(!func.kb_flag());
        assert!(!gate.kb_flag());

        // Load byte into both transmitter models
        func.tx_load(byte);
        gate.tx_load(byte);

        // Immediately after load, teleprinter is busy (tp_flag = false)
        assert!(!func.tp_flag(), "byte {byte}: func tp_flag should be false");
        assert!(!gate.tp_flag(), "byte {byte}: gate tp_flag should be false");

        let ascii = byte & 0x7F;
        let expected_parity = (ascii.count_ones() % 2) as u8;

        // Step through all 11 bit times of the 110-baud serial frame
        for bit_idx in 0..BITS_PER_FRAME {
            let func_bit = func.step_serial_bit();
            let gate_bit = gate.step_serial_bit();

            // Assert 100% wire-level bit match
            assert_eq!(
                func_bit, gate_bit,
                "byte 0x{byte:02X}, bit clock {bit_idx}: wire mismatch"
            );

            // Assert exact flag state alignment
            assert_eq!(
                func.tp_flag(),
                gate.tp_flag(),
                "byte 0x{byte:02X}, bit clock {bit_idx}: tp_flag mismatch"
            );
            assert_eq!(
                func.kb_flag(),
                gate.kb_flag(),
                "byte 0x{byte:02X}, bit clock {bit_idx}: kb_flag mismatch"
            );

            // Frame-specific bit assertions
            match bit_idx {
                0 => {
                    // Start bit must be Space (0)
                    assert_eq!(func_bit, SERIAL_SPACE);
                }
                1..=7 => {
                    // Data bits b0..b6 (LSB first)
                    let expected_data_bit = (ascii >> (bit_idx - 1)) & 1;
                    assert_eq!(func_bit, expected_data_bit);
                }
                8 => {
                    // Parity bit in bit position 8
                    assert_eq!(func_bit, expected_parity);
                }
                9 => {
                    // Stop bit 1 must be Mark (1)
                    assert_eq!(func_bit, SERIAL_MARK);
                }
                10 => {
                    // Stop bit 2 must be Mark (1)
                    assert_eq!(func_bit, SERIAL_MARK);
                }
                _ => unreachable!(),
            }
        }

        // After all 11 bits are clocked, transmission has completed!
        assert!(
            func.tp_flag(),
            "byte 0x{byte:02X}: func tp_flag must be true"
        );
        assert!(
            gate.tp_flag(),
            "byte 0x{byte:02X}: gate tp_flag must be true"
        );

        // Next bit clocks must remain idle Mark (1)
        assert_eq!(func.step_serial_bit(), SERIAL_MARK);
        assert_eq!(gate.step_serial_bit(), SERIAL_MARK);
        assert!(func.tp_flag());
        assert!(gate.tp_flag());

        // Verify serial TX completion printed exactly once (no double-printing!)
        match ascii {
            0x07 => {
                assert_eq!(func.bel_count(), 1);
                assert_eq!(func.platen_col(), 0);
            }
            0x0D => {
                assert_eq!(func.platen_col(), 0);
            }
            0x0A => {
                assert_eq!(func.platen_row(), 1);
                assert_eq!(func.platen_col(), 0);
            }
            0x20..=0x7E => {
                assert_eq!(func.platen_col(), 1, "byte 0x{byte:02X} printed twice or not at all");
                assert_eq!(func.platen_row(), 0);
                let expected_ch = if (0x61..=0x7A).contains(&ascii) {
                    (ascii - 0x20) as char
                } else {
                    ascii as char
                };
                if expected_ch != ' ' {
                    assert_eq!(func.paper_roll_text(), expected_ch.to_string());
                }
            }
            _ => {
                assert_eq!(func.platen_col(), 0);
                assert_eq!(func.platen_row(), 0);
            }
        }
    }
}

#[test]
fn test_keyboard_and_receiver_differential_equivalence() {
    for byte in 0u8..=255u8 {
        let mut func = Asr33Functional::new();
        let mut gate = Asr33GateLevel::new();

        // Push character from keyboard
        func.kbd_push_byte(byte);
        gate.kbd_push_byte(byte);

        assert!(func.kb_flag());
        assert!(gate.kb_flag());

        let ascii = byte & 0x7F;
        let expected_parity = compute_even_parity(byte);
        let expected_assembled = ascii | (expected_parity << 7);

        // Read and clear KRB
        let func_read = func.kbd_read_clear();
        let gate_read = gate.kbd_read_clear();

        assert_eq!(func_read, gate_read);
        assert_eq!(func_read, expected_assembled);
        assert!(!func.kb_flag());
        assert!(!gate.kb_flag());
    }
}

#[test]
fn test_serial_rx_stream_reconstruction() {
    let test_bytes = [b'A', b'Z', b'0', b'9', b'\r', b'\n', 0x07, 0x20];

    for &byte in &test_bytes {
        let mut func = Asr33Functional::new();
        let mut gate = Asr33GateLevel::new();

        let ascii = byte & 0x7F;
        let p = compute_even_parity(byte);
        let frame_bits = [
            SERIAL_SPACE,
            ascii & 1,
            (ascii >> 1) & 1,
            (ascii >> 2) & 1,
            (ascii >> 3) & 1,
            (ascii >> 4) & 1,
            (ascii >> 5) & 1,
            (ascii >> 6) & 1,
            p,
            SERIAL_MARK,
            SERIAL_MARK,
        ];

        for (idx, &bit) in frame_bits.iter().enumerate() {
            let func_res = func.rx_feed_bit(bit);
            let gate_res = gate.rx_feed_bit(bit);

            assert_eq!(func_res, gate_res);

            if idx < 10 {
                assert_eq!(func_res, None);
                assert!(!func.kb_flag());
                assert!(!gate.kb_flag());
            } else {
                let expected = ascii | (p << 7);
                assert_eq!(func_res, Some(expected));
                assert!(func.kb_flag());
                assert!(gate.kb_flag());
            }
        }
    }
}

#[test]
fn test_72_column_platen_autowrap_and_mechanics() {
    let mut asr = Asr33Functional::new();

    // Line 0: Print 72 characters 'A'
    for col in 0..PLATEN_WIDTH {
        assert_eq!(asr.platen_col(), col as u8);
        assert_eq!(asr.platen_row(), 0);
        asr.print_char(b'A');
    }

    // After 72 characters, platen_col is at 72, platen_row is 0
    assert_eq!(asr.platen_col(), 72);
    assert_eq!(asr.platen_row(), 0);

    // 73rd character auto-wraps to line 1, column 0 before printing, then advances to col 1
    asr.print_char(b'B');
    assert_eq!(asr.platen_row(), 1);
    assert_eq!(asr.platen_col(), 1);

    // Verify paper roll text contents
    let text = asr.paper_roll_text();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "A".repeat(72));
    assert_eq!(lines[1], "B");
}

#[test]
fn test_cr_lf_and_bel_mechanics() {
    let mut asr = Asr33Functional::new();

    // Print 10 characters
    for _ in 0..10 {
        asr.print_char(b'X');
    }
    assert_eq!(asr.platen_col(), 10);
    assert_eq!(asr.platen_row(), 0);

    // ASCII_BEL rings the bell, doesn't move column or advance row
    assert_eq!(asr.bel_count(), 0);
    asr.print_char(ASCII_BEL);
    assert_eq!(asr.bel_count(), 1);
    assert_eq!(asr.platen_col(), 10);
    assert_eq!(asr.platen_row(), 0);

    // Carriage Return (CR) resets column to 0 without advancing row
    asr.print_char(ASCII_CR);
    assert_eq!(asr.platen_col(), 0);
    assert_eq!(asr.platen_row(), 0);

    // Line Feed (LF) advances row without changing column
    asr.print_char(ASCII_LF);
    assert_eq!(asr.platen_col(), 0);
    assert_eq!(asr.platen_row(), 1);
}

#[test]
fn test_uppercase_drum_folding() {
    let mut asr = Asr33Functional::new();

    let input = b"Hello, World! 123";
    for &ch in input {
        asr.print_char(ch);
    }

    let text = asr.paper_roll_text();
    // Lowercase 'e', 'l', 'l', 'o', 'o', 'r', 'l', 'd' folded to uppercase
    assert_eq!(text, "HELLO, WORLD! 123");
}

#[test]
fn test_paint_vm_ascii_paper_roll_rendering() {
    let mut asr = Asr33Functional::new();

    let msg = b"TELETYPE MODEL 33 ASR\r\nREADY\r\n";
    for &ch in msg {
        asr.print_char(ch);
    }

    let scene = to_paint_scene(&asr);
    assert_eq!(scene.width, 72.0);
    assert!(scene.height >= 3.0);
    assert_eq!(scene.background, "#ffffff");

    let rendered = render_paper_roll_ascii(&asr);
    assert!(rendered.contains("TELETYPE MODEL 33 ASR"));
    assert!(rendered.contains("READY"));
    // Print carriage cursor character '█' (\u{2588}) should be present
    assert!(rendered.contains('\u{2588}'));
}

#[test]
fn test_adversarial_cwe190_integer_overflow_prevention() {
    let mut state = Asr33FrameState::new();
    state.platen_row = u32::MAX;
    state.bel_count = u32::MAX;

    // BEL increment must not panic or overflow
    state.print_byte(ASCII_BEL);
    assert_eq!(state.bel_count, u32::MAX);

    // LF increment must not panic or overflow
    state.print_byte(ASCII_LF);
    assert_eq!(state.platen_row, u32::MAX);
}

#[test]
fn test_adversarial_cwe125_cwe787_buffer_bounds() {
    let mut asr = Asr33Functional::new();

    // Print 100 lines (more than circular buffer 64 lines)
    for line in 0..100 {
        let msg = format!("LINE {:03}\r\n", line);
        for &b in msg.as_bytes() {
            asr.print_char(b);
        }
    }

    // Verify no panic, circular buffer holds up to 64 lines safely
    let text = asr.paper_roll_text();
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines.len() <= 64);
    assert!(lines.last().unwrap().starts_with("LINE 099"));
}

#[test]
fn test_adversarial_cwe20_high_byte_input_sanitization() {
    let mut asr = Asr33Functional::new();

    // Feed bytes with MSB set (0x80..=0xFF)
    for b in 0x80..=0xFF {
        asr.tx_load(b);
        for _ in 0..11 {
            asr.step_serial_bit();
        }
    }

    assert!(asr.tp_flag());
}
