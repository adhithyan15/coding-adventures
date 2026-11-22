//! # VER08b runtime capability conformance for `sim-discrete-arcade-logic`.
//!
//! `required_capabilities.json` declares `"capabilities": []` (pure
//! computation). This test proves it at run time: the full public API is
//! exercised inside a process that has been sandboxed, mechanically from that
//! manifest, to the minimal armed baseline (allocator + exit + TAP output).
//!
//! On non-Linux hosts the gate reports `NOT-LIVE` (RCG010); it never reports
//! a pass it did not earn. Set `RCG_STRICT=1` to turn not-live into failure.

#![deny(unsafe_code)]

use capability_runtime_conformance::{
    run_package_gate, Check, Exercise, GateConfig, PolicyOptions, Rings,
};
use sim_discrete_arcade_logic::{
    bcd_step_op, binary_step_op, dff_step_op, mux_eval_op, nand_eval_op, nor_eval_op,
    paddle_calc_clocks, sync_next_dot, ArcadeSyncGenerator, PaddlePotAdc, QuadLogicLevels,
    Ttl7400QuadNand, Ttl7402QuadNor, Ttl74153DualMux4to1, Ttl7474DualDFlipFlop,
    Ttl7490DecadeCounter, Ttl7493BinaryCounter, MASTER_CLOCK_HZ, PIXEL_CLOCK_HZ,
};
use std::path::PathBuf;

fn check(name: &str, ok: bool) -> Check {
    Check {
        name: name.to_string(),
        ok,
    }
}

fn prepare() -> Exercise {
    std::panic::set_hook(Box::new(|_| {}));

    Box::new(move || {
        let mut checks = Vec::new();

        // 7400 Quad NAND
        let nand = Ttl7400QuadNand::new_7400();
        let nand_res = nand.nand_eval_gate(0, 1, 1);
        checks.push(check("Ttl7400QuadNand eval 1,1 is 0", nand_res == Ok(0)));
        let quad_res = nand.nand_eval_quad(0, 0, 0, 1, 1, 0, 1, 1);
        checks.push(check(
            "Ttl7400QuadNand eval quad returns QuadLogicLevels",
            quad_res
                == Ok(QuadLogicLevels {
                    y0: 1,
                    y1: 1,
                    y2: 1,
                    y3: 0,
                }),
        ));

        // 7402 Quad NOR
        let nor = Ttl7402QuadNor::new_7402();
        let nor_res = nor.nor_eval_gate(0, 0, 0);
        checks.push(check("Ttl7402QuadNor eval 0,0 is 1", nor_res == Ok(1)));

        // 7474 Dual D-FF
        let mut dff = Ttl7474DualDFlipFlop::new_7474();
        let dff_res = dff.dff_step_channel(0, 1, 1, 1, 1);
        checks.push(check(
            "Ttl7474DualDFlipFlop step rising edge sets Q=1",
            dff_res.map(|o| o.q) == Ok(1),
        ));

        // 7490 Decade Counter
        let mut decade = Ttl7490DecadeCounter::new_7490();
        let _ = decade.decade_step_bcd(0);
        let _ = decade.decade_step_bcd(1);
        checks.push(check(
            "Ttl7490DecadeCounter advances to 1",
            decade.decade_read_bcd() == 1,
        ));

        // 7493 Binary Counter
        let mut binary = Ttl7493BinaryCounter::new_7493();
        let _ = binary.binary_step_4bit(0);
        let _ = binary.binary_step_4bit(1);
        checks.push(check(
            "Ttl7493BinaryCounter advances to 1",
            binary.binary_read_u8() == 1,
        ));

        // 74153 Dual MUX
        let mux = Ttl74153DualMux4to1::new_74153();
        let mux_res = mux.mux_eval_section(0, 0, 0, 0, 1, 0, 0, 0);
        checks.push(check(
            "Ttl74153DualMux4to1 selects channel 0",
            mux_res == Ok(1),
        ));

        // ArcadeSyncGenerator
        let mut sync = ArcadeSyncGenerator::new_arcade_sync(MASTER_CLOCK_HZ).unwrap();
        let state = sync.sync_step_master_clock();
        checks.push(check(
            "ArcadeSyncGenerator initial position valid",
            state.dot_x < 455 && state.scanline_y < 262,
        ));

        // PaddlePotAdc
        let mut adc = PaddlePotAdc::new_paddle_pot_adc(PIXEL_CLOCK_HZ, 100_000, 100_000).unwrap();
        let adc_state = adc.paddle_step_clock(0);
        checks.push(check("PaddlePotAdc steps clock cleanly", adc_state.is_ok()));

        // Freestanding operators
        checks.push(check("nand_eval_op", nand_eval_op(0, 0, 0) == Ok(1)));
        checks.push(check("nor_eval_op", nor_eval_op(0, 0, 0) == Ok(1)));
        checks.push(check(
            "mux_eval_op",
            mux_eval_op(0, 0, 0, 0, 1, 0, 0, 0) == Ok(1),
        ));
        checks.push(check("dff_step_op", dff_step_op(1, 1, true, 1, 0) == Ok(1)));
        checks.push(check("bcd_step_op", bcd_step_op(0) == Ok(1)));
        checks.push(check("binary_step_op", binary_step_op(0) == Ok(1)));
        checks.push(check("sync_next_dot", sync_next_dot(0) == 1));
        checks.push(check(
            "paddle_calc_clocks",
            paddle_calc_clocks(0, PIXEL_CLOCK_HZ, 100_000, 100_000).is_ok(),
        ));

        checks
    })
}

fn main() {
    let evidence_dir = option_env!("CARGO_TARGET_TMPDIR")
        .map(PathBuf::from)
        .map(|dir| dir.join("capability-runtime"));
    let config = GateConfig {
        package: "rust/sim-discrete-arcade-logic",
        manifest_json: include_str!("../required_capabilities.json"),
        evidence_dir,
        prepare,
        options: PolicyOptions::default(),
        rings: Rings::ALL,
    };
    std::process::exit(run_package_gate(&config));
}
