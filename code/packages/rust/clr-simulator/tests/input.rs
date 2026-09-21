use clr_simulator::{
    CLRSimulator, Value, BASIC_INPUT_I64_TOKEN, BASIC_INPUT_MORE_TOKEN, BASIC_INPUT_STR_TOKEN,
    OP_CALL, OP_RET,
};

fn call(token: u32, body: &mut Vec<u8>) {
    body.push(OP_CALL);
    body.extend_from_slice(&token.to_le_bytes());
}

fn run_simulator(input: &[u8], tokens: &[u32]) -> CLRSimulator {
    let mut body = Vec::new();
    for token in tokens {
        call(*token, &mut body);
    }
    body.push(OP_RET);
    let mut sim = CLRSimulator::new();
    sim.load(&body, 0);
    sim.set_input(input);
    sim.run(10_000);
    assert!(sim.halted);
    sim
}

fn run(input: &[u8], tokens: &[u32]) -> Vec<Option<Value>> {
    run_simulator(input, tokens).stack
}

#[test]
fn integer_input_consumes_lines_and_parses_the_full_i64_domain() {
    let input = b"-9223372036854775808\r\n +9223372036854775807 \ninvalid\n9223372036854775808\n7";
    assert_eq!(
        run(input, &[BASIC_INPUT_I64_TOKEN; 5]),
        vec![
            Some(Value::Int64(i64::MIN)),
            Some(Value::Int64(i64::MAX)),
            Some(Value::Int64(0)),
            Some(Value::Int64(0)),
            Some(Value::Int64(7)),
        ]
    );
}

#[test]
fn input_more_peeks_without_consuming_and_eof_reads_repeat_zero() {
    assert_eq!(
        run(
            b"41\n-2\n",
            &[
                BASIC_INPUT_MORE_TOKEN,
                BASIC_INPUT_MORE_TOKEN,
                BASIC_INPUT_I64_TOKEN,
                BASIC_INPUT_MORE_TOKEN,
                BASIC_INPUT_I64_TOKEN,
                BASIC_INPUT_MORE_TOKEN,
                BASIC_INPUT_I64_TOKEN,
                BASIC_INPUT_I64_TOKEN,
            ],
        ),
        vec![
            Some(Value::Int64(1)),
            Some(Value::Int64(1)),
            Some(Value::Int64(41)),
            Some(Value::Int64(1)),
            Some(Value::Int64(-2)),
            Some(Value::Int64(0)),
            Some(Value::Int64(0)),
            Some(Value::Int64(0)),
        ]
    );
    assert_eq!(
        run(b"\n", &[BASIC_INPUT_MORE_TOKEN, BASIC_INPUT_I64_TOKEN, BASIC_INPUT_MORE_TOKEN]),
        vec![
            Some(Value::Int64(1)),
            Some(Value::Int64(0)),
            Some(Value::Int64(0)),
        ]
    );
}

#[test]
fn load_does_not_rewind_input_and_replacement_does() {
    let mut first = Vec::new();
    call(BASIC_INPUT_I64_TOKEN, &mut first);
    first.push(OP_RET);
    let mut sim = CLRSimulator::new();
    sim.set_input(b"1\n2\n");
    sim.load(&first, 0);
    sim.run(10);
    assert_eq!(sim.stack, vec![Some(Value::Int64(1))]);

    sim.load(&first, 0);
    sim.run(10);
    assert_eq!(sim.stack, vec![Some(Value::Int64(2))]);

    sim.set_input(b"3");
    sim.load(&first, 0);
    sim.run(10);
    assert_eq!(sim.stack, vec![Some(Value::Int64(3))]);
}

#[test]
fn string_input_preserves_bytes_and_consumes_line_delimiters() {
    let sim = run_simulator(
        b"  hi \r\n\n\xff\t\nlast\rpart",
        &[
            BASIC_INPUT_MORE_TOKEN,
            BASIC_INPUT_STR_TOKEN,
            BASIC_INPUT_MORE_TOKEN,
            BASIC_INPUT_STR_TOKEN,
            BASIC_INPUT_STR_TOKEN,
            BASIC_INPUT_STR_TOKEN,
            BASIC_INPUT_MORE_TOKEN,
            BASIC_INPUT_STR_TOKEN,
            BASIC_INPUT_STR_TOKEN,
        ],
    );
    assert_eq!(sim.stack[0], Some(Value::Int64(1)));
    assert_eq!(sim.string_bytes(sim.stack[1].unwrap()), Some(&b"  hi "[..]));
    assert_eq!(sim.stack[2], Some(Value::Int64(1)));
    assert_eq!(sim.string_bytes(sim.stack[3].unwrap()), Some(&b""[..]));
    assert_eq!(sim.string_bytes(sim.stack[4].unwrap()), Some(&b"\xff\t"[..]));
    assert_eq!(
        sim.string_bytes(sim.stack[5].unwrap()),
        Some(&b"last\rpart"[..])
    );
    assert_eq!(sim.stack[6], Some(Value::Int64(0)));
    assert_eq!(sim.string_bytes(sim.stack[7].unwrap()), Some(&b""[..]));
    assert_eq!(sim.string_bytes(sim.stack[8].unwrap()), Some(&b""[..]));
}

#[test]
fn replacing_input_preserves_strings_until_the_next_program_load() {
    let mut body = Vec::new();
    call(BASIC_INPUT_STR_TOKEN, &mut body);
    body.push(OP_RET);
    let mut sim = CLRSimulator::new();
    sim.set_input(b"old");
    sim.load(&body, 0);
    sim.run(10);
    let old = sim.stack[0].unwrap();
    sim.set_input(b"new");
    assert_eq!(sim.string_bytes(old), Some(&b"old"[..]));
    sim.load(&body, 0);
    assert_eq!(sim.string_bytes(old), None);
    sim.run(10);
    assert_eq!(sim.string_bytes(sim.stack[0].unwrap()), Some(&b"new"[..]));
}

#[test]
fn unknown_memberref_refuses_before_state_or_input_changes() {
    let unknown = 0x0A00_0009_u32;
    let mut body = Vec::new();
    call(unknown, &mut body);
    let mut sim = CLRSimulator::new();
    sim.load(&body, 0);
    sim.set_input(b"42\n");
    sim.stack.push(Some(Value::Int(7)));
    let before = sim.stack.clone();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sim.step()));
    assert!(panic.is_err());
    assert_eq!(sim.pc, 0);
    assert_eq!(sim.stack, before);

    sim.bytecode.clear();
    call(BASIC_INPUT_I64_TOKEN, &mut sim.bytecode);
    sim.step();
    assert_eq!(sim.stack.last(), Some(&Some(Value::Int64(42))));
}
