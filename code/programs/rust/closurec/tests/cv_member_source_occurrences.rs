//! CLOC35 begins with exact result-owned member ancestry and raw source spans.
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
const BINARY: &str = env!("CARGO_BIN_EXE_closurec");
const SOURCE: &str = "closurec_source_v1";
const NODE: &str = "closurec_node_v1";

fn compile(sources: &[&str], level: &str) -> (Value, String, Vec<String>) {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "closurec-c35-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&dir).unwrap();
    let paths: Vec<_> = sources
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let p = dir.join(format!("input-{i}.js"));
            std::fs::write(&p, s).unwrap();
            p
        })
        .collect();
    let mut outputs = Vec::new();
    let mut trace = None;
    for traced in [true, false] {
        let output = dir.join(if traced { "traced.js" } else { "plain.js" });
        let mut cmd = Command::new(BINARY);
        cmd.args(["--compilation_level", level, "--js_output_file"])
            .arg(&output);
        for p in &paths {
            cmd.arg("--js").arg(p);
        }
        if traced {
            cmd.arg("--correlation_vector");
        }
        let result = cmd.output().unwrap();
        assert!(
            result.status.success(),
            "{level}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        outputs.push(std::fs::read_to_string(&output).unwrap());
        if traced {
            trace = Some(
                serde_json::from_str(
                    &std::fs::read_to_string(format!("{}.cv.json", output.display())).unwrap(),
                )
                .unwrap(),
            );
        }
    }
    assert_eq!(
        outputs[0], outputs[1],
        "member tracing changed emitted bytes"
    );
    let names = paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    std::fs::remove_dir_all(&dir).unwrap();
    (trace.unwrap(), outputs.remove(0), names)
}
fn position(source: &str, byte: usize) -> Value {
    let (mut scalar, mut utf16, mut line, mut column, mut utf16_column) = (0, 0, 0, 0, 0);
    let mut previous_cr = false;
    for ch in source[..byte].chars() {
        scalar += 1;
        utf16 += ch.len_utf16();
        if ch == '\n' && previous_cr {
        } else if matches!(ch, '\r' | '\n' | '\u{2028}' | '\u{2029}') {
            line += 1;
            column = 0;
            utf16_column = 0;
        } else {
            column += 1;
            utf16_column += ch.len_utf16();
        }
        previous_cr = ch == '\r';
    }
    json!({"byte_offset":byte.to_string(),"scalar_offset":scalar.to_string(),"utf16_offset":utf16.to_string(),"line":line.to_string(),"scalar_column":column.to_string(),"utf16_column":utf16_column.to_string()})
}
fn ancestry(j: &Value, id: &str) -> BTreeSet<String> {
    fn visit(j: &Value, id: &str, out: &mut BTreeSet<String>) {
        if !out.insert(id.to_owned()) {
            return;
        }
        for parent in j["entries"][id]["parent_ids"].as_array().unwrap() {
            visit(j, parent.as_str().unwrap(), out);
        }
    }
    let mut out = BTreeSet::new();
    visit(j, id, &mut out);
    out
}
fn number(v: &Value) -> usize {
    v.as_str().unwrap().parse().unwrap()
}
fn clock(v: &Value) -> u64 {
    u64::from_str_radix(v.as_str().unwrap(), 16).unwrap()
}

fn member(
    j: &Value,
    source: &str,
    path: &str,
    start: usize,
    end: usize,
    kind: &str,
    after: Option<&str>,
) -> String {
    let entries = j["entries"].as_object().unwrap();
    let roots: Vec<_> = entries
        .iter()
        .filter(|(_, e)| e["origin"]["source"] == "input_file" && e["origin"]["location"] == path)
        .collect();
    assert_eq!(roots.len(), 1);
    let root = roots[0].0;
    let expected_start = position(source, start);
    let expected_end = position(source, end);
    let found: Vec<_> = entries
        .iter()
        .filter(|(_, e)| {
            let m = &e["origin"]["meta"][NODE];
            e["origin"]["source"] == "typed_member"
                && m["kind"] == kind
                && m["input_id"] == root.as_str()
                && m["start"] == expected_start
                && m["end"] == expected_end
        })
        .collect();
    assert_eq!(
        found.len(),
        1,
        "missing exact {kind} occurrence {}",
        &source[start..end]
    );
    let (id, entry) = found[0];
    let m = &entry["origin"]["meta"][NODE];
    assert_eq!(entry["origin"]["location"], "member_prefix");
    assert!(entry["origin"]["timestamp"].is_null());
    assert_eq!(entry["origin"]["meta"].as_object().unwrap().len(), 1);
    assert_eq!(
        m.as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "schema",
            "kind",
            "input_id",
            "first_piece_id",
            "last_piece_id",
            "start",
            "end"
        ])
    );
    assert_eq!(m["schema"], "1");
    let inventories: Vec<_> = roots[0].1["contributions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["source"] == "source.inventory" && c["tag"] == "finalized")
        .collect();
    assert_eq!(inventories.len(), 1);
    let pieces = inventories[0]["meta"][SOURCE]["piece_ids"]
        .as_array()
        .unwrap();
    let first = pieces
        .iter()
        .position(|p| p == &m["first_piece_id"])
        .unwrap();
    let last = pieces
        .iter()
        .position(|p| p == &m["last_piece_id"])
        .unwrap();
    assert!(first < last);
    assert_eq!(
        entry["parent_ids"],
        Value::Array(pieces[first..=last].to_vec()),
        "exact original ordered physical interval"
    );
    assert!(
        !pieces.iter().any(|p| p == id.as_str()),
        "fresh checked member ID"
    );
    let mut cursor = start;
    for p in &pieces[first..=last] {
        let piece = &entries[p.as_str().unwrap()];
        let physical = &piece["origin"]["meta"][SOURCE];
        assert_eq!(piece["parent_ids"], json!([root]));
        assert_eq!(physical["input_id"], root.as_str());
        let finish = number(&physical["end"]["byte_offset"]);
        assert!(finish > cursor);
        assert_eq!(physical["start"], position(source, cursor));
        assert_eq!(physical["end"], position(source, finish));
        cursor = finish;
    }
    assert_eq!(cursor, end);
    for key in ["first_piece_id", "last_piece_id"] {
        assert_eq!(
            entries[m[key].as_str().unwrap()]["origin"]["meta"][SOURCE]["kind"],
            "token"
        );
    }
    let events = j["journal"]["events"].as_array().unwrap();
    let merges: Vec<_> = events
        .iter()
        .filter(|e| e["event"]["kind"] == "merge" && e["event"]["entity"] == id.as_str())
        .collect();
    assert_eq!(merges.len(), 1);
    assert!(merges[0]["context"].is_null());
    let sequence = clock(&merges[0]["sequence"]);
    let pipeline = events
        .iter()
        .find(|e| {
            clock(&e["sequence"]) > sequence
                && e["event"]["kind"] == "context_begin"
                && e["event"]["scope"]["kind"] == "pipeline"
        })
        .unwrap();
    let first_pass = events
        .iter()
        .find(|e| {
            e["event"]["kind"] == "context_begin"
                && e["event"]["scope"]["kind"] == "pass"
                && e["context"] == pipeline["sequence"]
        })
        .unwrap();
    assert!(
        sequence < clock(&pipeline["sequence"])
            && clock(&pipeline["sequence"]) < clock(&first_pass["sequence"]),
        "member Merge before its own input pipeline/pass"
    );
    if let Some(after) = after {
        let results: Vec<_> = entries
            .iter()
            .filter(|(result_id, e)| {
                ancestry(j, result_id).contains(id)
                    && e["contributions"].as_array().unwrap().iter().any(|c| {
                        c["source"] == "constant-fold"
                            && c["tag"] == "folded"
                            && c["meta"]["after"] == after
                            && c["meta"]["new_cv"] == result_id.as_str()
                    })
            })
            .collect();
        assert_eq!(
            results.len(),
            1,
            "unique result-owned fold for {}",
            &source[start..end]
        );
        let (result_id, _) = results[0];
        let reachable = ancestry(j, result_id);
        let reached_physical: BTreeSet<_> = pieces
            .iter()
            .filter(|p| reachable.contains(p.as_str().unwrap()))
            .map(|p| p.as_str().unwrap())
            .collect();
        let expected: BTreeSet<_> = pieces[first..=last]
            .iter()
            .map(|p| p.as_str().unwrap())
            .collect();
        assert_eq!(
            reached_physical, expected,
            "exact complete member operand interval, no physical siblings"
        );
        let birth = events
            .iter()
            .find(|e| {
                matches!(e["event"]["kind"].as_str(), Some("merge" | "derive"))
                    && e["event"]["entity"] == result_id.as_str()
            })
            .unwrap();
        assert!(sequence < clock(&birth["sequence"]));
    }
    id.clone()
}
fn exact(
    j: &Value,
    source: &str,
    path: &str,
    text: &str,
    kind: &str,
    after: Option<&str>,
) -> String {
    let start = source.find(text).unwrap();
    member(j, source, path, start, start + text.len(), kind, after)
}
#[test]
fn result_owned_primitive_and_array_folds_reach_exact_member_occurrences() {
    let source = "report([].length);report([,,][\"length\"]);report(\"😀\".length);report([\"x\"][0]);report([,][0]);\n";
    for level in ["SIMPLE", "ADVANCED"] {
        let (j, _, paths) = compile(&[source], level);
        for (text, after) in [
            ("[].length", "0"),
            ("[,,][\"length\"]", "2"),
            ("\"😀\".length", "2"),
            ("[,][0]", "void 0"),
        ] {
            exact(&j, source, &paths[0], text, "MemberExpression", Some(after));
        }
        // The original in-bounds member is certified here; its known unused
        // fork/selected-clone replacement ownership is a separate open repair.
        exact(
            &j,
            source,
            &paths[0],
            "[\"x\"][0]",
            "MemberExpression",
            None,
        );
    }
}
#[test]
fn every_flat_prefix_preserves_exact_original_group_call_new_optional_and_super_slices() {
    let source = "a.b.c;a[b][c];(a).b.c;f().x[y];new F().x;a?.b.c;a?.[[]];class C extends B{m(){super.x;super[y];}}";
    for level in ["SIMPLE", "ADVANCED"] {
        let (j, _, paths) = compile(&[source], level);
        let mut ids = BTreeSet::new();
        for text in [
            "a.b",
            "a.b.c",
            "a[b]",
            "a[b][c]",
            "(a).b",
            "(a).b.c",
            "f().x",
            "f().x[y]",
            "new F().x",
            "a?.b.c",
            "super.x",
            "super[y]",
        ] {
            assert!(ids.insert(exact(&j, source, &paths[0], text, "MemberExpression", None)));
        }
        for text in ["a?.b", "a?.[[]]"] {
            assert!(ids.insert(exact(
                &j,
                source,
                &paths[0],
                text,
                "OptionalMemberExpression",
                None
            )));
        }
        let inner = exact(&j, source, &paths[0], "a.b", "MemberExpression", None);
        let outer = exact(&j, source, &paths[0], "a.b.c", "MemberExpression", None);
        assert_ne!(inner, outer);
        assert!(j["entries"][outer]["parent_ids"]
            .as_array()
            .unwrap()
            .starts_with(j["entries"][inner]["parent_ids"].as_array().unwrap()));
    }
}

#[test]
fn c36_selected_operand_merge_and_own_fold_events_share_exact_pass_context() {
    for level in ["SIMPLE", "ADVANCED"] {
        for (source, key_after) in [
            ("report([1+2,9][0+0]);", "0"),
            ("report([1+2,9][\"\"+\"0\"]);", "\"0\""),
        ] {
            let (j, _, paths) = compile(&[source], level);
            let text = &source[7..source.len() - 2];
            let member = exact(&j, source, &paths[0], text, "MemberExpression", None);
            let entries = j["entries"].as_object().unwrap();
            let find_fold = |after: &str| {
                let rows: Vec<_> = entries
                    .iter()
                    .filter(|(id, e)| {
                        e["contributions"].as_array().unwrap().iter().any(|c| {
                            c["source"] == "constant-fold"
                                && c["tag"] == "folded"
                                && c["meta"]["after"] == after
                                && c["meta"]["new_cv"] == id.as_str()
                        })
                    })
                    .collect();
                assert_eq!(rows.len(), 1, "one exact own fold to {after}");
                rows[0].0.clone()
            };
            let selected = find_fold("3");
            let key = find_fold(key_after);
            let result = find_fold("element[0]");
            let arrays: Vec<_> = entries
                .iter()
                .filter(|(_, e)| {
                    e["origin"]["source"] == "typed_array"
                        && e["origin"]["meta"][NODE]["start"] == position(source, 7)
                        && e["origin"]["meta"][NODE]["end"] == position(source, 14)
                })
                .collect();
            assert_eq!(arrays.len(), 1);
            let merge = entries[&result]["parent_ids"].as_array().unwrap();
            assert_eq!(merge.len(), 1);
            let merge_id = merge[0].as_str().unwrap();
            assert_eq!(
                entries[merge_id]["parent_ids"],
                json!([member, arrays[0].0, key, selected])
            );
            let events = j["journal"]["events"].as_array().unwrap();
            let event = |kind: &str, id: &str| {
                let rows: Vec<_> = events
                    .iter()
                    .filter(|e| e["event"]["kind"] == kind && e["event"]["entity"] == id)
                    .collect();
                assert_eq!(rows.len(), 1);
                rows[0]
            };
            let merged = event("merge", merge_id);
            let derived = event("derive", &result);
            let contributed = event("contribution", &result);
            assert_eq!(derived["context"], merged["context"]);
            assert_eq!(contributed["context"], merged["context"]);
            assert!(clock(&merged["sequence"]) < clock(&derived["sequence"]));
            assert!(clock(&derived["sequence"]) < clock(&contributed["sequence"]));
            for child in [&key, &selected] {
                let rewritten = event("contribution", child);
                assert_eq!(rewritten["context"], merged["context"]);
                assert!(clock(&rewritten["sequence"]) < clock(&merged["sequence"]));
            }
            let pass = events
                .iter()
                .find(|e| e["sequence"] == merged["context"])
                .unwrap();
            assert_eq!(pass["event"]["kind"], "context_begin");
            assert_eq!(pass["event"]["scope"]["kind"], "pass");
            assert_eq!(clock(&pass["event"]["scope"]["sweep"]), 0);
            let schedule = events
                .iter()
                .find(|e| e["event"]["kind"] == "schedule" && e["context"] == pass["context"])
                .unwrap();
            let slot = clock(&pass["event"]["scope"]["slot"]) as usize;
            assert_eq!(schedule["event"]["passes"][slot]["name"], "constant-fold");
            let ended = events
                .iter()
                .find(|e| {
                    e["event"]["kind"] == "context_end" && e["event"]["begin"] == pass["sequence"]
                })
                .unwrap();
            assert_eq!(ended["event"]["outcome"]["kind"], "accepted");
            assert_eq!(ended["event"]["outcome"]["changed"], true);
            assert!(clock(&contributed["sequence"]) < clock(&ended["sequence"]));
        }
    }
}
#[test]
fn repeated_inputs_keep_raw_escaped_crlf_astral_asi_member_parents_separate() {
    let source = "/*😀*/\r\nreport([/*x*/\"\\u0061\",,[]].length)\na.b.c\n";
    for level in ["SIMPLE", "ADVANCED"] {
        let (j, _, paths) = compile(&[source, source], level);
        let mut ids = Vec::new();
        for path in &paths {
            ids.push(exact(
                &j,
                source,
                path,
                "[/*x*/\"\\u0061\",,[]].length",
                "MemberExpression",
                Some("3"),
            ));
            exact(&j, source, path, "a.b.c", "MemberExpression", None);
        }
        assert_ne!(ids[0], ids[1]);
        assert!(ancestry(&j, &ids[0]).is_disjoint(&ancestry(&j, &ids[1])));
    }
}

#[test]
fn c37_selected_call_joins_actual_callee_and_argument_rewrite_before_outer_event() {
    for level in ["SIMPLE", "ADVANCED"] {
        for source in [
            "report([observe(1+2),9][0+0]);",
            "report([observe(1+2),9][\"\"+\"0\"]);",
        ] {
            let (j, _, paths) = compile(&[source], level);
            let member = exact(
                &j,
                source,
                &paths[0],
                &source[7..source.len() - 2],
                "MemberExpression",
                None,
            );
            let entries = j["entries"].as_object().unwrap();
            let own_fold = |after: &str| {
                let rows: Vec<_> = entries
                    .iter()
                    .filter(|(id, entry)| {
                        entry["contributions"].as_array().unwrap().iter().any(|c| {
                            c["source"] == "constant-fold"
                                && c["tag"] == "folded"
                                && c["meta"]["after"] == after
                                && c["meta"]["new_cv"] == id.as_str()
                        })
                    })
                    .collect();
                assert_eq!(rows.len(), 1);
                rows[0].0.clone()
            };
            let result = own_fold("element[0]");
            let child = own_fold("3");
            let key = own_fold(if source.contains("\"\"") {
                "\"0\""
            } else {
                "0"
            });
            let array_end = source.find("][").unwrap() + 1;
            let arrays: Vec<_> = entries
                .iter()
                .filter(|(_, entry)| {
                    entry["origin"]["source"] == "typed_array"
                        && entry["origin"]["meta"][NODE]["start"] == position(source, 7)
                        && entry["origin"]["meta"][NODE]["end"] == position(source, array_end)
                })
                .collect();
            assert_eq!(arrays.len(), 1);
            let callee_start = source.find("observe").unwrap();
            let callees: Vec<_> = entries
                .iter()
                .filter(|(_, entry)| {
                    let physical = &entry["origin"]["meta"][SOURCE];
                    physical["kind"] == "token"
                        && physical["start"] == position(source, callee_start)
                        && physical["end"] == position(source, callee_start + "observe".len())
                })
                .collect();
            assert_eq!(callees.len(), 1);
            let parents = entries[&result]["parent_ids"].as_array().unwrap();
            assert_eq!(parents.len(), 1);
            let merged_id = parents[0].as_str().unwrap();
            assert_eq!(
                entries[merged_id]["parent_ids"],
                json!([member, arrays[0].0, key, callees[0].0, child])
            );
            assert!(
                ancestry(&j, &result).contains(&child),
                "selected root reaches newer argument identity"
            );
            let events = j["journal"]["events"].as_array().unwrap();
            let event = |kind: &str, id: &str| {
                let rows: Vec<_> = events
                    .iter()
                    .filter(|e| e["event"]["kind"] == kind && e["event"]["entity"] == id)
                    .collect();
                assert_eq!(rows.len(), 1);
                rows[0]
            };
            let merge = event("merge", merged_id);
            let derive = event("derive", &result);
            let contribution = event("contribution", &result);
            assert_eq!(merge["context"], derive["context"]);
            assert_eq!(merge["context"], contribution["context"]);
            assert!(clock(&merge["sequence"]) < clock(&derive["sequence"]));
            assert!(clock(&derive["sequence"]) < clock(&contribution["sequence"]));
            for rewritten in [&key, &child] {
                let prior = event("contribution", rewritten);
                assert_eq!(prior["context"], merge["context"]);
                assert!(clock(&prior["sequence"]) < clock(&merge["sequence"]));
            }
            let pass = events
                .iter()
                .find(|e| e["sequence"] == merge["context"])
                .unwrap();
            assert_eq!(pass["event"]["scope"]["kind"], "pass");
            assert_eq!(clock(&pass["event"]["scope"]["sweep"]), 0);
            let end = events
                .iter()
                .find(|e| {
                    e["event"]["kind"] == "context_end" && e["event"]["begin"] == pass["sequence"]
                })
                .unwrap();
            assert_eq!(end["event"]["outcome"]["kind"], "accepted");
            assert!(clock(&contribution["sequence"]) < clock(&end["sequence"]));
        }
    }
}
