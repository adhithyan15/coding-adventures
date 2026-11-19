//! Development CLI binding real TypeScript source bytes to bounded WIR/SAT evidence.
//!
//! This executable accepts the consumer's source file as an input. It never
//! embeds consumer bytes and never trusts a caller-supplied source label as
//! evidence of which bytes were parsed.

use bit_blaster::bv::{self, BitVec};
use coding_adventures_sha256::sha256_hex;
use coding_adventures_warrant_ir::sym::zero_extend;
use coding_adventures_warrant_ir::{Prover, SymMemory, SymVal, VerificationVerdict};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use typescript_warrant_ir::lower_typescript_bytes;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Query {
    Healthy,
    BitIsSetNotEqual,
    SetBitOr,
    ClearBitComplement,
}

impl Query {
    fn parse(value: Option<&str>) -> Result<Self, String> {
        let value = value.ok_or("--query name:expected-verdict is required")?;
        match value.split_once(':') {
            Some(("healthy", "Proved")) => Ok(Self::Healthy),
            Some(("bit_is_set_not_equal", "Counterexample")) => Ok(Self::BitIsSetNotEqual),
            Some(("set_bit_or", "Counterexample")) => Ok(Self::SetBitOr),
            Some(("clear_bit_complement", "Counterexample")) => Ok(Self::ClearBitComplement),
            Some((name, expected)) => Err(format!(
                "unexpected query/verdict pair {name:?}:{expected:?}"
            )),
            None => Err("--query must be name:expected-verdict".to_string()),
        }
    }

    fn helper(self) -> Option<&'static str> {
        match self {
            Self::Healthy => None,
            Self::BitIsSetNotEqual => Some("bit_is_set"),
            Self::SetBitOr => Some("set_bit"),
            Self::ClearBitComplement => Some("clear_bit"),
        }
    }

    fn guard(self) -> &'static str {
        match self {
            Self::Healthy => "true (unrestricted mask and slot domain)",
            Self::BitIsSetNotEqual | Self::SetBitOr => "selected prior bit is 0",
            Self::ClearBitComplement => "selected prior bit is 1",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::BitIsSetNotEqual => "bit_is_set_not_equal",
            Self::SetBitOr => "set_bit_or",
            Self::ClearBitComplement => "clear_bit_complement",
        }
    }

    fn expected_verdict(self) -> &'static str {
        if self == Query::Healthy {
            "Proved"
        } else {
            "Counterexample"
        }
    }
}

fn validate_provenance_flags(
    query: Query,
    query_name: Option<&str>,
    origin_path: Option<&str>,
    origin_sha: Option<&str>,
    mutation: Option<&str>,
) -> Result<(), String> {
    if query == Query::Healthy {
        if origin_path.is_some() || origin_sha.is_some() || mutation.is_some() {
            return Err("healthy query does not accept origin or mutation flags".to_string());
        }
        return Ok(());
    }
    if origin_path.is_none() || origin_sha.is_none() {
        return Err("mutant query requires --origin-path and --origin-sha256".to_string());
    }
    let mutation = mutation.ok_or("mutant query requires --mutation")?;
    let query_name = query_name
        .and_then(|name| name.split_once(':').map(|pair| pair.0))
        .ok_or("mutant query requires explicit query name and expected verdict")?;
    if mutation != query_name || mutation != query.name() {
        return Err("mutation descriptor must exactly match the selected query name".to_string());
    }
    Ok(())
}

fn validate_source_binding(
    actual_path: &Path,
    expected_path: &str,
    root: &Path,
    bytes: &[u8],
    expected_sha: &str,
) -> Result<(Option<String>, String, String), String> {
    let relative_path = actual_path
        .strip_prefix(root)
        .ok()
        .map(|path| path.to_string_lossy().replace('\\', "/"));
    let expected_matches = if Path::new(expected_path).is_absolute() {
        Path::new(expected_path)
            .canonicalize()
            .is_ok_and(|expected| expected == actual_path)
    } else {
        relative_path.as_deref() == Some(expected_path)
    };
    if !expected_matches {
        return Err(format!(
            "source path mismatch: actual {:?}, expected {expected_path:?}",
            actual_path.display()
        ));
    }
    let digest = sha256_hex(bytes);
    if digest != expected_sha {
        return Err(format!(
            "source SHA-256 mismatch: actual {digest}, expected {expected_sha}"
        ));
    }
    let identity = relative_path
        .clone()
        .unwrap_or_else(|| actual_path.to_string_lossy().replace('\\', "/"));
    Ok((relative_path, identity, digest))
}

fn validate_mutant_bytes(query: Query, original: &str, mutated: &str) -> Result<(), String> {
    let (before, after) = match query {
        Query::BitIsSetNotEqual => ("!== 0n", "=== 0n"),
        Query::SetBitOr => ("return mask |", "return mask &"),
        Query::ClearBitComplement => (
            "mask & ~(1n << BigInt(slot))",
            "mask & (1n << BigInt(slot))",
        ),
        Query::Healthy => return Err("healthy query cannot carry mutant provenance".to_string()),
    };
    let expected_mutant = original.replacen(before, after, 1);
    if expected_mutant == original || expected_mutant != mutated {
        return Err(format!(
            "actual source bytes are not the single admitted {query:?} mutation"
        ));
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{{\"status\":\"refused\",\"error\":{}}}", json!(error));
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut source_path = None;
    let mut expected_path = None;
    let mut expected_sha = None;
    let mut query_name = None;
    let mut origin_path = None;
    let mut origin_sha = None;
    let mut mutation = None;
    let mut origin_record = Value::Null;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag.as_str() {
            "--source" => source_path = Some(PathBuf::from(value)),
            "--expected-path" => expected_path = Some(value),
            "--sha256" => expected_sha = Some(value),
            "--query" => query_name = Some(value),
            "--origin-path" => origin_path = Some(value),
            "--origin-sha256" => origin_sha = Some(value),
            "--mutation" => mutation = Some(value),
            _ => return Err(format!("unknown argument {flag:?}")),
        }
    }
    let source_path = source_path.ok_or("--source is required")?;
    let expected_path = expected_path.ok_or("--expected-path is required")?;
    let expected_sha = expected_sha.ok_or("--sha256 is required")?;
    let query = Query::parse(query_name.as_deref())?;
    validate_provenance_flags(
        query,
        query_name.as_deref(),
        origin_path.as_deref(),
        origin_sha.as_deref(),
        mutation.as_deref(),
    )?;
    let actual_path = source_path
        .canonicalize()
        .map_err(|e| format!("source path: {e}"))?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .ok_or("could not locate repository root")?
        .canonicalize()
        .map_err(|e| format!("repository root: {e}"))?;
    let bytes = std::fs::read(&actual_path).map_err(|e| format!("read source: {e}"))?;
    let (relative_path, displayed_path, digest) =
        validate_source_binding(&actual_path, &expected_path, &root, &bytes, &expected_sha)?;
    if query != Query::Healthy {
        let origin_path = origin_path
            .as_deref()
            .ok_or("mutant query requires --origin-path")?;
        let origin_sha = origin_sha
            .as_deref()
            .ok_or("mutant query requires --origin-sha256")?;
        let origin = Path::new(origin_path)
            .canonicalize()
            .map_err(|e| format!("origin path: {e}"))?;
        let origin_bytes = std::fs::read(&origin).map_err(|e| format!("read origin: {e}"))?;
        if sha256_hex(&origin_bytes) != origin_sha {
            return Err("origin source SHA-256 mismatch".to_string());
        }
        let original = std::str::from_utf8(&origin_bytes).map_err(|_| "origin is not UTF-8")?;
        let mutated = std::str::from_utf8(&bytes).map_err(|_| "mutant is not UTF-8")?;
        validate_mutant_bytes(query, original, mutated)?;
        origin_record = json!({
            "path": origin.strip_prefix(&root).ok().map(|path| path.to_string_lossy().replace('\\', "/")),
            "physical_path": origin,
            "sha256": origin_sha
        });
    }

    // Parse and lower only after path, exact bytes, digest, and provenance checks.
    let program =
        lower_typescript_bytes(&bytes).map_err(|e| format!("parse/lower/typecheck: {e}"))?;
    let helper_names = ["bit_is_set", "set_bit", "clear_bit"];
    for helper in helper_names {
        if program.find_function(helper).is_none() {
            return Err(format!("missing lowered helper {helper}"));
        }
    }
    let targets: Vec<&str> = query
        .helper()
        .map_or_else(|| helper_names.to_vec(), |name| vec![name]);
    let mut records = Vec::new();
    for helper in targets {
        let function = program
            .find_function(helper)
            .ok_or_else(|| format!("missing helper {helper}"))?;
        let mut prover = Prover::new();
        let mask = prover.executor.create_input_bits("mask", 64);
        let slot6 = prover.executor.create_input_bits("slot", 6);
        let slot8 = zero_extend(&slot6, 8);
        let guard = if query == Query::Healthy {
            bit_blaster::aig::Edge::TRUE
        } else {
            // Slot-specific mux below is assembled as a 64-way selection, so
            // the call-site predicate uses the same unrestricted symbolic slot.
            let mut selected_bit = bit_blaster::aig::Edge::FALSE;
            for (bit, held) in mask.bits.iter().copied().enumerate() {
                let at_slot = bv::eq(
                    &mut prover.executor.aig,
                    &slot6,
                    &BitVec::constant(6, bit as u128),
                );
                let selected_level = prover.executor.aig.and(at_slot, held);
                selected_bit = prover.executor.aig.or(selected_bit, selected_level);
            }
            if query == Query::ClearBitComplement {
                selected_bit
            } else {
                !selected_bit
            }
        };
        let spec_helper = match query {
            Query::Healthy => helper,
            Query::BitIsSetNotEqual => "bit_is_set",
            Query::SetBitOr => "set_bit",
            Query::ClearBitComplement => "clear_bit",
        };
        let verdict = prover.prove_equivalence_with_guard(
            function,
            vec![SymVal::Bits(mask.clone()), SymVal::Bits(slot8)],
            SymMemory::new(),
            guard,
            |aig, args, _| {
                let input_mask = args[0].to_bits()?;
                let slot = args[1].to_bits()?;
                let mut outputs = Vec::with_capacity(64);
                for (bit, held) in input_mask.bits.iter().copied().enumerate() {
                    let at_slot = bv::eq(aig, &slot, &BitVec::constant(8, bit as u128));
                    outputs.push(match spec_helper {
                        "bit_is_set" => aig.and(at_slot, held),
                        "set_bit" => aig.or(at_slot, held),
                        "clear_bit" => aig.and(!at_slot, held),
                        _ => unreachable!(),
                    });
                }
                Ok(if spec_helper == "bit_is_set" {
                    SymVal::Bool(aig.or_all(&outputs))
                } else {
                    SymVal::Bits(BitVec { bits: outputs })
                })
            },
        );
        let (verdict_name, model) = match verdict {
            VerificationVerdict::Proved => ("UNSAT", None),
            VerificationVerdict::Falsified { counterexample } => {
                let mut values = std::collections::BTreeMap::new();
                for (name, value) in counterexample {
                    values.insert(name, value);
                }
                let mask_value = *values.get("mask").ok_or("SAT model omitted mask")?;
                let slot_value = *values.get("slot").ok_or("SAT model omitted slot")?;
                let bit = (mask_value >> slot_value) & 1;
                let expected = match spec_helper {
                    "bit_is_set" => json!(false),
                    "set_bit" => json!((mask_value | (1u128 << slot_value)).to_string()),
                    "clear_bit" => json!((mask_value & !(1u128 << slot_value)).to_string()),
                    _ => unreachable!(),
                };
                let predicted = match query {
                    Query::BitIsSetNotEqual => json!(true),
                    Query::SetBitOr => json!((mask_value & (1u128 << slot_value)).to_string()),
                    Query::ClearBitComplement => {
                        json!((mask_value & (1u128 << slot_value)).to_string())
                    }
                    Query::Healthy => {
                        return Err("healthy query unexpectedly found a mismatch".to_string())
                    }
                };
                if (query == Query::BitIsSetNotEqual && bit != 0)
                    || (query == Query::SetBitOr && bit != 0)
                    || (query == Query::ClearBitComplement && bit != 1)
                {
                    return Err("SAT witness violates the call-site guard".to_string());
                }
                if query != Query::Healthy && expected == predicted {
                    return Err("SAT witness does not distinguish source from oracle".to_string());
                }
                (
                    "SAT",
                    Some(json!({
                        "mask_decimal": mask_value.to_string(),
                        "slot_decimal": slot_value.to_string(),
                        "selected_prior_bit": bit,
                        "expected": expected,
                        "predicted": predicted,
                        "guard_evaluated": true
                    })),
                )
            }
            VerificationVerdict::Unknown(reason) => {
                return Err(format!("solver UNKNOWN for {helper}: {reason}"))
            }
            VerificationVerdict::Bounded(bound) => {
                return Err(format!("unexpected bounded verdict {bound}"))
            }
        };
        if (query == Query::Healthy && verdict_name != "UNSAT")
            || (query != Query::Healthy && verdict_name != "SAT")
        {
            return Err(format!("unexpected verdict {verdict_name} for {:?}", query));
        }
        records.push(json!({
            "helper": helper,
            "kind": format!("{query:?}"),
            "expected_verdict": query.expected_verdict(),
            "guard": query.guard(),
            "verdict": verdict_name,
            "model": model
        }));
    }
    let provenance = if query == Query::Healthy {
        Value::Null
    } else {
        json!({"origin": origin_record, "mutation": mutation.unwrap()})
    };
    println!(
        "{}",
        json!({
            "schema": "typescript-warrant-ir/source-proof/v1",
            "status": "proved",
            "source": {"path": relative_path, "identity": displayed_path, "physical_path": actual_path, "bytes": bytes.len(), "sha256": digest},
            "provenance": provenance,
            "profile": {"parser": "ts5.8", "bridge_version": env!("CARGO_PKG_VERSION"), "lowerer": "typescript-warrant-ir"},
            "solver": "coding_adventures_warrant_ir::Prover using cdcl-sat; UNSAT results are DRAT-checked",
            "helpers": helper_names,
            "wir_types": {
                "parameters": {"mask": "U64", "slot": "U8"},
                "returns": {"bit_is_set": "Bool", "set_bit": "U64", "clear_bit": "U64"},
                "bool_width": 1,
                "integer_width": 64
            },
            "inputs": {"mask_bits": 64, "slot_bits": 6, "slot_domain": [0, 63]},
            "queries": records
        })
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    fn temp_root() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "typescript-warrant-ir-cli-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).expect("create unique test directory");
        path
    }

    #[test]
    fn query_and_expected_verdict_pairs_are_closed() {
        assert_eq!(Query::parse(Some("healthy:Proved")), Ok(Query::Healthy));
        assert!(Query::parse(None).is_err());
        assert_eq!(
            Query::parse(Some("set_bit_or:Counterexample")),
            Ok(Query::SetBitOr)
        );
        assert!(Query::parse(Some("set_bit_or:Proved")).is_err());
        assert!(Query::parse(Some("unknown:Counterexample")).is_err());
        assert!(Query::parse(Some("set_bit_or")).is_err());
    }

    #[test]
    fn healthy_and_mutant_provenance_flags_are_enforced() {
        assert!(validate_provenance_flags(
            Query::Healthy,
            Some("healthy:Proved"),
            None,
            None,
            None
        )
        .is_ok());
        assert!(validate_provenance_flags(
            Query::Healthy,
            Some("healthy:Proved"),
            Some("source.ts"),
            Some("hash"),
            None
        )
        .is_err());
        assert!(validate_provenance_flags(
            Query::Healthy,
            Some("healthy:Proved"),
            None,
            None,
            Some("mutation")
        )
        .is_err());
        assert!(validate_provenance_flags(
            Query::SetBitOr,
            Some("set_bit_or:Counterexample"),
            Some("source.ts"),
            Some("hash"),
            Some("set_bit_or")
        )
        .is_ok());
        assert!(validate_provenance_flags(
            Query::SetBitOr,
            Some("set_bit_or:Counterexample"),
            Some("source.ts"),
            Some("hash"),
            Some("other")
        )
        .is_err());
        assert!(validate_provenance_flags(
            Query::SetBitOr,
            Some("set_bit_or:Counterexample"),
            Some("source.ts"),
            None,
            Some("set_bit_or")
        )
        .is_err());
        assert!(validate_provenance_flags(
            Query::SetBitOr,
            Some("set_bit_or:Counterexample"),
            Some("source.ts"),
            Some("hash"),
            None
        )
        .is_err());
    }

    #[test]
    fn source_path_and_digest_binding_refuse_mismatches() {
        let root = temp_root();
        let source = root.join("digital.ts");
        let bytes = b"export const exact = true;";
        std::fs::write(&source, bytes).expect("write test source");
        let actual = source.canonicalize().expect("canonical source path");
        let canonical_root = root.canonicalize().expect("canonical temp root");
        let expected_sha = sha256_hex(bytes);
        assert!(validate_source_binding(
            &actual,
            "digital.ts",
            &canonical_root,
            bytes,
            &expected_sha
        )
        .is_ok());
        assert!(validate_source_binding(
            &actual,
            "wrong.ts",
            &canonical_root,
            bytes,
            &expected_sha
        )
        .is_err());
        assert!(validate_source_binding(
            &actual,
            "digital.ts",
            &canonical_root,
            bytes,
            "wrong-hash"
        )
        .is_err());
        assert!(validate_source_binding(
            &actual,
            actual.to_str().unwrap(),
            &canonical_root,
            bytes,
            &expected_sha
        )
        .is_ok());
        std::fs::remove_dir_all(root).expect("remove only this test's directory");
    }

    #[test]
    fn mutant_bytes_must_match_one_admitted_source_change() {
        let source = "export function set_bit(mask: bigint, slot: number): bigint { return mask | (1n << BigInt(slot)); }";
        let mutant = source.replacen("return mask |", "return mask &", 1);
        assert!(validate_mutant_bytes(Query::SetBitOr, source, &mutant).is_ok());
        assert!(validate_mutant_bytes(Query::SetBitOr, source, source).is_err());
        assert!(
            validate_mutant_bytes(Query::SetBitOr, source, &format!("{mutant} extra")).is_err()
        );
        assert!(validate_mutant_bytes(Query::Healthy, source, source).is_err());
    }
}
