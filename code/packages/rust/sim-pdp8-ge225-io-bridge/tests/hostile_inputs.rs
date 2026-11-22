//! Hostile-input and deterministic fuzz test suite for `sim-pdp8-ge225-io-bridge`.

#![forbid(unsafe_code)]

use pdp8_gatelevel::Pdp8GateLevel;
use pdp8_simulator::{IotEvent, Pdp8Error, Pdp8Simulator};
use sim_pdp8_ge225_io_bridge::{
    ascii_to_ge225_bcd, assemble_ge225_typewriter_program, assemble_pdp8_teletype_printer_program,
    encode_ascii_for_pdp8, ge225_bcd_to_ascii, ge225_bcd_to_str, pdp8_dispatch_iot_functional,
    str_to_ge225_bcd, GateLevelIotDecoder, Ge225TeletypeBridge, Pdp8TeletypeBridge,
    Pdp8TeletypeGateLevelBridge, MAX_BRIDGE_RUN_STEPS, MAX_GE225_TYPEWRITER_WORDS,
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

    fn next_u16(&mut self) -> u16 {
        (self.next_u64() & 0xFFFF) as u16
    }

    fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as i32
    }
}

#[test]
fn hostile_bridge_step_limits_and_bcd_and_assembler() {
    // Infinite-loop PDP-8 program: JMP 0o0200 (0o5200)
    let jmp_self = [0o5200u16];
    let mut sim = Pdp8Simulator::new();
    sim.load_words(&jmp_self, 0o0200).unwrap();
    let mut cpu = Pdp8GateLevel::new();
    cpu.load_words(&jmp_self, 0o0200).unwrap();

    let mut bridge_func = Pdp8TeletypeBridge::new();
    let mut bridge_gate = Pdp8TeletypeGateLevelBridge::new();

    for &bad_steps in &[0usize, MAX_BRIDGE_RUN_STEPS + 1, usize::MAX] {
        assert_eq!(
            bridge_func.run_sim_until_halt(&mut sim, bad_steps),
            Err(Pdp8Error::MaxStepsExceeded {
                max_steps: bad_steps
            })
        );
        assert_eq!(
            bridge_func.run_gate_until_halt(&mut cpu, bad_steps),
            Err(Pdp8Error::MaxStepsExceeded {
                max_steps: bad_steps
            })
        );
        assert_eq!(
            bridge_gate.run_sim_until_halt(&mut sim, bad_steps),
            Err(Pdp8Error::MaxStepsExceeded {
                max_steps: bad_steps
            })
        );
        assert_eq!(
            bridge_gate.run_gate_until_halt(&mut cpu, bad_steps),
            Err(Pdp8Error::MaxStepsExceeded {
                max_steps: bad_steps
            })
        );
    }

    // All 256 device/pulse/BCD codes
    for code in 0..=255u8 {
        let _ = ge225_bcd_to_ascii(code);
        let _ = ascii_to_ge225_bcd(code);
        let _ = pdp8_dispatch_iot_functional(code, code, u16::MAX, true, true, code);
        let _ = GateLevelIotDecoder::decode(code, code, u16::MAX, false, true, code);
    }

    // Extremal assembler arguments
    assert!(assemble_ge225_typewriter_program(i32::MIN, 1).is_none());
    assert!(assemble_ge225_typewriter_program(i32::MAX, 1).is_none());
    assert!(assemble_ge225_typewriter_program(0, usize::MAX).is_none());
    assert!(assemble_ge225_typewriter_program(0, MAX_GE225_TYPEWRITER_WORDS + 1).is_none());
    let _ = assemble_pdp8_teletype_printer_program(u16::MAX);
    let _ = encode_ascii_for_pdp8("HOSTILE\x00\x7f\u{2603}");
    let _ = str_to_ge225_bcd("Hostile 123 !@#");
    let _ = ge225_bcd_to_str(&[0, 0o77, 255, 128]);
    ge_bridge_sync_check(&mut bridge_func);
}

fn ge_bridge_sync_check(bridge_func: &mut Pdp8TeletypeBridge) {
    let sim = ge225_simulator::Simulator::new(4096).unwrap();
    Ge225TeletypeBridge::sync_sim_typewriter(&sim, &mut bridge_func.asr33);
}

#[test]
fn hostile_fuzz_campaign_1000_cases() {
    let mut rng = SplitMix64(0x9D98_6E22_0000_0001);
    let mut bridge_func = Pdp8TeletypeBridge::new();
    let mut bridge_gate = Pdp8TeletypeGateLevelBridge::new();

    for i in 0..1_000 {
        let dev = rng.next_u8();
        let pulses = rng.next_u8();
        let ac = rng.next_u16();
        let krb = rng.next_u8();
        let kb = (i & 1) == 0;
        let tp = (i & 2) == 0;

        let f_res = pdp8_dispatch_iot_functional(dev, pulses, ac, kb, tp, krb);
        let g_out = GateLevelIotDecoder::decode(dev, pulses, ac, kb, tp, krb);
        assert_eq!(f_res.skip, g_out.skip == 1);
        assert_eq!(f_res.new_ac, g_out.new_ac);

        // Mutate underlying Asr33 state before IOT dispatch
        if i % 15 == 0 {
            bridge_func.asr33.state.rx_active = true;
            bridge_func.asr33.state.rx_bits_received = rng.next_u8();
            bridge_gate.asr33.rx_active = true;
            bridge_gate.asr33.rx_bits_received = rng.next_u8();
            bridge_gate.asr33.tx_shift_ff[0].master_q = rng.next_u8();
            bridge_gate.asr33.kb_flag_ff.slave_q = rng.next_u8();
        }

        let _ = bridge_func.handle_iot(
            IotEvent {
                device: dev,
                pulses,
            },
            ac,
        );
        let _ = bridge_gate.handle_iot(
            IotEvent {
                device: dev,
                pulses,
            },
            ac,
        );
        bridge_func.kbd_push_byte(krb);
        bridge_gate.kbd_push_byte(krb);

        let _ = ge225_bcd_to_ascii(dev);
        let _ = ascii_to_ge225_bcd(krb);
        let _ = ge225_bcd_to_str(&[dev, krb]);
        let _ = assemble_ge225_typewriter_program(rng.next_i32(), (rng.next_u16() as usize) % 5000);
    }
}
