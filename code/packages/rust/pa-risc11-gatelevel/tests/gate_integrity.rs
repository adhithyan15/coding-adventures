use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FILES: [(&str, &str); 5] = [
    ("lib.rs", include_str!("../src/lib.rs")),
    ("decoder.rs", include_str!("../src/decoder.rs")),
    ("engine.rs", include_str!("../src/engine.rs")),
    ("gates.rs", include_str!("../src/gates.rs")),
    ("state.rs", include_str!("../src/state.rs")),
];

fn normalized(source: &str) -> String {
    let syntax = syn::parse_file(source).expect("production source parses");
    format!("{syntax:#?}")
}

fn validate(sources: &BTreeMap<&str, String>) -> Result<(), String> {
    for (name, original) in FILES {
        let source = sources.get(name).ok_or_else(|| format!("missing {name}"))?;
        syn::parse_file(source).map_err(|error| format!("{name}: {error}"))?;
        if normalized(source) != normalized(original) {
            return Err(format!("transition-chain lock changed in {name}"));
        }
    }
    let lib = syn::parse_file(sources.get("lib.rs").expect("checked lib"))
        .map_err(|error| error.to_string())?;
    let modules: Vec<_> = lib
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Mod(module) if module.content.is_none() => Some((
                module.ident.to_string(),
                module.attrs.len(),
                module.vis.clone(),
            )),
            _ => None,
        })
        .collect();
    if modules.len() != 4
        || modules
            .iter()
            .map(|entry| entry.0.as_str())
            .collect::<Vec<_>>()
            != ["decoder", "engine", "gates", "state"]
        || modules
            .iter()
            .any(|entry| entry.1 != 0 || !matches!(entry.2, syn::Visibility::Inherited))
    {
        return Err("external module routing changed".into());
    }
    Ok(())
}

fn original_sources() -> BTreeMap<&'static str, String> {
    FILES
        .into_iter()
        .map(|(name, source)| (name, source.to_owned()))
        .collect()
}

fn replace_once(source: &mut String, old: &str, new: &str) {
    assert_eq!(source.matches(old).count(), 1, "hostile anchor: {old}");
    *source = source.replacen(old, new, 1);
}

fn compile_crate(sources: &BTreeMap<&str, String>) {
    let temp = tempfile::tempdir().unwrap();
    let src = temp.path().join("src");
    fs::create_dir(&src).unwrap();
    for (name, source) in sources {
        fs::write(src.join(name), source).unwrap();
    }
    let package = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rust = package.parent().unwrap();
    let manifest = format!(
        "[package]\nname='hostile-pa-risc-gate'\nversion='0.0.0'\nedition='2021'\n\
         [dependencies]\narithmetic={{path='{}'}}\nlogic-gates={{path='{}'}}\n\
         pa-risc11-simulator={{path='{}'}}\n",
        rust.join("arithmetic").display(),
        rust.join("logic-gates").display(),
        rust.join("pa-risc11-simulator").display(),
    );
    fs::write(temp.path().join("Cargo.toml"), manifest).unwrap();
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("target/pa-risc11-hostile");
    let result = Command::new(env!("CARGO"))
        .args(["check", "--quiet"])
        .env("CARGO_TARGET_DIR", target)
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "mutation must compile before rejection:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn production_chain_and_module_routes_are_exactly_locked() {
    let sources = original_sources();
    validate(&sources).unwrap();
    let joined = sources.values().cloned().collect::<String>();
    for forbidden in [
        "PaRisc11Simulator",
        "unsafe {",
        ".unwrap_unchecked(",
        "static mut",
    ] {
        assert!(
            !joined.contains(forbidden),
            "forbidden production shortcut {forbidden}"
        );
    }
}

#[test]
fn compile_valid_hostile_rewrites_are_rejected() {
    let mutations = [
        ("state.rs", "clock_bit(&mut self.halted, value.halted);", "self.halted = u8::from(value.halted);"),
        ("lib.rs", "mod gates;", "#[path = \"gates.rs\"] mod gates;"),
        ("lib.rs", "mod gates;", "#[path = \"gates.rs\"] mod gate_impl; mod gates { pub(crate) use super::gate_impl::*; }"),
        ("lib.rs", "mod gates;", "#[path = \"gates.rs\"] mod gate_impl; use gate_impl as gates;"),
        ("lib.rs", "mod gates;", "macro_rules! route_gates { () => { mod gates; } } route_gates!();"),
        ("lib.rs", "mod gates;", "macro_rules! discard_route { ($($tokens:tt)*) => {}; } discard_route!(mod fake_gates;); mod gates;"),
        ("lib.rs", "(engine::execute_word(&mut candidate, raw_word)?, false)", "((|state, word| engine::execute_word(state, word))(&mut candidate, raw_word)?, false)"),
        ("lib.rs", "(engine::execute_word(&mut candidate, raw_word)?, false)", "({ let mut oracle = pa_risc11_simulator::PaRisc11Simulator::new(); oracle.restore(&candidate)?; let trace = oracle.step()?; candidate = trace.state_after; trace.mnemonic }, false)"),
        ("lib.rs", "state_after,\n        };", "state_after: before.clone(),\n        };"),
        ("lib.rs", "                            final_state,\n", "                            final_state: self.observer.clone(),\n"),
        ("engine.rs", "state.gpr[register] = gates::mux32(state.gpr[register], value, enable);", "state.gpr[register] = if enable == 1 { value } else { state.gpr[register] };"),
        ("engine.rs", "gates::compare_condition(decoded.condition, left, right)", "u8::from(left < right)"),
        ("decoder.rs", "let primary = ((word >> 26) & 0x3f) as u8;", "let primary = (word / (1 << 26)) as u8;"),
        ("decoder.rs", "if lines[0] == 1 {", "if primary == 0 {"),
        ("gates.rs", "pub(crate) fn add32(", "#[allow(dead_code)] fn native_decoy(a: u32, b: u32) -> u32 { a.wrapping_add(b) }\n\npub(crate) fn add32("),
        ("gates.rs", "pub(crate) fn add32(", "#[allow(dead_code)] fn add32_duplicate(left: u32, right: u32) -> u32 { add32(left, right, 0).0 }\n\npub(crate) fn add32("),
        ("engine.rs", "state.next_pc = gates::add32(state.next_pc, 4, 0).0;", "state.next_pc = state.next_pc.wrapping_add(4);"),
        ("engine.rs", "let value = gates::shift_left(gates::assemble21(word & 0x1f_ffff), 11);", "let value = gates::assemble21(word & 0x1f_ffff) << 11;"),
        ("engine.rs", "gates::and32(gates::add32(read_gpr(state, decoded.right), index, 0).0, !3)", "gates::add32(read_gpr(state, decoded.right), index, 0).0 & !3"),
        ("engine.rs", "let enables = gates::register_enables(index, 1);", "let enables = [1; 32];"),
        ("engine.rs", "let decoded = decoder::decode(word, state.pc, state.in_delay_slot)?;", "let mut decoded = decoder::decode(word, state.pc, state.in_delay_slot)?; decoded.kind = Kind::Break;"),
        ("engine.rs", "let decoded = decoder::decode(word, state.pc, state.in_delay_slot)?;", "if false { let _ = gates::add32(word, 0, 0); } let decoded = decoder::decode(word, state.pc, state.in_delay_slot)?;"),
        ("engine.rs", "let value = gates::add32(read_gpr(state, decoded.right), displacement, 0).0;\n            write_gpr(state, decoded.left, value);", "let value = gates::add32(read_gpr(state, decoded.right), displacement, 0).0;\n            let value = 0;\n            write_gpr(state, decoded.left, value);"),
        ("lib.rs", "pub fn snapshot(&self) -> Result<PaRisc11State, PaRisc11Error> {\n        self.project(AllocationPoint::Snapshot)\n    }", "pub fn snapshot(&self) -> Result<PaRisc11State, PaRisc11Error> {\n        checked_clone(&self.observer)\n    }"),
        ("lib.rs", "self.physical.apply(&candidate);\n        self.observer = candidate;\n        Ok(trace)", "self.physical.apply(&candidate);\n        self.observer = before;\n        Ok(trace)"),
        ("lib.rs", "self.physical.apply(&candidate);\n        self.observer = candidate;\n        Ok(trace)", "self.physical.apply(&candidate);\n        candidate = before;\n        self.observer = candidate;\n        Ok(trace)"),
        ("decoder.rs", "pub(crate) fn decode(word: u32, pc: u32, in_delay_slot: bool) -> Result<Decoded, PaRisc11Error> {", "#[allow(dead_code)] fn second_decoder(word: u32) -> u32 { match word >> 26 { 0 => 0, _ => 1 } }\n\npub(crate) fn decode(word: u32, pc: u32, in_delay_slot: bool) -> Result<Decoded, PaRisc11Error> {"),
        ("decoder.rs", "pub(crate) fn decode(word: u32, pc: u32, in_delay_slot: bool) -> Result<Decoded, PaRisc11Error> {", "#[allow(dead_code)] fn inactive_decoy(word: u32) -> bool { if false { return word >> 26 == 0; } false }\n\npub(crate) fn decode(word: u32, pc: u32, in_delay_slot: bool) -> Result<Decoded, PaRisc11Error> {"),
        ("engine.rs", "use crate::gates;", "use crate::gates;\n#[allow(unused_imports)] use crate::gates as shadowed_gate_authority;"),
        ("engine.rs", "use crate::gates;", "use crate::gates;\n#[allow(dead_code)] fn dead_gate_anchor() -> u32 { gates::add32(1, 2, 0).0 }"),
        ("state.rs", "self.q[start + bit] = d_flip_flop(input, 1, &mut state).0;", "let _ = d_flip_flop(input, 1, &mut state); self.q[start + bit] = input;"),
    ];
    for (file, old, new) in mutations {
        let mut sources = original_sources();
        replace_once(sources.get_mut(file).unwrap(), old, new);
        compile_crate(&sources);
        assert!(
            validate(&sources).is_err(),
            "validator accepted hostile {file}: {old}"
        );
    }
}

#[test]
fn comments_strings_and_character_literals_do_not_confuse_parsed_lock() {
    let mut sources = original_sources();
    let lib = sources.get_mut("lib.rs").unwrap();
    lib.push_str("\n#[allow(dead_code)] fn confusers() { let _ = \"mod gates;\"; let _ = 'Q'; /* direct q */ }\n");
    compile_crate(&sources);
    assert!(validate(&sources).is_err());
}
