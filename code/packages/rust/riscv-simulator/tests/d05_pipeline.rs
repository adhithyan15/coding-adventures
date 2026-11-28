#[path = "support/d05_pipeline_adapter.rs"]
mod adapter;

use adapter::{D05InitialWord, D05PipelineError, run_d05_pipeline};
use riscv_simulator::Rv32ISimulator;

const INPUT_JSON: &str =
    include_str!("../../../../specs/fixtures/go-riscv-d05-v1/input.json");

fn fixture() -> (Vec<u8>, usize, usize, Vec<D05InitialWord>) {
    let value: serde_json::Value = serde_json::from_str(INPUT_JSON).expect("valid fixture JSON");
    assert_eq!(value["schema"], "go-riscv-d05-pipeline-input-v1");
    assert_eq!(value["id"], "dependent-store-load-branch");

    let hex = value["program_bytes_hex"].as_str().expect("program hex string");
    assert_eq!(hex.len() % 2, 0);
    let program = hex
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let byte = std::str::from_utf8(pair).expect("ASCII hex");
            u8::from_str_radix(byte, 16).expect("valid hex byte")
        })
        .collect();
    let ram_bytes = value["ram_bytes"].as_u64().expect("RAM byte count") as usize;
    let max_cycles = value["max_cycles"].as_u64().expect("cycle limit") as usize;
    let words = value["initial_memory_words"]
        .as_array()
        .expect("initial word array")
        .iter()
        .map(|word| D05InitialWord {
            address: word["address"].as_u64().expect("word address") as usize,
            value: word["value"].as_u64().expect("word value") as u32,
        })
        .collect();
    (program, ram_bytes, max_cycles, words)
}

#[test]
fn canonical_program_runs_through_core_and_matches_sequential_rv32i_state() {
    let (program, ram_bytes, max_cycles, initial_words) = fixture();
    let result = run_d05_pipeline(&program, ram_bytes, &initial_words, max_cycles)
        .expect("bounded D05 fixture completes");

    assert_eq!(
        result.retired_pcs,
        [0, 4, 8, 12, 16, 20, 24, 32, 36],
        "WB must retire the target instruction and suppress wrong-path PC 28"
    );
    assert!(!result.retired_pcs.contains(&28));
    assert_eq!(result.total_cycles, result.cycles.len());
    assert!(result.total_cycles <= max_cycles);
    assert_eq!(result.stall_count, 1, "load-use creates one hold");
    assert_eq!(result.flush_count, 1, "taken branch redirects once");
    assert_eq!(
        result.cycles.iter().filter(|cycle| cycle.snapshot.stalled).count(),
        1
    );
    assert_eq!(
        result.cycles.iter().filter(|cycle| cycle.snapshot.flushing).count(),
        1
    );

    let store_has_two_distinct_sources = result.cycles.iter().any(|cycle| {
        let store_in_execute = cycle
            .snapshot
            .stages
            .get("EX")
            .is_some_and(|token| token.pc == 12 && token.opcode == "sw");
        store_in_execute
            && cycle.forwarded_operands.iter().any(|forward| {
                forward.operand == cpu_pipeline::OperandSlot::Rs1
                    && forward.source_stage == cpu_pipeline::OperandForwardingStage::Execute
            })
            && cycle.forwarded_operands.iter().any(|forward| {
                forward.operand == cpu_pipeline::OperandSlot::Rs2
                    && forward.source_stage == cpu_pipeline::OperandForwardingStage::Memory
            })
    });
    assert!(store_has_two_distinct_sources, "SW receives independent EX and MEM values");

    let load_result_reaches_branch = result.cycles.iter().any(|cycle| {
        cycle
            .snapshot
            .stages
            .get("EX")
            .is_some_and(|token| token.pc == 24 && token.opcode == "beq")
            && cycle.forwarded_operands.iter().any(|forward| {
                forward.operand == cpu_pipeline::OperandSlot::Rs1
                    && forward.register == 3
                    && forward.source_stage == cpu_pipeline::OperandForwardingStage::Memory
            })
    });
    assert!(load_result_reaches_branch, "taken branch consumes the loaded value");
    assert!(result
        .cycles
        .last()
        .and_then(|cycle| cycle.snapshot.stages.get("WB"))
        .is_some_and(|token| token.pc == 36 && token.is_halt));

    let mut reference = Rv32ISimulator::new();
    reference.load_checked(&program).expect("valid oracle program");
    for word in &initial_words {
        for (offset, byte) in word.value.to_le_bytes().into_iter().enumerate() {
            reference
                .write_byte((word.address + offset) as u32, byte)
                .expect("initial word fits oracle RAM");
        }
    }
    let expected = reference
        .run_checked(max_cycles)
        .expect("sequential oracle halts within fixture limit");
    assert!(expected.halted);
    assert_eq!(result.registers, expected.final_state.registers);
    assert_eq!(result.memory, expected.final_state.memory[..ram_bytes]);
    assert_eq!(&result.memory[128..132], &[0, 0, 0, 0]);
}

#[test]
fn adapter_rejects_non_ecall_system_encoding_before_running() {
    for raw in [0x0010_0073u32, 0x3000_1073u32] {
        let err = run_d05_pipeline(&raw.to_le_bytes(), 256, &[], 100)
            .expect_err("unsupported system instruction");
        assert!(matches!(
            err,
            D05PipelineError::UnsupportedInstruction { .. }
        ));
    }
}

#[test]
fn forwarded_and_committed_registers_compare_as_rv32_bit_patterns() {
    use riscv_simulator::encoding::{assemble, encode_addi, encode_beq, encode_ecall};

    let program = assemble(&[
        encode_addi(2, 0, -1),
        encode_addi(1, 0, -1),
        encode_beq(1, 2, 8),
        encode_addi(3, 0, 99),
        encode_addi(4, 1, 1),
        encode_ecall(),
    ]);
    let result = run_d05_pipeline(&program, 256, &[], 100).unwrap();
    assert_eq!(result.retired_pcs, [0, 4, 8, 16, 20]);
    assert_eq!(result.registers[1], u32::MAX);
    assert_eq!(result.registers[2], u32::MAX);
    assert_eq!(result.registers[3], 0, "wrong path did not execute");
    assert_eq!(result.registers[4], 0, "RV32 addition wraps at 32 bits");

    let mut oracle = Rv32ISimulator::new();
    oracle.load_checked(&program).unwrap();
    let expected = oracle.run_checked(100).unwrap();
    assert_eq!(result.registers, expected.final_state.registers);
    assert_eq!(result.memory, expected.final_state.memory[..256]);
}
