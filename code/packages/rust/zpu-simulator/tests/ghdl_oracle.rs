use serde::Deserialize;
use zpu_simulator::{BusTransaction, ZpuSimulator, RESET_SP};

const CORPUS: &str = include_str!("fixtures/zpu4_small_oracle.json");
const EXTERNAL_VALUE: u32 = 0x89ab_cdef;

#[derive(Debug, Deserialize)]
struct Corpus {
    schema: String,
    source_commit: String,
    derived_config_sha256: String,
    initial_external_read_value: u32,
    isolated: Vec<Isolated>,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
struct Isolated {
    raw: String,
    mnemonic: String,
    pc_before: u16,
    sp_before: u16,
    top_before: u32,
    next_before: u32,
    pc_after: u16,
    sp_after: u16,
    top_after: u32,
    next_after: u32,
    idim_after: bool,
    in_interrupt_after: bool,
    break_after: bool,
    ram_writes: Vec<RamWrite>,
    bus_transaction: Option<CorpusBus>,
}

#[derive(Clone, Debug, Deserialize)]
struct RamWrite {
    port: String,
    address: usize,
    value: u32,
}

#[derive(Debug, Deserialize)]
struct CorpusBus {
    kind: String,
    address: u16,
    value: u32,
}

#[derive(Debug, Deserialize)]
struct Scenario {
    case: u16,
    name: String,
    boundaries: Vec<Boundary>,
    events: Vec<ScenarioEvent>,
}

#[derive(Debug, Deserialize)]
struct Boundary {
    raw: String,
    pc: u16,
    sp: u16,
    top: u32,
    next: u32,
    idim_after: bool,
    in_interrupt_after: bool,
    break_after: bool,
}

#[derive(Debug, Deserialize)]
struct ScenarioEvent {
    kind: String,
    address: Option<u16>,
    value: Option<u32>,
}

fn parse() -> Corpus {
    serde_json::from_str(CORPUS).expect("checked-in GHDL corpus must parse")
}

fn write_word(bytes: &mut [u8], word: usize, value: u32) {
    let address = word * 4;
    bytes[address..address + 4].copy_from_slice(&value.to_be_bytes());
}

fn initial_image(raw: u8, scenario: u8) -> Vec<u8> {
    let mut image = vec![0; 32_768];
    for word in 0..8192 {
        write_word(&mut image, word, 0x0b0b_0b0b);
    }
    for offset in 0..32 {
        write_word(
            &mut image,
            (8190 + offset) % 8192,
            0x4000_0000 + offset as u32,
        );
    }
    write_word(&mut image, 64, 0xa5a5_5a5a);
    write_word(&mut image, 8190, 0x0000_0100);
    write_word(&mut image, 8191, 0x1122_3344);
    write_word(&mut image, 0, (u32::from(raw) << 24) | 0x000b_0b0b);
    if matches!(scenario, 1 | 2) {
        write_word(&mut image, 8190, 0x0000_8000);
    } else if matches!(scenario, 3 | 4) {
        write_word(&mut image, 0, 0x8182_0b0b);
    }
    image
}

fn read_word(bytes: &[u8], address: u16) -> u32 {
    let start = usize::from(address);
    u32::from_be_bytes(bytes[start..start + 4].try_into().expect("word"))
}

fn assert_boundary(trace: &zpu_simulator::StepTrace, boundary: &Boundary) {
    assert_eq!(trace.pc_before, boundary.pc);
    assert_eq!(trace.sp_before, boundary.sp);
    assert_eq!(trace.top_before, boundary.top);
    assert_eq!(trace.next_before, boundary.next);
    assert_eq!(trace.idim_after, boundary.idim_after);
    assert_eq!(trace.in_interrupt_after, boundary.in_interrupt_after);
    assert_eq!(trace.break_after, boundary.break_after);
    if let Some(raw) = trace.raw {
        assert_eq!(format!("{raw:02x}"), boundary.raw);
    }
}

#[test]
fn all_256_bytes_match_checksum_pinned_ghdl_full_state() {
    let corpus = parse();
    assert_eq!(corpus.schema, "coding-adventures.zpu4-small-ghdl.v1");
    assert_eq!(
        corpus.source_commit,
        "e29aef5f7aa741557d0eb16ecf13e974d55789ff"
    );
    assert_eq!(
        corpus.derived_config_sha256,
        "166a3f446015b0efe4aad59728be76a297e6a70686ef1510fd33920147f1cdef"
    );
    assert_eq!(corpus.initial_external_read_value, EXTERNAL_VALUE);
    assert_eq!(corpus.isolated.len(), 256);

    for (raw, row) in (0_u8..=255).zip(&corpus.isolated) {
        assert_eq!(row.raw, format!("{raw:02x}"));
        let initial = initial_image(raw, 0);
        let mut cpu = ZpuSimulator::new();
        cpu.load(&initial, 0).expect("full internal image fits");
        cpu.write_external_word(0x8000, EXTERNAL_VALUE)
            .expect("external fixture address");
        let trace = cpu
            .step()
            .unwrap_or_else(|error| panic!("raw {raw:02x}: {error}"));

        assert_eq!(trace.mnemonic, row.mnemonic, "raw {raw:02x}");
        assert_eq!(trace.pc_before, row.pc_before, "raw {raw:02x}");
        assert_eq!(trace.sp_before, row.sp_before, "raw {raw:02x}");
        assert_eq!(trace.top_before, row.top_before, "raw {raw:02x}");
        assert_eq!(trace.next_before, row.next_before, "raw {raw:02x}");
        assert_eq!(trace.pc_after, row.pc_after, "raw {raw:02x}");
        assert_eq!(trace.sp_after, row.sp_after, "raw {raw:02x}");
        assert_eq!(trace.top_after, row.top_after, "raw {raw:02x}");
        assert_eq!(trace.next_after, row.next_after, "raw {raw:02x}");
        assert_eq!(trace.idim_after, row.idim_after, "raw {raw:02x}");
        assert_eq!(
            trace.in_interrupt_after, row.in_interrupt_after,
            "raw {raw:02x}"
        );
        assert_eq!(trace.break_after, row.break_after, "raw {raw:02x}");

        let mut expected_memory = initial;
        for write in &row.ram_writes {
            assert!(matches!(write.port.as_str(), "A" | "B"));
            expected_memory[write.address..write.address + 4]
                .copy_from_slice(&write.value.to_be_bytes());
        }
        assert_eq!(
            trace.state_after.internal_memory, expected_memory,
            "full internal state for raw {raw:02x}"
        );
        match (&row.bus_transaction, trace.bus_transaction) {
            (None, None) => {}
            (Some(expected), Some(actual)) => match (expected.kind.as_str(), actual) {
                ("read", BusTransaction::Read { address, value })
                | ("write", BusTransaction::Write { address, value }) => {
                    assert_eq!(address, expected.address);
                    assert_eq!(value, expected.value);
                }
                _ => panic!("wrong transaction kind for raw {raw:02x}"),
            },
            _ => panic!("transaction presence differs for raw {raw:02x}"),
        }
    }
}

#[test]
fn multistep_external_and_interrupt_scenarios_match_ghdl() {
    let corpus = parse();
    assert_eq!(corpus.scenarios.len(), 4);

    for scenario in &corpus.scenarios {
        match scenario.name.as_str() {
            "external-load" | "external-store" => {
                let raw = if scenario.name == "external-load" {
                    0x08
                } else {
                    0x0c
                };
                let mut cpu = ZpuSimulator::new();
                cpu.load(&initial_image(raw, (scenario.case - 255) as u8), 0)
                    .expect("scenario image");
                cpu.write_external_word(0x8000, EXTERNAL_VALUE)
                    .expect("external fixture");
                let trace = cpu.step().expect("external scenario step");
                assert_boundary(&trace, &scenario.boundaries[0]);
                assert_eq!(trace.pc_after, scenario.boundaries[1].pc);
                assert_eq!(trace.sp_after, scenario.boundaries[1].sp);
                assert_eq!(trace.top_after, scenario.boundaries[1].top);
                let event = scenario
                    .events
                    .iter()
                    .find(|event| event.kind.starts_with("ZPU_BUS"))
                    .expect("external bus event");
                let (address, value) = match trace.bus_transaction.expect("transaction") {
                    BusTransaction::Read { address, value }
                    | BusTransaction::Write { address, value } => (address, value),
                };
                assert_eq!(Some(address), event.address);
                assert_eq!(Some(value), event.value);
            }
            "continued-immediate" | "immediate-interrupt-deferral" => {
                let mut cpu = ZpuSimulator::new();
                cpu.load(&initial_image(0x81, (scenario.case - 255) as u8), 0)
                    .expect("scenario image");
                let first = cpu.step().expect("first IM");
                assert_boundary(&first, &scenario.boundaries[0]);
                if scenario.name == "immediate-interrupt-deferral" {
                    cpu.set_interrupt_level(true);
                }
                let second = cpu.step().expect("continued IM");
                assert_boundary(&second, &scenario.boundaries[1]);
                let third = cpu.step().expect("IDIM-clearing NOP");
                assert_boundary(&third, &scenario.boundaries[2]);
                if scenario.name == "immediate-interrupt-deferral" {
                    let interrupt = cpu.step().expect("deferred interrupt");
                    assert!(interrupt.raw.is_none());
                    assert_eq!(interrupt.mnemonic, "INTERRUPT");
                    assert_boundary(&interrupt, &scenario.boundaries[3]);
                    assert_eq!(interrupt.pc_after, scenario.boundaries[4].pc);
                    assert_eq!(interrupt.sp_after, scenario.boundaries[4].sp);
                    assert_eq!(interrupt.top_after, scenario.boundaries[4].top);
                }
            }
            other => panic!("unexpected scenario {other}"),
        }
    }
}

#[test]
fn fixture_reset_values_are_explicit() {
    let image = initial_image(0x0b, 0);
    assert_eq!(read_word(&image, RESET_SP), 0x100);
    assert_eq!(read_word(&image, RESET_SP + 4), 0x1122_3344);
}
