mod rtl_support;

use coding_adventures_sha256::sha256_hex;
use microwatt_powerisa_simulator::MicrowattPowerIsaSimulator;

#[test]
fn every_pinned_rtl_row_matches_the_functional_simulator() {
    for row in rtl_support::rows() {
        let mut cpu = MicrowattPowerIsaSimulator::new();
        let before = rtl_support::initial_state(&row);
        cpu.restore(&before).unwrap();
        let trace = cpu
            .step()
            .unwrap_or_else(|error| panic!("{}: {error}", row.name));
        assert_eq!(trace.raw, row.raw, "{} raw", row.name);
        let actual = cpu.get_state();
        let expected = rtl_support::expected_state(&row, actual.clone());
        assert_eq!(actual.gpr, expected.gpr, "{} GPRs", row.name);
        assert_eq!(actual.cia, expected.cia, "{} CIA", row.name);
        assert_eq!(actual.lr, expected.lr, "{} LR", row.name);
        assert_eq!(actual.ctr, expected.ctr, "{} CTR", row.name);
        assert_eq!(actual.cr, expected.cr, "{} CR", row.name);
        assert_eq!(actual.xer, expected.xer, "{} XER", row.name);
        assert_eq!(actual.halted, expected.halted, "{} halt", row.name);
        assert_eq!(
            sha256_hex(&actual.memory),
            row.memory_sha256,
            "{} memory",
            row.name
        );
    }
}
