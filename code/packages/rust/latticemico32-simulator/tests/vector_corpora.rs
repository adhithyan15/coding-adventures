use latticemico32_simulator::{decode, opcode_class};

#[test]
fn manual_primary_opcode_corpus_is_complete_and_reproducible() {
    let corpus = include_str!("vectors/manual-opcodes.csv");
    let mut rows = 0;
    for line in corpus.lines().skip(1).filter(|line| !line.is_empty()) {
        let (opcode, expected) = line.split_once(',').unwrap();
        let opcode = u8::from_str_radix(opcode, 16).unwrap();
        assert_eq!(opcode_class(opcode), expected, "opcode {opcode:#04x}");
        rows += 1;
    }
    assert_eq!(rows, 64);
}

#[test]
fn pinned_rtl_decode_evidence_covers_every_implemented_family() {
    let corpus = include_str!("vectors/pinned-rtl-decode.csv");
    let mut rows = 0;
    let mut seen = [false; 64];
    for line in corpus.lines().skip(1).filter(|line| !line.is_empty()) {
        let (word, expected) = line.split_once(',').unwrap();
        let word = u32::from_str_radix(word, 16).unwrap();
        let decoded = decode(word).unwrap();
        assert_eq!(decoded.mnemonic, expected, "word {word:#010x}");
        seen[(word >> 26) as usize] = true;
        rows += 1;
    }
    assert_eq!(rows, 61);
    for (opcode, present) in seen.iter().enumerate() {
        if opcode_class(opcode as u8) == "implemented" {
            assert!(*present, "missing primary opcode {opcode:#04x}");
        }
    }
}
