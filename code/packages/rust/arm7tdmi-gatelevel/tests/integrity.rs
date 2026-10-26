use std::fs;
use std::path::PathBuf;
use std::process::Command;

const ENGINE: &str = include_str!("../src/engine.rs");
const GATES: &str = include_str!("../src/gates.rs");
const LIB: &str = include_str!("../src/lib.rs");
const STATE: &str = include_str!("../src/state.rs");

const CONDITION: &str = r#"fn condition_passed(cpsr: u32, condition: u8) -> bool {
    gates::condition_holds(cpsr, condition) == 1
}"#;
const ADD: &str = r#"fn add_with_carry(left: u32, right: u32, carry: bool) -> (u32, bool, bool) {
    let (result, carry, overflow) = gates::add_32(left, right, u8::from(carry));
    (result, carry == 1, overflow == 1)
}"#;
const SHIFT: &str = r#"fn shifted(value: u32, kind: u32, amount: u32, immediate: bool, old_carry: bool) -> (u32, bool) {
    gates::arm_shift(value, kind, amount, immediate, old_carry)
}"#;
const CLOCK_BIT: &str = r#"pub(crate) fn clock_bit(q: &mut u8, value: bool) {
    let input = [u8::from(value)];
    let mut state = [stable_state(*q)];
    register(&input, 0, &mut state);
    *q = register(&input, 1, &mut state)[0];
}"#;
const MEMORY_WRITE: &str = r#"let old = self.q[address];
        let input: [u8; 8] = std::array::from_fn(|bit| (value >> bit) & 1);
        let mut state: [FlipFlopState; 8] =
            std::array::from_fn(|bit| stable_state((old >> bit) & 1));
        register(&input, 0, &mut state);
        self.q[address] = register(&input, 1, &mut state)
            .iter()
            .enumerate()
            .fold(0, |byte, (bit, q)| byte | (q << bit));"#;

// These anchors span the reviewed source-to-sink boundary.  They deliberately
// lock the decoded family wires, operation wires, mux/control consumers,
// architectural read routes, and DFF/public commit routes separately: keeping
// a dead gate expression beside a host-authoritative result cannot satisfy the
// whole set.
const ARM_FAMILY_ROUTE: &str = r#"let family = gates::field(raw, 25, 3);
        if gates::selected(family, 0, 3) | gates::selected(family, 1, 3) == 1 {
            execute_data_processing(state, raw, pc)?
        } else if gates::selected(family, 2, 3) | gates::selected(family, 3, 3) == 1 {
            execute_single_transfer(state, raw, pc)?
        } else if gates::selected(family, 4, 3) == 1 {
            execute_block_transfer(state, raw, pc)?
        } else if gates::selected(family, 5, 3) == 1 {
            execute_branch(state, raw, pc)?"#;
const THUMB_FAMILY_ROUTE: &str = r#"let family = gates::field(word, 13, 3);
    let mnemonic = if gates::selected(family, 0, 3) == 1 {
        execute_thumb_shift_add(state, raw, pc)?
    } else if gates::selected(family, 1, 3) == 1 {
        execute_thumb_immediate(state, raw)?
    } else if gates::selected(family, 2, 3) == 1 {
        execute_thumb_data_or_register_transfer(state, raw, pc)?
    } else if gates::selected(family, 3, 3) == 1 {
        execute_thumb_immediate_transfer(state, raw, false)?"#;
const FLAG_ROUTE: &str = r#"fn set_nzcv(state: &mut Arm7TdmiState, value: u32, carry: bool, overflow: bool) {
    state.cpsr = gates::status_with_nzcv(state.cpsr, value, u8::from(carry), u8::from(overflow));
}"#;
const BANK_READ_ROUTE: &str = r#"0..=7 => state.registers.common_r0_r7[register],
        8..=12 if mode(state) == Mode::Fiq => state.registers.fiq_r8_r14[register - 8],
        8..=12 => state.registers.normal_r8_r12[register - 8],
        13..=14 => bank_13_14(&state.registers, mode(state))[register - 13],
        15 => state.registers.pc,"#;
const BANK_WRITE_ROUTE: &str = r#"0..=7 => state.registers.common_r0_r7[register] = value,
        8..=12 if current_mode == Mode::Fiq => state.registers.fiq_r8_r14[register - 8] = value,
        8..=12 => state.registers.normal_r8_r12[register - 8] = value,
        13..=14 => bank_13_14_mut(&mut state.registers, current_mode)[register - 13] = value,
        15 => state.registers.pc = value,"#;
const STATUS_READ_ROUTE: &str = r#"Mode::Fiq => Ok(state.spsr_fiq),
        Mode::Irq => Ok(state.spsr_irq),
        Mode::Supervisor => Ok(state.spsr_supervisor),
        Mode::Abort => Ok(state.spsr_abort),
        Mode::Undefined => Ok(state.spsr_undefined),
        mode @ (Mode::User | Mode::System) => Err(Arm7TdmiError::UnavailableSavedStatus { mode }),"#;
const STATUS_WRITE_ROUTE: &str = r#"Mode::Fiq => state.spsr_fiq = value,
        Mode::Irq => state.spsr_irq = value,
        Mode::Supervisor => state.spsr_supervisor = value,
        Mode::Abort => state.spsr_abort = value,
        Mode::Undefined => state.spsr_undefined = value,"#;
const PC_STATE_ROUTE: &str = r#"if target & 1 != 0 {
        let exchanged = target & !1;
        checked_target(exchanged, InstructionSet::Thumb)?;
        state.cpsr |= CPSR_T;
        state.registers.pc = exchanged;
    } else {
        checked_target(target, InstructionSet::Arm)?;
        state.cpsr &= !CPSR_T;
        state.registers.pc = target;
    }"#;
const MEMORY_READ_ROUTE: &str = r#"check_alignment(address, width)?;
    let range = checked_range(address, width)?;
    Ok(match width {
        1 => state.memory[range.start] as u32,
        2 => u32::from(gates::assemble_le_u16(
            state.memory[range].try_into().expect("halfword range"),
        )),
        4 => gates::assemble_le_u32(state.memory[range].try_into().expect("word range")),"#;
const MEMORY_CONTROL_ROUTE: &str = r#"check_alignment(address, width)?;
    let range = checked_range(address, width)?;
    let bytes = gates::split_le_u32(value);
    state.memory[range].copy_from_slice(&bytes[..width]);"#;
const STABLE_Q_READ: &str = r#"bits.iter()
        .enumerate()
        .fold(0, |value, (bit, q)| value | (u64::from(*q) << bit))"#;
const GATE_DECODE: &str = r#"(0..32).fold(1, |line, bit| {
        let equal = xnor_gate(value[bit], expected[bit]);
        and_gate(line, mux2(1, equal, mask[bit]))
    })"#;
const GATE_ADD: &str = r#"let full = ripple_carry_adder_with_carry(&left_bits, &right_bits, carry_in);
    let low = ripple_carry_adder_with_carry(&left_bits[..31], &right_bits[..31], carry_in);"#;
const GATE_FLAGS: &str = r#"status[31] = value[31];
    status[30] = not_gate(value.into_iter().fold(0, or_gate));"#;
const GATE_SHIFT_OUTPUT: &str = r#"let result = (0..4).fold(0, |out, index| {
        bit_or_32(out, select_32(0, outputs[index], kind_lines[index]))
    });
    let carry = (0..4).fold(0, |out, index| {
        or_gate(out, and_gate(carries[index], kind_lines[index]))
    });"#;

fn one(source: &str, needle: &str) -> Result<(), String> {
    if source.matches(needle).count() == 1 {
        Ok(())
    } else {
        Err(format!(
            "expected one exact authority chain, found {}",
            source.matches(needle).count()
        ))
    }
}

fn exact_count(source: &str, needle: &str, expected: usize) -> Result<(), String> {
    let actual = source.matches(needle).count();
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected} occurrences, found {actual}"))
    }
}

fn validate(engine: &str, gates: &str, lib: &str, state: &str) -> Result<(), String> {
    syn::parse_file(engine).map_err(|error| error.to_string())?;
    syn::parse_file(gates).map_err(|error| error.to_string())?;
    syn::parse_file(lib).map_err(|error| error.to_string())?;
    syn::parse_file(state).map_err(|error| error.to_string())?;
    for (name, contract) in [
        ("condition", CONDITION),
        ("add", ADD),
        ("shift", SHIFT),
        ("arm family route", ARM_FAMILY_ROUTE),
        ("thumb family route", THUMB_FAMILY_ROUTE),
        ("flag route", FLAG_ROUTE),
        ("bank read route", BANK_READ_ROUTE),
        ("bank write route", BANK_WRITE_ROUTE),
        ("status read route", STATUS_READ_ROUTE),
        ("status write route", STATUS_WRITE_ROUTE),
        ("PC/state route", PC_STATE_ROUTE),
        ("memory read route", MEMORY_READ_ROUTE),
        ("memory control route", MEMORY_CONTROL_ROUTE),
    ] {
        one(engine, contract).map_err(|error| format!("{name}: {error}"))?;
    }
    exact_count(engine, "gates::selected(opcode", 16)
        .map_err(|error| format!("complete operation-wire route: {error}"))?;
    exact_count(engine, "gates::selected(family", 14)
        .map_err(|error| format!("complete family-wire route: {error}"))?;
    for (name, contract) in [
        ("masked decoder", GATE_DECODE),
        ("ripple arithmetic", GATE_ADD),
        ("flag datapath", GATE_FLAGS),
        ("barrel output mux", GATE_SHIFT_OUTPUT),
    ] {
        one(gates, contract).map_err(|error| format!("{name}: {error}"))?;
    }
    for (name, contract) in [("clock_bit", CLOCK_BIT), ("memory_write", MEMORY_WRITE)] {
        one(state, contract).map_err(|error| format!("{name}: {error}"))?;
    }
    one(state, STABLE_Q_READ).map_err(|error| format!("stable-Q read: {error}"))?;
    one(state, "register(&input, 0, &mut state);\n    q.copy_from_slice(&register(&input, 1, &mut state));").map_err(|e| format!("clock_bits: {e}"))?;
    one(lib, "self.clock_complete_state(&Arm7TdmiState::default());")
        .map_err(|e| format!("reset: {e}"))?;
    one(
        lib,
        "validator.restore(snapshot.clone())?;\n        self.commit_transition(&snapshot)?;",
    )
    .map_err(|e| format!("restore: {e}"))?;
    one(lib, "candidate.load(program, origin, instruction_set)?;\n        self.commit_transition(&candidate.get_state())?;").map_err(|e| format!("load: {e}"))?;
    one(lib, "self.state_with_memory(self.memory.snapshot())")
        .map_err(|e| format!("snapshot: {e}"))?;
    one(lib, "engine.restore(self.get_state())?;")
        .map_err(|e| format!("stable-Q engine entry: {e}"))?;
    one(lib, "candidate.write_register(register, value)?;\n        self.commit_transition(&candidate.get_state())?;").map_err(|e| format!("register write: {e}"))?;
    one(lib, "candidate.write_memory(address, bytes)?;\n        self.commit_transition(&candidate.get_state())?;").map_err(|e| format!("memory write: {e}"))?;
    one(
        lib,
        "registers,\n            cpsr: read_bits(&self.cpsr) as u32,",
    )
    .map_err(|e| format!("status observation: {e}"))?;
    one(lib, "let mut candidate = self.engine()?;\n        let trace = candidate.step()?;\n        self.commit_transition(&trace.state_after)?;").map_err(|e| format!("step: {e}"))?;
    one(lib, "let mut candidate = self.engine()?;\n        let result = candidate.run(max_steps)?;\n        self.commit_transition(&result.final_state)?;").map_err(|e| format!("run: {e}"))?;
    one(lib, "let mut candidate = self.engine()?;\n        let result = candidate.execute(program, origin, instruction_set, max_steps)?;\n        self.commit_transition(&result.final_state)?;").map_err(|e| format!("execute: {e}"))?;
    one(
        lib,
        "clock_bits(&mut self.loaded_len, next.loaded_len as u64);\n    }",
    )
    .map_err(|e| format!("complete commit: {e}"))?;
    one(lib, "clock_bits(\n                &mut self.registers[0],\n                u64::from(next.registers.common_r0_r7[0]),\n            );\n            self.clock_complete_state(&before);\n            return Err(Arm7TdmiError::CommitFailed);").map_err(|e| format!("rollback: {e}"))?;
    if [engine, gates, lib, state]
        .iter()
        .any(|source| source.contains("arm7tdmi_simulator") || source.contains("oracle::"))
    {
        return Err("production source references the functional oracle".into());
    }
    Ok(())
}

fn replace_once(source: &str, old: &str, new: &str) -> String {
    assert_eq!(source.matches(old).count(), 1, "hostile mutation anchor");
    source.replacen(old, new, 1)
}

fn compile_variant(engine: &str, gates: &str, lib: &str, state: &str) {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let nonce = format!("{}-{:?}", std::process::id(), std::thread::current().id())
        .replace(['(', ')', ' '], "");
    let root = std::env::temp_dir().join(format!("arm7tdmi-gate-hostile-{nonce}"));
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    let packages = package.parent().unwrap();
    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname='arm7tdmi-gate-hostile'\nversion='0.0.0'\nedition='2021'\n[lib]\npath='src/lib.rs'\n[dependencies]\narithmetic={{path='{}'}}\nlogic-gates={{path='{}'}}\n",
            packages.join("arithmetic").display(),
            packages.join("logic-gates").display()
        ),
    )
    .unwrap();
    fs::write(src.join("engine.rs"), engine).unwrap();
    fs::write(src.join("lib.rs"), lib).unwrap();
    fs::write(src.join("state.rs"), state).unwrap();
    fs::write(src.join("gates.rs"), gates).unwrap();
    let output = Command::new("cargo")
        .args(["check", "--quiet", "--offline", "--manifest-path"])
        .arg(root.join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            package.parent().unwrap().join("target/hostile-check"),
        )
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&root);
    assert!(
        output.status.success(),
        "hostile fixture must compile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn reviewed_source_to_sink_authority_chains_are_exact() {
    validate(ENGINE, GATES, LIB, STATE).unwrap();
}

#[test]
fn compile_valid_hostile_condition_and_helper_bypasses_are_rejected() {
    let host = replace_once(
        CONDITION,
        "gates::condition_holds(cpsr, condition) == 1",
        "condition == 14 || (cpsr & (1 << 30) != 0 && condition == 0)",
    );
    let mutated = replace_once(ENGINE, CONDITION, &host);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert!(validate(&mutated, GATES, LIB, STATE).is_err());

    let indirect = replace_once(
        CONDITION,
        "gates::condition_holds(cpsr, condition) == 1",
        "host_condition(cpsr, condition)",
    );
    let indirect = format!("{indirect}\nfn host_condition(cpsr: u32, condition: u8) -> bool {{ condition == 14 || (condition == 0 && cpsr & (1 << 30) != 0) }}");
    let mutated = replace_once(ENGINE, CONDITION, &indirect);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert!(validate(&mutated, GATES, LIB, STATE).is_err());
}

#[test]
fn compile_valid_hostile_arithmetic_decode_and_oracle_bypasses_are_rejected() {
    let host_add = replace_once(
        ADD,
        "let (result, carry, overflow) = gates::add_32(left, right, u8::from(carry));\n    (result, carry == 1, overflow == 1)",
        "let carry = u32::from(carry);\n    (left.wrapping_add(right).wrapping_add(carry), false, false)",
    );
    let mutated = replace_once(ENGINE, ADD, &host_add);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert!(validate(&mutated, GATES, LIB, STATE).is_err());

    let host_decode = replace_once(
        ARM_FAMILY_ROUTE,
        "if gates::selected(family, 0, 3) | gates::selected(family, 1, 3) == 1",
        "if family == 0 || family == 1",
    );
    let mutated = replace_once(ENGINE, ARM_FAMILY_ROUTE, &host_decode);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert!(validate(&mutated, GATES, LIB, STATE).is_err());

    let oracle_condition = replace_once(
        CONDITION,
        "gates::condition_holds(cpsr, condition) == 1",
        "oracle::condition_holds(cpsr, condition)",
    );
    let oracle_condition = format!(
        "{oracle_condition}\nmod oracle {{ pub(super) fn condition_holds(cpsr: u32, condition: u8) -> bool {{ condition == 14 || (condition == 0 && cpsr & (1 << 30) != 0) }} }}"
    );
    let mutated = replace_once(ENGINE, CONDITION, &oracle_condition);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert!(validate(&mutated, GATES, LIB, STATE).is_err());
}

#[test]
fn compile_valid_hostile_dead_anchor_and_post_gate_overwrite_are_rejected() {
    let dead_anchor = replace_once(
        ADD,
        "let (result, carry, overflow) = gates::add_32(left, right, u8::from(carry));\n    (result, carry == 1, overflow == 1)",
        "let _gate_anchor = gates::add_32(left, right, u8::from(carry));\n    (left.wrapping_add(right).wrapping_add(u32::from(carry)), false, false)",
    );
    let mutated = replace_once(ENGINE, ADD, &dead_anchor);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert!(validate(&mutated, GATES, LIB, STATE).is_err());

    let post_gate = replace_once(
        ADD,
        "let (result, carry, overflow) = gates::add_32(left, right, u8::from(carry));\n    (result, carry == 1, overflow == 1)",
        "let (mut result, carry, overflow) = gates::add_32(left, right, u8::from(carry));\n    result = left.wrapping_add(right).wrapping_add(u32::from(carry));\n    (result, carry == 1, overflow == 1)",
    );
    let mutated = replace_once(ENGINE, ADD, &post_gate);
    compile_variant(&mutated, GATES, LIB, STATE);
    assert!(validate(&mutated, GATES, LIB, STATE).is_err());
}

#[test]
fn compile_valid_hostile_direct_q_and_post_commit_bypasses_are_rejected() {
    let direct = replace_once(CLOCK_BIT, "register(&input, 0, &mut state);\n    *q = register(&input, 1, &mut state)[0];", "register(&input, 0, &mut state);\n    let _anchor = register(&input, 1, &mut state)[0];\n    *q = u8::from(value);");
    let mutated_state = replace_once(STATE, CLOCK_BIT, &direct);
    compile_variant(ENGINE, GATES, LIB, &mutated_state);
    assert!(validate(ENGINE, GATES, LIB, &mutated_state).is_err());

    let direct_memory = replace_once(MEMORY_WRITE, "self.q[address] = register(&input, 1, &mut state)\n            .iter()\n            .enumerate()\n            .fold(0, |byte, (bit, q)| byte | (q << bit));", "let _anchor = register(&input, 1, &mut state);\n        self.q[address] = value;");
    let mutated_state = replace_once(STATE, MEMORY_WRITE, &direct_memory);
    compile_variant(ENGINE, GATES, LIB, &mutated_state);
    assert!(validate(ENGINE, GATES, LIB, &mutated_state).is_err());

    let mutated_lib = replace_once(LIB, "clock_bits(&mut self.loaded_len, next.loaded_len as u64);\n    }", "clock_bits(&mut self.loaded_len, next.loaded_len as u64);\n        self.halted = 0;\n    }");
    compile_variant(ENGINE, GATES, &mutated_lib, STATE);
    assert!(validate(ENGINE, GATES, &mutated_lib, STATE).is_err());
}
