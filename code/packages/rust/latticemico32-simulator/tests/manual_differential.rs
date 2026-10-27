use latticemico32_simulator::{Lm32Simulator, Operation, EXCEPTION_DIVIDE_BY_ZERO};

const fn reg(op: u32) -> u32 {
    (op << 26) | (1 << 21) | (2 << 16) | (3 << 11)
}

fn loaded(word: u32, a: u32, b: u32) -> Lm32Simulator {
    let mut cpu = Lm32Simulator::new();
    cpu.load(&word.to_be_bytes(), 0x100).unwrap();
    cpu.write_register(1, a).unwrap();
    cpu.write_register(2, b).unwrap();
    cpu
}

fn expected_binary(operation: Operation, a: u32, b: u32) -> Option<u32> {
    use Operation::*;
    match operation {
        Add => Some(a.wrapping_add(b)),
        Sub => Some(a.wrapping_sub(b)),
        Mul => Some(a.wrapping_mul(b)),
        Divu => a.checked_div(b),
        Modu => a.checked_rem(b),
        And => Some(a & b),
        Or => Some(a | b),
        Xor => Some(a ^ b),
        Xnor => Some(!(a ^ b)),
        Nor => Some(!(a | b)),
        Sl => Some(a.wrapping_shl(b & 31)),
        Sr => Some(((a as i32) >> (b & 31)) as u32),
        Sru => Some(a >> (b & 31)),
        Cmpe => Some(u32::from(a == b)),
        Cmpne => Some(u32::from(a != b)),
        Cmpg => Some(u32::from((a as i32) > (b as i32))),
        Cmpge => Some(u32::from((a as i32) >= (b as i32))),
        Cmpgu => Some(u32::from(a > b)),
        Cmpgeu => Some(u32::from(a >= b)),
        _ => unreachable!(),
    }
}

#[test]
fn independent_manual_oracle_matches_672_complete_state_transitions() {
    let operations = [
        (0x2d, Operation::Add),
        (0x32, Operation::Sub),
        (0x22, Operation::Mul),
        (0x23, Operation::Divu),
        (0x31, Operation::Modu),
        (0x28, Operation::And),
        (0x2e, Operation::Or),
        (0x26, Operation::Xor),
        (0x29, Operation::Xnor),
        (0x21, Operation::Nor),
        (0x2f, Operation::Sl),
        (0x25, Operation::Sr),
        (0x20, Operation::Sru),
        (0x39, Operation::Cmpe),
        (0x3f, Operation::Cmpne),
        (0x3a, Operation::Cmpg),
        (0x3b, Operation::Cmpge),
        (0x3d, Operation::Cmpgu),
        (0x3c, Operation::Cmpgeu),
    ];
    let corpus = include_str!("vectors/manual-operands.csv");
    let mut seeds = 0;
    let mut transitions = 0;
    for line in corpus.lines().skip(1).filter(|line| !line.is_empty()) {
        let (a, b) = line.split_once(',').unwrap();
        let a = u32::from_str_radix(a, 16).unwrap();
        let b = u32::from_str_radix(b, 16).unwrap();
        seeds += 1;
        for (opcode, operation) in operations {
            let mut cpu = loaded(reg(opcode), a, b);
            let mut expected = cpu.snapshot().unwrap();
            let trace = cpu.step().unwrap();
            if let Some(value) = expected_binary(operation, a, b) {
                expected.pc = 0x104;
                expected.gpr[3] = value;
                assert!(trace.exception.is_none());
            } else {
                expected.pc = u32::from(EXCEPTION_DIVIDE_BY_ZERO) * 32;
                expected.gpr[30] = 0x100;
                assert_eq!(trace.exception.unwrap().id, EXCEPTION_DIVIDE_BY_ZERO);
            }
            assert_eq!(
                trace.state_after, expected,
                "{operation:?} a={a:#010x} b={b:#010x}"
            );
            assert_eq!(trace.transactions.len(), 1);
            assert!(trace.transactions[0].success);
            transitions += 1;
        }

        for (opcode, operation, value) in [
            (0x2c, Operation::Sextb, (a as u8 as i8 as i32) as u32),
            (0x37, Operation::Sexth, (a as u16 as i16 as i32) as u32),
        ] {
            let mut cpu = loaded(reg(opcode), a, b);
            let mut expected = cpu.snapshot().unwrap();
            expected.pc = 0x104;
            expected.gpr[3] = value;
            let trace = cpu.step().unwrap();
            assert_eq!(trace.state_after, expected, "{operation:?} a={a:#010x}");
            transitions += 1;
        }
    }
    assert_eq!(seeds, 32);
    assert_eq!(transitions, 672);
}
