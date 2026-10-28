use powerpc601_simulator::encoding::{assemble, halt, x_form, xo_form};
use powerpc601_simulator::{PowerPc601Simulator, PowerPcError};

fn seeded(raw: u32) -> PowerPc601Simulator {
    let mut cpu = PowerPc601Simulator::new();
    cpu.load_checked(&assemble(&[raw, halt()])).unwrap();
    let mut state = cpu.get_state();
    state.gpr[1] = 0x7fff_ffff;
    state.gpr[2] = 1;
    state.gpr[3] = 3;
    state.xer = 0xd5a5_5a5a;
    state.cr = 0x1234_5678;
    state.memory[0x100] = 0xa5;
    cpu.restore(&state).unwrap();
    cpu
}

#[test]
fn xo_arithmetic_profile_rejects_every_excluded_modifier_atomically() {
    for xo in [10, 40, 104, 138, 235, 266, 459, 491] {
        for oe in [false, true] {
            for rc in [false, true] {
                let raw = xo_form(31, 4, 1, 2, oe, xo, rc);
                let mut step_cpu = seeded(raw);
                let before = step_cpu.get_state();
                if !oe && !rc {
                    step_cpu.step_checked().unwrap();
                    continue;
                }
                assert_eq!(
                    step_cpu.step_checked(),
                    Err(PowerPcError::UnknownInstruction { raw, cia: 0 }),
                    "XO={xo} OE={oe} Rc={rc}"
                );
                assert_eq!(step_cpu.get_state(), before);

                let mut run_cpu = seeded(raw);
                let before = run_cpu.get_state();
                assert_eq!(
                    run_cpu.run_loaded_checked(4),
                    Err(PowerPcError::UnknownInstruction { raw, cia: 0 }),
                    "bounded XO={xo} OE={oe} Rc={rc}"
                );
                assert_eq!(run_cpu.get_state(), before);
            }
        }
    }
}

#[test]
fn functional_sraw_profile_uses_all_six_count_bits_and_preserves_xer() {
    for (count, expected_ca) in [(0, 0), (1, 0), (31, 0), (32, 1), (33, 1), (63, 1)] {
        let raw = x_form(31, 1, 4, 2, 792, false);
        let mut cpu = seeded(raw);
        let mut state = cpu.get_state();
        state.gpr[1] = 0x8000_0000;
        state.gpr[2] = count;
        state.xer = 0xd5a5_5a5a;
        cpu.restore(&state).unwrap();
        cpu.step_checked().unwrap();
        let after = cpu.get_state();
        assert_eq!((after.xer >> 29) & 1, expected_ca, "count={count}");
        assert_eq!(after.xer & !(1 << 29), state.xer & !(1 << 29));
    }
}
