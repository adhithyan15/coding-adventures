//! Hostile-input and deterministic fuzz test suite for `sim-discrete-arcade-logic`.

#![forbid(unsafe_code)]

use sim_discrete_arcade_logic::{
    bcd_step_op, binary_step_op, dff_step_op, mux_eval_op, nand_eval_op, nor_eval_op,
    paddle_calc_clocks, sync_next_dot, ArcadeSyncGenerator, LogicError, PaddlePotAdc,
    Ttl7400QuadNand, Ttl7402QuadNor, Ttl74153DualMux4to1, Ttl7474DualDFlipFlop,
    Ttl7490DecadeCounter, Ttl7493BinaryCounter,
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

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
}

#[test]
fn hostile_ic_boundary_rejections() {
    let nand = Ttl7400QuadNand::new_7400();
    assert!(matches!(
        nand.nand_eval_gate(usize::MAX, 0, 0),
        Err(LogicError::InvalidChannel { .. })
    ));
    assert!(matches!(
        nand.nand_eval_gate_level(0, u8::MAX, 1),
        Err(LogicError::InvalidLogicLevel { .. })
    ));
    assert!(nand.nand_eval_quad(2, 0, 0, 0, 0, 0, 0, 0).is_err());

    let nor = Ttl7402QuadNor::new_7402();
    assert!(nor.nor_eval_gate(usize::MAX, 0, 0).is_err());
    assert!(nor.nor_eval_gate_level(0, 0, u8::MAX).is_err());
    assert!(nor.nor_eval_quad(0, 0, 0, 0, 0, 0, 0, 2).is_err());

    let mut dff = Ttl7474DualDFlipFlop::new_7474();
    assert!(dff.dff_step_channel(usize::MAX, 0, 0, 1, 1).is_err());
    assert!(dff.dff_step_channel_gate_level(0, 2, 0, 1, 1).is_err());
    assert!(dff_step_op(2, 0, false, 1, 1).is_err());

    let mut cnt90 = Ttl7490DecadeCounter::new_7490();
    assert!(cnt90.decade_step_bcd(u8::MAX).is_err());
    assert!(cnt90.decade_step_bcd_gate_level(2).is_err());
    assert!(cnt90.decade_reset(2, 0, 0, 0).is_err());
    assert!(bcd_step_op(10).is_err());
    assert!(bcd_step_op(u8::MAX).is_err());

    let mut cnt93 = Ttl7493BinaryCounter::new_7493();
    assert!(cnt93.binary_step_4bit(u8::MAX).is_err());
    assert!(cnt93.binary_step_4bit_gate_level(2).is_err());
    assert!(cnt93.binary_reset(2, 0).is_err());
    assert!(binary_step_op(16).is_err());
    assert!(binary_step_op(u8::MAX).is_err());

    let mux = Ttl74153DualMux4to1::new_74153();
    assert!(mux.mux_eval_section(2, 0, 0, 0, 0, 0, 0, 0).is_err());
    assert!(mux
        .mux_eval_section_gate_level(0, 2, 0, 0, 0, 0, 0, 0)
        .is_err());
    assert!(mux_eval_op(0, 0, 0, 0, 0, 0, 0, 2).is_err());

    assert!(ArcadeSyncGenerator::new_arcade_sync(0).is_err());
    let _ = sync_next_dot(u16::MAX);

    assert!(PaddlePotAdc::new_paddle_pot_adc(0, 1, 1).is_err());
    assert!(PaddlePotAdc::new_paddle_pot_adc(1, 0, 1).is_err());
    assert!(PaddlePotAdc::new_paddle_pot_adc(1, 1, 0).is_err());
    assert!(paddle_calc_clocks(u16::MAX, 0, 1, 1).is_err());
    assert!(paddle_calc_clocks(u16::MAX, u64::MAX, u64::MAX, u64::MAX).is_ok());
}

#[test]
fn hostile_fuzz_campaign_1000_cases() {
    let mut rng = SplitMix64(0x7400_1061_0000_0001);
    let nand = Ttl7400QuadNand::new_7400();
    let nor = Ttl7402QuadNor::new_7402();
    let mux = Ttl74153DualMux4to1::new_74153();
    let mut dff = Ttl7474DualDFlipFlop::new_7474();
    let mut cnt90 = Ttl7490DecadeCounter::new_7490();
    let mut cnt93 = Ttl7493BinaryCounter::new_7493();
    let mut sync_gen = ArcadeSyncGenerator::new_arcade_sync(u64::MAX).unwrap();
    let mut adc = PaddlePotAdc::new_paddle_pot_adc(u64::MAX, u64::MAX, u64::MAX).unwrap();

    for i in 0..1_000 {
        let ch = (rng.next_u32() as usize) % 8;
        let a = rng.next_u8() % 4;
        let b = rng.next_u8() % 4;
        let c = rng.next_u8() % 4;
        let d = rng.next_u8() % 4;

        let _ = nand.nand_eval_gate(ch, a, b);
        let _ = nand.nand_eval_gate_level(ch, a, b);
        let _ = nand_eval_op(ch, a, b);

        let _ = nor.nor_eval_gate(ch, a, b);
        let _ = nor.nor_eval_gate_level(ch, a, b);
        let _ = nor_eval_op(ch, a, b);

        let _ = dff.dff_step_channel(ch, a, b, c, d);
        let _ = dff.dff_step_channel_gate_level(ch, a, b, c, d);
        let _ = dff_step_op(a, b, i % 2 == 0, c, d);

        let _ = cnt90.decade_step_bcd(a);
        let _ = cnt90.decade_step_bcd_gate_level(a);
        let _ = cnt90.decade_reset(a, b, c, d);
        let _ = bcd_step_op(rng.next_u8() % 20);

        let _ = cnt93.binary_step_4bit(a);
        let _ = cnt93.binary_step_4bit_gate_level(a);
        let _ = cnt93.binary_reset(a, b);
        let _ = binary_step_op(rng.next_u8() % 24);

        let _ = mux.mux_eval_section(ch, a, b, c, d, a, b, c);
        let _ = mux.mux_eval_section_gate_level(ch, a, b, c, d, a, b, c);
        let _ = mux_eval_op(ch, a, b, c, d, a, b, c);

        let _ = sync_next_dot(rng.next_u16());
        let _ = sync_gen.sync_step_master_clock();
        let _ = sync_gen.sync_step_master_clock_gate_level();

        let pot = rng.next_u16();
        let _ = adc.paddle_charge_duration_clocks(pot);
        let _ = adc.paddle_step_clock(pot);
        if i % 50 == 0 {
            adc.paddle_trigger_discharge();
        }
        let _ = paddle_calc_clocks(
            pot,
            if i % 10 == 0 { 0 } else { rng.next_u64() },
            if i % 11 == 0 { 0 } else { rng.next_u64() },
            if i % 12 == 0 { 0 } else { rng.next_u64() },
        );
    }
}
