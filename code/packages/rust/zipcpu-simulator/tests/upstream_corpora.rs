use zipcpu_simulator::{
    encoding, Condition, InstructionClass, MemoryDirection, MemoryPurpose, MemoryTransaction,
    ZipCpuSimulator, CC_CPU_INFO,
};

const ORIGIN: u32 = 0x100;
const ZSIM: &str = include_str!("fixtures/zsim-rev3-common.csv");
const RTL: &str = include_str!("fixtures/rtl-rev3-cpuops.csv");

fn hex(value: &str) -> u32 {
    u32::from_str_radix(value, 16).unwrap()
}

fn class(op: u32) -> InstructionClass {
    match op {
        0 => InstructionClass::Sub,
        1 => InstructionClass::And,
        2 => InstructionClass::Add,
        3 => InstructionClass::Or,
        4 => InstructionClass::Xor,
        5 => InstructionClass::Lsr,
        6 => InstructionClass::Lsl,
        7 => InstructionClass::Asr,
        8 => InstructionClass::Brev,
        9 => InstructionClass::Ldilo,
        10 => InstructionClass::Mpyuhi,
        11 => InstructionClass::Mpyshi,
        12 => InstructionClass::Mpy,
        13 => InstructionClass::Mov,
        16 => InstructionClass::Cmp,
        17 => InstructionClass::Tst,
        _ => panic!("unexpected corpus opcode {op}"),
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
    })
}

#[test]
fn pinned_cpp_simulator_common_surface_matches_complete_state() {
    assert_eq!(fnv1a(ZSIM.as_bytes()), 0xf535_1c3d_41e3_a1ae);
    let mut count = 0;
    for line in ZSIM.lines().skip(1) {
        let fields: Vec<_> = line.split(',').collect();
        assert_eq!(fields.len(), 7);
        let word = hex(fields[0]);
        let a = hex(fields[1]);
        let b = hex(fields[2]);
        let flags_in = hex(fields[3]);
        let expected_result = hex(fields[4]);
        let expected_flags = hex(fields[5]);
        let expected_pc = hex(fields[6]);

        let mut cpu = ZipCpuSimulator::new().unwrap();
        cpu.load(&word.to_be_bytes(), ORIGIN).unwrap();
        let mut state = cpu.snapshot().unwrap();
        state.supervisor_gpr[1] = a;
        state.supervisor_gpr[2] = b;
        state.supervisor_cc = CC_CPU_INFO | flags_in;
        cpu.restore(&state).unwrap();
        let mut expected = state;
        expected.supervisor_gpr[1] = expected_result;
        expected.supervisor_cc = CC_CPU_INFO | expected_flags;
        expected.supervisor_pc = expected_pc;
        let trace = cpu.step().unwrap();
        assert_eq!(cpu.state(), &expected, "{line}");
        assert_eq!(trace.state_after, expected, "{line}");
        assert_eq!(trace.raw_bytes, word.to_be_bytes(), "{line}");
        assert_eq!(
            trace.transactions,
            vec![MemoryTransaction {
                purpose: MemoryPurpose::Instruction,
                direction: MemoryDirection::Read,
                address: ORIGIN,
                width: 4,
                value: word,
                byte_enable: 0x0f,
                success: true,
            }],
            "{line}"
        );
        count += 1;
    }
    assert_eq!(count, 1_954);
}

#[test]
fn pinned_rtl_cpuops_matches_complete_state_projection() {
    assert_eq!(fnv1a(RTL.as_bytes()), 0x5dc4_fe5c_1e80_40db);
    let mut count = 0;
    for line in RTL.lines().skip(1) {
        let fields: Vec<_> = line.split(',').collect();
        assert_eq!(fields.len(), 5);
        let op = hex(fields[0]);
        let a = hex(fields[1]);
        let b = hex(fields[2]);
        let expected_result = hex(fields[3]);
        let expected_flags = hex(fields[4]);
        let instruction_class = class(op);
        let word = if instruction_class == InstructionClass::Mov {
            encoding::mov(1, false, Condition::Always, 2, false, 0)
        } else {
            encoding::register(instruction_class, 1, Condition::Always, 2, 0)
        };
        let mut cpu = ZipCpuSimulator::new().unwrap();
        cpu.load(&word.to_be_bytes(), ORIGIN).unwrap();
        let mut state = cpu.snapshot().unwrap();
        state.supervisor_gpr[1] = a;
        state.supervisor_gpr[2] = b;
        cpu.restore(&state).unwrap();
        let mut expected = state;
        expected.supervisor_gpr[1] = expected_result;
        if !matches!(
            instruction_class,
            InstructionClass::Brev | InstructionClass::Ldilo | InstructionClass::Mov
        ) {
            expected.supervisor_cc = CC_CPU_INFO | expected_flags;
        }
        expected.supervisor_pc = ORIGIN + 4;
        let trace = cpu.step().unwrap();
        assert_eq!(cpu.state(), &expected, "{line}");
        assert_eq!(trace.state_after, expected, "{line}");
        assert_eq!(trace.raw_bytes, word.to_be_bytes(), "{line}");
        assert_eq!(
            trace.transactions,
            vec![MemoryTransaction {
                purpose: MemoryPurpose::Instruction,
                direction: MemoryDirection::Read,
                address: ORIGIN,
                width: 4,
                value: word,
                byte_enable: 0x0f,
                success: true,
            }],
            "{line}"
        );
        count += 1;
    }
    assert_eq!(count, 2_016);
}
