use microwatt_powerisa_simulator::encoding::{assemble, attn, d_form, sync, x_form, xo_form};
use microwatt_powerisa_simulator::{Error, MemoryKind, MicrowattPowerIsaSimulator, Xer};

fn loaded(word: u32) -> MicrowattPowerIsaSimulator {
    let mut cpu = MicrowattPowerIsaSimulator::new();
    cpu.load_program(&assemble(&[word, attn()]), 0x100).unwrap();
    cpu
}

#[test]
fn transport_trace_and_halt_are_exact() {
    let program = assemble(&[d_form(14, 3, 0, 42), attn()]);
    let mut cpu = MicrowattPowerIsaSimulator::new();
    let result = cpu.execute(&program, 0x100, 4).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces[0].bytes, d_form(14, 3, 0, 42).to_le_bytes());
    assert_eq!(result.traces[0].raw, d_form(14, 3, 0, 42));
    assert_eq!(result.traces[0].mnemonic, "addi");
    assert_eq!(result.traces[0].gpr_writes, vec![(3, 42)]);
    assert_eq!(result.traces[1].mnemonic, "attn");
    assert_eq!(result.final_state.gpr[3], 42);
    assert_eq!(cpu.step(), Err(Error::Halted));
}

#[test]
fn ov32_alone_never_sets_sticky_so() {
    // Pinned execute1.vhdl witness: 0x7fffffff + 1 overflows a word but
    // not the 64-bit operation. Microwatt drives SO only from OV.
    let mut cpu = loaded(xo_form(5, 3, 4, true, 266, true));
    cpu.write_register(3, 0x7fff_ffff).unwrap();
    cpu.write_register(4, 1).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.mnemonic, "addo.");
    assert_eq!(cpu.read_register(5).unwrap(), 0x8000_0000);
    assert_eq!(
        trace.state_after.xer,
        Xer {
            so: false,
            ov: false,
            ov32: true,
            ..Xer::default()
        }
    );
    assert_eq!(trace.state_after.cr >> 28, 0b0100);
}

#[test]
fn word_high_multiply_duplicates_and_records_full_result() {
    // Pinned execute1.vhdl multiplier-result shaping.
    let mut cpu = loaded(x_form(31, 5, 3, 4, 11, true));
    cpu.write_register(3, 0xffff_ffff).unwrap();
    cpu.write_register(4, 0xffff_ffff).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.mnemonic, "mulhwu.");
    assert_eq!(cpu.read_register(5).unwrap(), 0xffff_fffe_ffff_fffe);
    assert_eq!(trace.state_after.cr >> 28, 0b1000);
}

#[test]
fn word_divide_zeroes_upper_word_and_rc_uses_full_64_bits() {
    // Pinned divider.vhdl choice: -10 / 3 is 0x00000000fffffffd,
    // which the 64-bit Rc comparison classifies as greater than zero.
    let mut cpu = loaded(xo_form(5, 3, 4, false, 491, true));
    cpu.write_register(3, (-10_i64) as u64).unwrap();
    cpu.write_register(4, 3).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.mnemonic, "divw.");
    assert_eq!(cpu.read_register(5).unwrap(), 0x0000_0000_ffff_fffd);
    assert_eq!(trace.state_after.cr >> 28, 0b0100);
}

#[test]
fn ptesync_is_recognized_unsupported_and_atomic() {
    let mut cpu = loaded(sync(2));
    let before = cpu.get_state();
    assert_eq!(
        cpu.step(),
        Err(Error::UnsupportedInstruction {
            cia: 0x100,
            raw: sync(2),
            name: "ptesync",
        })
    );
    assert_eq!(cpu.get_state(), before);

    for (level, mnemonic) in [(0, "sync"), (1, "lwsync")] {
        let mut cpu = loaded(sync(level));
        assert_eq!(cpu.step().unwrap().mnemonic, mnemonic);
    }
}

#[test]
fn scalar_memory_is_little_endian_and_traced() {
    let mut cpu = loaded(d_form(36, 3, 4, 8)); // stw r3,8(r4)
    cpu.write_register(3, 0x1122_3344_5566_7788).unwrap();
    cpu.write_register(4, 0x200).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(cpu.read_word(0x208).unwrap(), 0x5566_7788);
    assert_eq!(trace.memory_access.unwrap().kind, MemoryKind::Store);
    assert_eq!(trace.memory_access.unwrap().value, 0x5566_7788);

    let mut cpu = loaded(d_form(42, 5, 4, 8)); // lha r5,8(r4)
    cpu.write_register(4, 0x200).unwrap();
    cpu.write_half(0x208, 0x8001).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(cpu.read_register(5).unwrap(), 0xffff_ffff_ffff_8001);
    assert_eq!(trace.memory_access.unwrap().value, 0x8001);
}

#[test]
fn register_shift_counts_do_not_use_host_masking() {
    let mut cpu = loaded(x_form(31, 3, 5, 4, 24, false));
    cpu.write_register(3, 1).unwrap();
    cpu.write_register(4, 32).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(5).unwrap(), 0);

    let mut cpu = loaded(x_form(31, 3, 5, 4, 27, false));
    cpu.write_register(3, 1).unwrap();
    cpu.write_register(4, 64).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(5).unwrap(), 0);
}
