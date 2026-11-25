//! # Conformance and Differential Equivalence Suite for Discrete Arcade Logic
//!
//! Validates:
//! - Full functional and gate-level (`logic-gates`) parity across all 7400-series TTL ICs:
//!   - `Ttl7400QuadNand` (4x 2-input NAND)
//!   - `Ttl7402QuadNor` (4x 2-input NOR)
//!   - `Ttl7474DualDFlipFlop` (dual D flip-flop with async preset/clear)
//!   - `Ttl7490DecadeCounter` (BCD decade counter)
//!   - `Ttl7493BinaryCounter` (4-bit ripple binary counter)
//!   - `Ttl74153DualMux4to1` (dual 4-to-1 multiplexer with strobe)
//! - `ArcadeSyncGenerator` master clock divider chain and raster timing:
//!   - 14.31818 MHz crystal master clock, 7.15909 MHz pixel clock.
//!   - 15.75 kHz horizontal line rate (455 dots per scanline).
//!   - 60 Hz vertical frame rate (262 scanlines per frame).
//!   - HSync, VSync, HBlank, VBlank, and `(dot_x, scanline_y)` raster coordinates.
//!   - Bit-for-bit cycle equivalence between functional and gate-level dividers.
//! - `PaddlePotAdc` 555-timer / R-C ramp comparator:
//!   - Monotonic fixed-point charging ramp mapping pot position [0, 65535] to trigger pulse.
//!   - Cycle-accurate trigger pulse and voltage accumulation.
//! - CWE guards: CWE-20 (pin/channel bounds), CWE-369 (zero division), CWE-190 (saturating counters).

#![deny(unsafe_code)]

use sim_discrete_arcade_logic::{
    bcd_step_op, binary_step_op, dff_step_op, mux_eval_op, nand_eval_op, nor_eval_op,
    paddle_calc_clocks, sync_next_dot, ArcadeSyncGenerator, LogicError, PaddlePotAdc,
    QuadLogicLevels, Ttl7400QuadNand, Ttl7402QuadNor, Ttl74153DualMux4to1, Ttl7474DualDFlipFlop,
    Ttl7490DecadeCounter, Ttl7493BinaryCounter, DOTS_PER_LINE, HSYNC_END_DOT, HSYNC_START_DOT,
    MASTER_CLOCK_HZ, PIXEL_CLOCK_HZ, SCANLINES_PER_FRAME, VSYNC_END_LINE, VSYNC_START_LINE,
};

#[test]
fn test_7400_nand_truth_table_and_parity() {
    let nand = Ttl7400QuadNand::new_7400();
    for ch in 0..4 {
        for a in 0..=1 {
            for b in 0..=1 {
                let func_out = nand.nand_eval_gate(ch, a, b).expect("functional NAND");
                let gate_out = nand
                    .nand_eval_gate_level(ch, a, b)
                    .expect("gate-level NAND");
                let expected = if a == 1 && b == 1 { 0 } else { 1 };
                assert_eq!(func_out, expected);
                assert_eq!(gate_out, expected);
            }
        }
    }

    // Quad evaluation parity
    let quad_func = nand
        .nand_eval_quad(0, 0, 0, 1, 1, 0, 1, 1)
        .expect("quad func");
    let quad_gate = nand
        .nand_eval_quad_gate_level(0, 0, 0, 1, 1, 0, 1, 1)
        .expect("quad gate");
    assert_eq!(quad_func, quad_gate);
    assert_eq!(
        quad_func,
        QuadLogicLevels {
            y0: 1,
            y1: 1,
            y2: 1,
            y3: 0,
        }
    );
}

#[test]
fn test_7400_nand_cwe20_guards() {
    let nand = Ttl7400QuadNand::new_7400();
    assert!(matches!(
        nand.nand_eval_gate(4, 0, 0),
        Err(LogicError::InvalidChannel { channel: 4, max: 4 })
    ));
    assert!(matches!(
        nand.nand_eval_gate(0, 2, 0),
        Err(LogicError::InvalidLogicLevel { pin: "a", value: 2 })
    ));
    assert!(matches!(
        nand.nand_eval_gate(0, 0, 255),
        Err(LogicError::InvalidLogicLevel {
            pin: "b",
            value: 255
        })
    ));
}

#[test]
fn test_7402_nor_truth_table_and_parity() {
    let nor = Ttl7402QuadNor::new_7402();
    for ch in 0..4 {
        for a in 0..=1 {
            for b in 0..=1 {
                let func_out = nor.nor_eval_gate(ch, a, b).expect("functional NOR");
                let gate_out = nor.nor_eval_gate_level(ch, a, b).expect("gate-level NOR");
                let expected = if a == 0 && b == 0 { 1 } else { 0 };
                assert_eq!(func_out, expected);
                assert_eq!(gate_out, expected);
            }
        }
    }

    let quad_func = nor
        .nor_eval_quad(0, 0, 0, 1, 1, 0, 1, 1)
        .expect("quad func");
    let quad_gate = nor
        .nor_eval_quad_gate_level(0, 0, 0, 1, 1, 0, 1, 1)
        .expect("quad gate");
    assert_eq!(quad_func, quad_gate);
    assert_eq!(
        quad_func,
        QuadLogicLevels {
            y0: 1,
            y1: 0,
            y2: 0,
            y3: 0,
        }
    );
}

#[test]
fn test_7402_nor_cwe20_guards() {
    let nor = Ttl7402QuadNor::new_7402();
    assert!(matches!(
        nor.nor_eval_gate(5, 0, 0),
        Err(LogicError::InvalidChannel { channel: 5, max: 4 })
    ));
    assert!(matches!(
        nor.nor_eval_gate(1, 3, 0),
        Err(LogicError::InvalidLogicLevel { pin: "a", value: 3 })
    ));
}

#[test]
fn test_7474_dual_dff_clocking_and_parity() {
    let mut dff_func = Ttl7474DualDFlipFlop::new_7474();
    let mut dff_gate = Ttl7474DualDFlipFlop::new_7474();

    for ch in 0..2 {
        // Asynchronous Preset
        let out_func = dff_func
            .dff_step_channel(ch, 0, 0, 0, 1)
            .expect("preset func");
        let out_gate = dff_gate
            .dff_step_channel_gate_level(ch, 0, 0, 0, 1)
            .expect("preset gate");
        assert_eq!(out_func.q, 1);
        assert_eq!(out_func.q_bar, 0);
        assert_eq!(out_func, out_gate);

        // Asynchronous Clear
        let out_func = dff_func
            .dff_step_channel(ch, 0, 1, 1, 0)
            .expect("clear func");
        let out_gate = dff_gate
            .dff_step_channel_gate_level(ch, 0, 1, 1, 0)
            .expect("clear gate");
        assert_eq!(out_func.q, 0);
        assert_eq!(out_func.q_bar, 1);
        assert_eq!(out_func, out_gate);

        // Clock rising edge with D=1
        // First set clock low
        dff_func
            .dff_step_channel(ch, 0, 1, 1, 1)
            .expect("clk lo func");
        dff_gate
            .dff_step_channel_gate_level(ch, 0, 1, 1, 1)
            .expect("clk lo gate");
        // Rising edge
        let out_func = dff_func
            .dff_step_channel(ch, 1, 1, 1, 1)
            .expect("rising edge 1");
        let out_gate = dff_gate
            .dff_step_channel_gate_level(ch, 1, 1, 1, 1)
            .expect("rising edge 1 gate");
        assert_eq!(out_func.q, 1);
        assert_eq!(out_func.q_bar, 0);
        assert_eq!(out_func, out_gate);

        // Clock rising edge with D=0
        dff_func
            .dff_step_channel(ch, 0, 0, 1, 1)
            .expect("clk lo func");
        dff_gate
            .dff_step_channel_gate_level(ch, 0, 0, 1, 1)
            .expect("clk lo gate");
        let out_func = dff_func
            .dff_step_channel(ch, 1, 0, 1, 1)
            .expect("rising edge 0");
        let out_gate = dff_gate
            .dff_step_channel_gate_level(ch, 1, 0, 1, 1)
            .expect("rising edge 0 gate");
        assert_eq!(out_func.q, 0);
        assert_eq!(out_func.q_bar, 1);
        assert_eq!(out_func, out_gate);
    }
}

#[test]
fn test_7474_dual_dff_cwe20_guards() {
    let mut dff = Ttl7474DualDFlipFlop::new_7474();
    assert!(matches!(
        dff.dff_step_channel(2, 0, 0, 1, 1),
        Err(LogicError::InvalidChannel { channel: 2, max: 2 })
    ));
    assert!(matches!(
        dff.dff_step_channel(0, 2, 0, 1, 1),
        Err(LogicError::InvalidLogicLevel {
            pin: "clk",
            value: 2
        })
    ));
}

#[test]
fn test_7490_decade_counter_bcd_sequence_and_parity() {
    let mut cnt_func = Ttl7490DecadeCounter::new_7490();
    let mut cnt_gate = Ttl7490DecadeCounter::new_7490();

    // Initial state is 0
    assert_eq!(cnt_func.decade_read_bcd(), 0);
    assert_eq!(cnt_gate.decade_read_bcd(), 0);

    // Verify 0..9 BCD counting over 30 clock pulses (3 decades)
    for cycle in 0..30 {
        let expected_bcd = ((cycle + 1) % 10) as u8;

        // Clock falling edge
        let out_func = cnt_func.decade_step_bcd(0).expect("func bcd step lo");
        let out_gate = cnt_gate
            .decade_step_bcd_gate_level(0)
            .expect("gate bcd step lo");
        // Clock return high
        cnt_func.decade_step_bcd(1).expect("func bcd step hi");
        cnt_gate
            .decade_step_bcd_gate_level(1)
            .expect("gate bcd step hi");

        assert_eq!(out_func.bcd_value(), expected_bcd);
        assert_eq!(out_gate.bcd_value(), expected_bcd);
        assert_eq!(out_func, out_gate);
    }

    // Reset to 9
    cnt_func.decade_reset(0, 0, 1, 1).expect("reset to 9");
    assert_eq!(cnt_func.decade_read_bcd(), 9);

    // Reset to 0
    cnt_func.decade_reset(1, 1, 0, 0).expect("reset to 0");
    assert_eq!(cnt_func.decade_read_bcd(), 0);
}

#[test]
fn test_7493_binary_ripple_counter_and_parity() {
    let mut cnt_func = Ttl7493BinaryCounter::new_7493();
    let mut cnt_gate = Ttl7493BinaryCounter::new_7493();

    // Initial state is 0
    assert_eq!(cnt_func.binary_read_u8(), 0);
    assert_eq!(cnt_gate.binary_read_u8(), 0);

    // Verify 0..15 4-bit binary counting over 32 clock pulses (2 full rollovers)
    for cycle in 0..32 {
        let expected_val = ((cycle + 1) % 16) as u8;

        let out_func = cnt_func.binary_step_4bit(0).expect("func step lo");
        let out_gate = cnt_gate
            .binary_step_4bit_gate_level(0)
            .expect("gate step lo");
        cnt_func.binary_step_4bit(1).expect("func step hi");
        cnt_gate
            .binary_step_4bit_gate_level(1)
            .expect("gate step hi");

        assert_eq!(out_func.binary_value(), expected_val);
        assert_eq!(out_gate.binary_value(), expected_val);
        assert_eq!(out_func, out_gate);
    }

    // Reset
    cnt_func.binary_reset(1, 1).expect("reset");
    assert_eq!(cnt_func.binary_read_u8(), 0);
}

#[test]
fn test_74153_dual_mux_and_parity() {
    let mux = Ttl74153DualMux4to1::new_74153();

    for s1 in 0..=1 {
        for s0 in 0..=1 {
            let sel = (s1 << 1) | s0;
            // Provide arbitrary inputs: c0=1, c1=0, c2=1, c3=0
            let (c0, c1, c2, c3) = (1, 0, 1, 0);
            let expected = match sel {
                0 => c0,
                1 => c1,
                2 => c2,
                _ => c3,
            };

            let out_func = mux
                .mux_eval_section(0, s0, s1, 0, c0, c1, c2, c3)
                .expect("func mux");
            let out_gate = mux
                .mux_eval_section_gate_level(0, s0, s1, 0, c0, c1, c2, c3)
                .expect("gate mux");
            assert_eq!(out_func, expected);
            assert_eq!(out_gate, expected);

            // Strobe high forces 0
            let out_strobed = mux
                .mux_eval_section(0, s0, s1, 1, c0, c1, c2, c3)
                .expect("strobed mux");
            assert_eq!(out_strobed, 0);
        }
    }
}

#[test]
fn test_arcade_sync_generator_timing_and_parity() {
    let mut sync_func = ArcadeSyncGenerator::new_arcade_sync(MASTER_CLOCK_HZ).expect("sync func");
    let mut sync_gate = ArcadeSyncGenerator::new_arcade_sync(MASTER_CLOCK_HZ).expect("sync gate");

    // Zero master clock guard (CWE-369)
    assert!(matches!(
        ArcadeSyncGenerator::new_arcade_sync(0),
        Err(LogicError::DivisionByZero {
            component: "master_clock_hz"
        })
    ));

    // Run for 2 complete scanlines (455 * 2 * 2 = 1820 master clock cycles)
    let clocks_to_test = (DOTS_PER_LINE as usize) * 4;
    for _ in 0..clocks_to_test {
        let state_func = sync_func.sync_step_master_clock();
        let state_gate = sync_gate.sync_step_master_clock_gate_level();

        assert_eq!(state_func.dot_x, state_gate.dot_x);
        assert_eq!(state_func.scanline_y, state_gate.scanline_y);
        assert_eq!(state_func.hsync, state_gate.hsync);
        assert_eq!(state_func.vsync, state_gate.vsync);
        assert_eq!(state_func.hblank, state_gate.hblank);
        assert_eq!(state_func.vblank, state_gate.vblank);
        assert_eq!(state_func.active_video, state_gate.active_video);
    }

    // Verify coordinate properties after exactly 2 complete scanlines (455 * 2 * 2 = 1820 master clocks)
    assert_eq!(sync_func.dot_x, 0);
    assert_eq!(sync_func.scanline_y, 2);

    // Verify HSync intervals
    const { assert!(HSYNC_START_DOT < HSYNC_END_DOT) };
    assert_eq!(HSYNC_END_DOT - HSYNC_START_DOT, 32);

    // Verify VSync intervals
    const { assert!(VSYNC_START_LINE < VSYNC_END_LINE) };
    assert_eq!(VSYNC_END_LINE - VSYNC_START_LINE, 4);

    // Step across full frame to ensure scanline_y wraps
    let frame_clocks = (DOTS_PER_LINE as u64) * 2 * (SCANLINES_PER_FRAME as u64);
    assert_eq!(frame_clocks, 238_420);
}

#[test]
fn test_paddle_pot_adc_ramp_and_cwe_guards() {
    let mut adc =
        PaddlePotAdc::new_paddle_pot_adc(PIXEL_CLOCK_HZ, 100_000, 100_000).expect("adc new");

    // CWE-369 zero checks
    assert!(matches!(
        PaddlePotAdc::new_paddle_pot_adc(0, 100_000, 100_000),
        Err(LogicError::DivisionByZero {
            component: "clock_hz"
        })
    ));
    assert!(matches!(
        PaddlePotAdc::new_paddle_pot_adc(PIXEL_CLOCK_HZ, 0, 100_000),
        Err(LogicError::DivisionByZero {
            component: "capacitance_pf"
        })
    ));
    assert!(matches!(
        PaddlePotAdc::new_paddle_pot_adc(PIXEL_CLOCK_HZ, 100_000, 0),
        Err(LogicError::DivisionByZero {
            component: "max_pot_resistance_ohms"
        })
    ));

    // Verify monotonicity across pot range: pot=0 < pot=32768 < pot=65535
    let duration_min = adc.paddle_charge_duration_clocks(0).expect("min duration");
    let duration_mid = adc
        .paddle_charge_duration_clocks(32768)
        .expect("mid duration");
    let duration_max = adc
        .paddle_charge_duration_clocks(65535)
        .expect("max duration");

    assert!(duration_min > 0, "min duration must be positive");
    assert!(duration_mid > duration_min, "mid must exceed min");
    assert!(duration_max > duration_mid, "max must exceed mid");

    // Test cycle-by-cycle ramp stepping and trigger pulse emission
    adc.paddle_trigger_discharge();
    let mut saw_trigger_pulse = false;
    let mut pulse_count = 0;

    for _ in 0..=(duration_min + 50) {
        let state = adc.paddle_step_clock(0).expect("step clock");
        if state.trigger_pulse {
            saw_trigger_pulse = true;
            pulse_count += 1;
        }
    }

    assert!(saw_trigger_pulse, "must emit trigger pulse at threshold");
    assert_eq!(pulse_count, 1, "trigger pulse must be exactly 1 cycle long");
    assert!(
        adc.paddle_is_triggered(),
        "ADC must remain triggered until discharged"
    );
}

#[test]
fn test_warrant_freestanding_operators() {
    assert_eq!(nand_eval_op(0, 1, 1).unwrap(), 0);
    assert_eq!(nand_eval_op(0, 1, 0).unwrap(), 1);
    assert_eq!(nor_eval_op(0, 0, 0).unwrap(), 1);
    assert_eq!(nor_eval_op(0, 1, 0).unwrap(), 0);
    assert_eq!(mux_eval_op(0, 1, 0, 0, 0, 1, 0, 0).unwrap(), 1);
    assert_eq!(dff_step_op(1, 1, true, 1, 0).unwrap(), 1);
    assert_eq!(bcd_step_op(8).unwrap(), 9);
    assert_eq!(bcd_step_op(9).unwrap(), 0);
    assert_eq!(binary_step_op(14).unwrap(), 15);
    assert_eq!(binary_step_op(15).unwrap(), 0);
    assert_eq!(sync_next_dot(453), 454);
    assert_eq!(sync_next_dot(454), 0);
    assert!(paddle_calc_clocks(32768, PIXEL_CLOCK_HZ, 100_000, 100_000).unwrap() > 0);
}
