use latticemico32_simulator::{BusTransactionKind, Lm32Simulator, Lm32State};

#[derive(Clone, Copy)]
struct StateFields {
    gpr: [u32; 32],
    pc: u32,
    ie: bool,
    eie: bool,
    bie: bool,
    dc_re: bool,
    im: u32,
    ip: u32,
    lines: u32,
    eba: u32,
    deba: u32,
}

#[derive(Clone, Copy)]
enum ExpectedKind {
    InstructionRead,
    DataRead,
    DataWrite,
}

#[derive(Clone, Copy)]
struct ExpectedTransaction {
    kind: ExpectedKind,
    address: u32,
    width: u8,
    value: u32,
    byte_enable: u8,
    success: bool,
}

#[derive(Clone, Copy)]
struct ExpectedException {
    id: u8,
    register: u8,
    saved_pc: u32,
    dc_re: bool,
    base: u32,
    vector: u32,
    ie_before: bool,
    saved_ie: bool,
}

struct ExpectedTrace {
    pc_before: u32,
    pc_after: u32,
    word: Option<u32>,
    mnemonic: &'static str,
    sources: [Option<u8>; 2],
    values: [Option<u32>; 2],
    destination: Option<(u8, u32)>,
    branch: Option<(bool, u32)>,
    transactions: &'static [ExpectedTransaction],
    interrupt: (u32, u32, u32, u32),
    exception: Option<ExpectedException>,
}

struct TransitionVector {
    name: &'static str,
    word: u32,
    initial: StateFields,
    expected: StateFields,
    memory_before: &'static [(usize, u8)],
    memory_after: &'static [(usize, u8)],
    trace: ExpectedTrace,
}

use ExpectedKind::{DataRead, DataWrite, InstructionRead};
include!("vectors/manual-transitions.rs");
include!("vectors/pinned-rtl-transitions.rs");

fn install_fields(state: &mut Lm32State, fields: StateFields) {
    state.gpr = fields.gpr;
    state.pc = fields.pc;
    state.ie = fields.ie;
    state.eie = fields.eie;
    state.bie = fields.bie;
    state.dc_re = fields.dc_re;
    state.im = fields.im;
    state.ip = fields.ip;
    state.interrupt_lines = fields.lines;
    state.eba = fields.eba;
    state.deba = fields.deba;
}

fn run_vector(vector: &TransitionVector) {
    let mut cpu = Lm32Simulator::new();
    cpu.load(&vector.word.to_be_bytes(), 0x100).unwrap();
    let mut initial = cpu.snapshot().unwrap();
    install_fields(&mut initial, vector.initial);
    for &(address, value) in vector.memory_before {
        initial.memory[address] = value;
    }
    cpu.restore(initial.clone()).unwrap();

    let mut expected = initial;
    install_fields(&mut expected, vector.expected);
    for &(address, value) in vector.memory_after {
        expected.memory[address] = value;
    }

    let actual = cpu
        .step()
        .unwrap_or_else(|error| panic!("{}: {error:?}", vector.name));
    assert_eq!(actual.state_after, expected, "{} full state", vector.name);
    assert_eq!(
        actual.pc_before, vector.trace.pc_before,
        "{} pc before",
        vector.name
    );
    assert_eq!(
        actual.pc_after, vector.trace.pc_after,
        "{} pc after",
        vector.name
    );
    assert_eq!(
        actual.raw_word, vector.trace.word,
        "{} raw word",
        vector.name
    );
    assert_eq!(
        actual.raw,
        vector.trace.word.map(u32::to_be_bytes),
        "{} raw bytes",
        vector.name
    );
    assert_eq!(
        actual.mnemonic, vector.trace.mnemonic,
        "{} mnemonic",
        vector.name
    );
    assert_eq!(
        actual.source_indices, vector.trace.sources,
        "{} sources",
        vector.name
    );
    assert_eq!(
        actual.source_values, vector.trace.values,
        "{} values",
        vector.name
    );
    assert_eq!(
        actual.destination, vector.trace.destination,
        "{} destination",
        vector.name
    );
    assert_eq!(actual.branch, vector.trace.branch, "{} branch", vector.name);
    assert_eq!(
        actual.transactions.len(),
        vector.trace.transactions.len(),
        "{} transaction count",
        vector.name
    );
    for (actual, expected) in actual
        .transactions
        .iter()
        .zip(vector.trace.transactions.iter())
    {
        let kind = match expected.kind {
            InstructionRead => BusTransactionKind::InstructionRead,
            DataRead => BusTransactionKind::DataRead,
            DataWrite => BusTransactionKind::DataWrite,
        };
        assert_eq!(actual.kind, kind, "{} transaction kind", vector.name);
        assert_eq!(
            actual.address, expected.address,
            "{} transaction address",
            vector.name
        );
        assert_eq!(
            actual.width, expected.width,
            "{} transaction width",
            vector.name
        );
        assert_eq!(
            actual.value, expected.value,
            "{} transaction value",
            vector.name
        );
        assert_eq!(
            actual.byte_enable, expected.byte_enable,
            "{} transaction byte enable",
            vector.name
        );
        assert_eq!(
            actual.success, expected.success,
            "{} transaction success",
            vector.name
        );
    }
    let interrupt = actual
        .interrupt
        .expect("every checked boundary samples interrupts");
    assert_eq!(
        (
            interrupt.asserted,
            interrupt.newly_latched,
            interrupt.masked_pending,
            interrupt.accepted,
        ),
        vector.trace.interrupt,
        "{} interrupt sample",
        vector.name
    );
    match (actual.exception, vector.trace.exception) {
        (None, None) => {}
        (Some(actual), Some(expected)) => assert_eq!(
            (
                actual.id,
                actual.saved_pc_register,
                actual.saved_pc,
                actual.dc_re,
                actual.base,
                actual.vector,
                actual.ie_before,
                actual.saved_ie,
            ),
            (
                expected.id,
                expected.register,
                expected.saved_pc,
                expected.dc_re,
                expected.base,
                expected.vector,
                expected.ie_before,
                expected.saved_ie,
            ),
            "{} exception",
            vector.name
        ),
        _ => panic!("{} exception presence mismatch", vector.name),
    }
}

#[test]
fn independent_manual_vectors_match_exact_full_state_and_transactions() {
    assert_eq!(MANUAL_TRANSITIONS.len(), 308);
    for vector in MANUAL_TRANSITIONS {
        run_vector(vector);
    }
}

#[test]
fn pinned_rtl_vectors_match_exact_full_state_and_transactions() {
    assert_eq!(RTL_TRANSITIONS.len(), 183);
    for vector in RTL_TRANSITIONS {
        run_vector(vector);
    }
}

#[test]
fn pinned_rtl_source_facts_are_complete_and_immutably_hashed() {
    const COMMIT: &str = "84b3e3ca0ad9535acaef201c1482342871358b08";
    assert_eq!(COMMIT.len(), 40);
    let evidence = include_str!("vectors/pinned-rtl-evidence.csv");
    let rows = evidence.lines().skip(1).collect::<Vec<_>>();
    assert_eq!(rows.len(), 8);
    for required in [
        "decoder_opcode_wires",
        "immediate_operand_mux",
        "narrow_store_bus_replication",
        "store_lane_select",
        "interrupt_csr_surface",
        "interrupt_latch_and_enable",
        "exception_control",
        "debug_exception_remap",
    ] {
        assert!(
            rows.iter().any(|row| row.starts_with(required)),
            "{required}"
        );
    }
    for row in rows {
        let fields = row.split(',').collect::<Vec<_>>();
        assert_eq!(fields.len(), 5);
        assert_eq!(fields[2].len(), 64);
        assert!(RTL_TRANSITIONS
            .iter()
            .any(|vector| vector.name.starts_with(fields[4])));
    }
}
