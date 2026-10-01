use barcode_layout_1d::{
    compute_layout_v1, expand_binary_v1, expand_width_v1, project_scene_v1,
    project_scene_v1_with_resolver_probe, Barcode1DRun, Barcode1DRunColor, Barcode1DRunRole,
    Barcode1DSymbolDescriptor, Barcode1DSymbolRole, PaintBarcode1DOptions,
    RunsFromBinaryPatternOptions, RunsFromWidthPatternOptions,
};
use coding_adventures_sha256::sha256_hex;
use paint_instructions::PaintInstruction;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use text_interfaces::{FontQuery, FontResolutionError, FontResolver};

const CORPUS_SHA256: &str = "be95aa0381041ef3bd729b36bb4292f7a20692e139b53adca97af4e157cb7388";
const MAX_FIXTURE_BYTES: usize = 131_072;
const MAX_FIXTURE_DEPTH: usize = 8;
const MAX_FIXTURE_CASES: usize = 64;

#[derive(Debug)]
enum JsonFrame {
    Object {
        keys: HashSet<String>,
        expect_key: bool,
    },
    Array,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/fixtures/barcode-layout-1d-v1/cases.json")
}

fn read_bounded_fixture(path: &Path) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|_| "fixture-invalid-json".to_string())?;
    if file
        .metadata()
        .map_err(|_| "fixture-invalid-json".to_string())?
        .len()
        > MAX_FIXTURE_BYTES as u64
    {
        return Err("fixture-size-limit".to_string());
    }
    let mut encoded = Vec::new();
    file.take((MAX_FIXTURE_BYTES + 1) as u64)
        .read_to_end(&mut encoded)
        .map_err(|_| "fixture-invalid-json".to_string())?;
    if encoded.len() > MAX_FIXTURE_BYTES {
        return Err("fixture-size-limit".to_string());
    }
    if encoded.is_empty() {
        return Err("fixture-invalid-json".to_string());
    }
    Ok(encoded)
}

fn preflight_json(encoded: &[u8], depth_limit: usize) -> Result<(), String> {
    std::str::from_utf8(encoded).map_err(|_| "fixture-invalid-scalar".to_string())?;
    let mut frames = Vec::<JsonFrame>::new();
    let mut index = 0usize;
    while index < encoded.len() {
        match encoded[index] {
            b'"' => {
                let start = index;
                index += 1;
                let mut escaped = false;
                let mut closed = false;
                while index < encoded.len() {
                    let byte = encoded[index];
                    index += 1;
                    if escaped {
                        escaped = false;
                    } else if byte == b'\\' {
                        escaped = true;
                    } else if byte == b'"' {
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    return Err("fixture-invalid-json".to_string());
                }
                if let Some(JsonFrame::Object { keys, expect_key }) = frames.last_mut() {
                    if *expect_key {
                        let key: String = serde_json::from_slice(&encoded[start..index])
                            .map_err(|_| "fixture-invalid-scalar".to_string())?;
                        if !keys.insert(key) {
                            return Err("fixture-duplicate-key".to_string());
                        }
                        *expect_key = false;
                    }
                }
                continue;
            }
            b'{' => frames.push(JsonFrame::Object {
                keys: HashSet::new(),
                expect_key: true,
            }),
            b'[' => frames.push(JsonFrame::Array),
            b',' => {
                if let Some(JsonFrame::Object { expect_key, .. }) = frames.last_mut() {
                    *expect_key = true;
                }
            }
            b'}' | b']' => {
                frames.pop();
            }
            _ => {}
        }
        if frames.len() > depth_limit {
            return Err("fixture-depth-limit".to_string());
        }
        index += 1;
    }
    Ok(())
}

fn validate_json_tree(value: &Value, depth: usize) -> Result<(), String> {
    match value {
        Value::Array(values) => {
            let next_depth = depth + 1;
            if next_depth > MAX_FIXTURE_DEPTH {
                return Err("fixture-depth-limit".to_string());
            }
            for item in values {
                validate_json_tree(item, next_depth)?;
            }
        }
        Value::Object(values) => {
            let next_depth = depth + 1;
            if next_depth > MAX_FIXTURE_DEPTH {
                return Err("fixture-depth-limit".to_string());
            }
            for item in values.values() {
                validate_json_tree(item, next_depth)?;
            }
        }
        Value::Number(number) if number.as_i64().is_none() && number.as_u64().is_none() => {
            return Err("fixture-invalid-type".to_string());
        }
        _ => {}
    }
    Ok(())
}

fn validate_corpus(document: &Value) -> Result<(), String> {
    let root = document
        .as_object()
        .ok_or_else(|| "fixture-schema-invalid".to_string())?;
    let required = ["schema_version", "profile", "limits", "error_ids", "cases"];
    if root.len() != required.len() || required.iter().any(|key| !root.contains_key(*key)) {
        return Err("fixture-schema-invalid".to_string());
    }
    if root["schema_version"] != json!(1) || root["profile"] != json!("barcode-layout-1d-v1") {
        return Err("fixture-schema-invalid".to_string());
    }
    let limits = root["limits"]
        .as_object()
        .ok_or_else(|| "fixture-schema-invalid".to_string())?;
    for (key, expected) in [
        ("max_cases", MAX_FIXTURE_CASES as u64),
        ("max_fixture_bytes", MAX_FIXTURE_BYTES as u64),
        ("max_fixture_depth", MAX_FIXTURE_DEPTH as u64),
    ] {
        if limits.get(key).and_then(Value::as_u64) != Some(expected) {
            return Err("fixture-schema-invalid".to_string());
        }
    }
    let errors = root["error_ids"]
        .as_array()
        .ok_or_else(|| "fixture-schema-invalid".to_string())?;
    let expected_errors = [
        "pattern-too-long",
        "empty-pattern",
        "invalid-binary-token",
        "invalid-width-token",
        "invalid-marker-configuration",
        "invalid-module-count",
        "too-many-runs",
        "content-too-wide",
        "non-alternating-runs",
        "invalid-quiet-zone",
        "too-many-symbols",
        "symbol-width-mismatch",
        "invalid-render-config",
        "metadata-too-large",
        "human-readable-text-unsupported",
        "invalid-source-attribution",
    ];
    if errors.len() != expected_errors.len()
        || errors
            .iter()
            .zip(expected_errors)
            .any(|(actual, expected)| actual.as_str() != Some(expected))
    {
        return Err("fixture-schema-invalid".to_string());
    }
    let cases = root["cases"]
        .as_array()
        .ok_or_else(|| "fixture-schema-invalid".to_string())?;
    if cases.len() > MAX_FIXTURE_CASES {
        return Err("fixture-schema-invalid".to_string());
    }
    let operations = [
        "expand-binary",
        "expand-width",
        "compute-layout",
        "project-scene",
    ];
    let mut ids = HashSet::new();
    for test_case in cases {
        let case = test_case
            .as_object()
            .ok_or_else(|| "fixture-schema-invalid".to_string())?;
        let id = case
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "fixture-schema-invalid".to_string())?;
        let operation = case
            .get("operation")
            .and_then(Value::as_str)
            .ok_or_else(|| "fixture-schema-invalid".to_string())?;
        if !ids.insert(id) || !operations.contains(&operation) {
            return Err("fixture-schema-invalid".to_string());
        }
        if !case.contains_key("input") || !case.contains_key("expected") {
            return Err("fixture-schema-invalid".to_string());
        }
    }
    Ok(())
}

fn load_corpus(encoded: &[u8]) -> Result<Value, String> {
    if encoded.len() > MAX_FIXTURE_BYTES {
        return Err("fixture-size-limit".to_string());
    }
    preflight_json(encoded, MAX_FIXTURE_DEPTH)?;
    let document: Value =
        serde_json::from_slice(encoded).map_err(|_| "fixture-invalid-json".to_string())?;
    validate_json_tree(&document, 0)?;
    validate_corpus(&document)?;
    Ok(document)
}

fn run_role(value: &str) -> Barcode1DRunRole {
    match value {
        "data" => Barcode1DRunRole::Data,
        "start" => Barcode1DRunRole::Start,
        "stop" => Barcode1DRunRole::Stop,
        "guard" => Barcode1DRunRole::Guard,
        "check" => Barcode1DRunRole::Check,
        "inter-character-gap" => Barcode1DRunRole::InterCharacterGap,
        _ => panic!("unknown run role {value}"),
    }
}

fn symbol_role(value: &str) -> Barcode1DSymbolRole {
    match value {
        "data" => Barcode1DSymbolRole::Data,
        "start" => Barcode1DSymbolRole::Start,
        "stop" => Barcode1DSymbolRole::Stop,
        "guard" => Barcode1DSymbolRole::Guard,
        "check" => Barcode1DSymbolRole::Check,
        _ => panic!("unknown symbol role {value}"),
    }
}

fn color(value: &str) -> Barcode1DRunColor {
    match value {
        "bar" => Barcode1DRunColor::Bar,
        "space" => Barcode1DRunColor::Space,
        _ => panic!("unknown color {value}"),
    }
}

fn parse_run(value: &Value) -> Barcode1DRun {
    Barcode1DRun {
        color: color(value["color"].as_str().unwrap()),
        modules: value["modules"].as_u64().unwrap() as u32,
        source_label: value["sourceLabel"].as_str().unwrap().to_string(),
        source_index: value["sourceIndex"].as_i64().unwrap() as isize,
        role: run_role(value["role"].as_str().unwrap()),
    }
}

fn materialize_runs(input: &Value) -> Result<Vec<Barcode1DRun>, String> {
    if let Some(runs) = input["runs"].as_array() {
        return Ok(runs.iter().map(parse_run).collect());
    }
    let repeat = &input["repeatRuns"];
    let count = repeat["count"].as_u64().unwrap();
    if count > 40_979 {
        return Err("too-many-runs".to_string());
    }
    let count = count as usize;
    let mut next = color(repeat["firstColor"].as_str().unwrap());
    let mut runs = Vec::with_capacity(count);
    for _ in 0..count {
        runs.push(Barcode1DRun {
            color: next.clone(),
            modules: repeat["modules"].as_u64().unwrap() as u32,
            source_label: repeat["sourceLabel"].as_str().unwrap().to_string(),
            source_index: repeat["sourceIndex"].as_i64().unwrap() as isize,
            role: run_role(repeat["role"].as_str().unwrap()),
        });
        next = match next {
            Barcode1DRunColor::Bar => Barcode1DRunColor::Space,
            Barcode1DRunColor::Space => Barcode1DRunColor::Bar,
        };
    }
    Ok(runs)
}

fn materialize_pattern(input: &Value) -> Result<String, String> {
    if let Some(pattern) = input["pattern"].as_str() {
        return Ok(pattern.to_string());
    }
    let repeat = &input["repeat"];
    let token = repeat["token"].as_str().unwrap();
    let count = repeat["count"].as_u64().unwrap() as usize;
    let suffix = repeat["suffix"].as_str().unwrap_or("");
    let scalar_count = token
        .chars()
        .count()
        .checked_mul(count)
        .and_then(|size| size.checked_add(suffix.chars().count()))
        .ok_or_else(|| "pattern-too-long".to_string())?;
    if scalar_count > 65_567 {
        return Err("pattern-too-long".to_string());
    }
    Ok(format!("{}{}", token.repeat(count), suffix))
}

fn binary_options(input: &Value) -> RunsFromBinaryPatternOptions {
    RunsFromBinaryPatternOptions {
        source_label: input["sourceLabel"].as_str().unwrap().to_string(),
        source_index: input["sourceIndex"].as_i64().unwrap() as isize,
        role: run_role(input["role"].as_str().unwrap()),
    }
}

fn width_options(input: &Value) -> RunsFromWidthPatternOptions {
    let mut options = RunsFromWidthPatternOptions::new(
        input["sourceLabel"].as_str().unwrap(),
        input["sourceIndex"].as_i64().unwrap() as isize,
        run_role(input["role"].as_str().unwrap()),
    );
    if let Some(value) = input["narrowModules"].as_u64() {
        options.narrow_modules = value as u32;
    }
    if let Some(value) = input["wideModules"].as_u64() {
        options.wide_modules = value as u32;
    }
    if let Some(value) = input["narrowMarker"].as_str() {
        options.narrow_marker = value.chars().next().unwrap();
    }
    if let Some(value) = input["wideMarker"].as_str() {
        options.wide_marker = value.chars().next().unwrap();
    }
    if let Some(value) = input["startingColor"].as_str() {
        options.starting_color = color(value);
    }
    options
}

fn descriptors(input: &Value) -> Result<Option<Vec<Barcode1DSymbolDescriptor>>, String> {
    if let Some(values) = input["symbols"].as_array() {
        return Ok(Some(
            values
                .iter()
                .map(|value| Barcode1DSymbolDescriptor {
                    label: value["label"].as_str().unwrap().to_string(),
                    modules: value["modules"].as_u64().unwrap() as u32,
                    source_index: value["sourceIndex"].as_i64().unwrap() as isize,
                    role: symbol_role(value["role"].as_str().unwrap()),
                })
                .collect(),
        ));
    }
    if input.get("repeatSymbols").is_none() {
        return Ok(None);
    }
    let repeated = &input["repeatSymbols"];
    let count = repeated["count"].as_u64().unwrap() as usize;
    if count > 40_979 {
        return Err("too-many-symbols".to_string());
    }
    Ok(Some(
        (0..count)
            .map(|index| Barcode1DSymbolDescriptor {
                label: repeated["label"].as_str().unwrap().to_string(),
                modules: repeated["modules"].as_u64().unwrap() as u32,
                source_index: index as isize,
                role: symbol_role(repeated["role"].as_str().unwrap()),
            })
            .collect(),
    ))
}

fn run_json(run: &Barcode1DRun) -> Value {
    json!({
        "color": run.color.as_str(),
        "modules": run.modules,
        "role": run.role.as_str(),
        "sourceIndex": run.source_index,
        "sourceLabel": run.source_label,
    })
}

fn runs_result(runs: &[Barcode1DRun], digest: bool) -> Value {
    let values: Vec<Value> = runs.iter().map(run_json).collect();
    if !digest {
        return json!({"runs": values});
    }
    let encoded = serde_json::to_vec(&values).unwrap();
    json!({"runDigest": {
        "contentModules": runs.iter().map(|run| u64::from(run.modules)).sum::<u64>(),
        "firstRun": values.first().unwrap(),
        "lastRun": values.last().unwrap(),
        "runCount": runs.len(),
        "runsSha256": sha256_hex(&encoded),
    }})
}

fn layout_json(layout: &barcode_layout_1d::Barcode1DLayout) -> Value {
    json!({
        "contentModules": layout.content_modules,
        "leftQuietZoneModules": layout.left_quiet_zone_modules,
        "rightQuietZoneModules": layout.right_quiet_zone_modules,
        "symbolLayouts": layout.symbol_layouts.iter().map(|symbol| json!({
            "endModule": symbol.end_module,
            "label": symbol.label,
            "role": symbol.role.as_str(),
            "sourceIndex": symbol.source_index,
            "startModule": symbol.start_module,
        })).collect::<Vec<_>>(),
        "totalModules": layout.total_modules,
    })
}

fn scene_options(input: &Value) -> PaintBarcode1DOptions {
    let mut options = PaintBarcode1DOptions::default();
    let config = &input["renderConfig"];
    if let Some(value) = config["moduleWidth"].as_f64() {
        options.render_config.module_width = value;
    }
    if let Some(value) = config["barHeight"].as_f64() {
        options.render_config.bar_height = value;
    }
    if let Some(value) = config["foreground"].as_str() {
        options.render_config.foreground = value.to_string();
    }
    if let Some(value) = config["background"].as_str() {
        options.render_config.background = value.to_string();
    }
    if let Some(value) = config["includeHumanReadableText"].as_bool() {
        options.render_config.include_human_readable_text = value;
    }
    options.render_config.quiet_zone_modules = input["quietZoneModules"].as_u64().unwrap() as u32;
    options.human_readable_text = input["humanReadableText"].as_str().map(str::to_string);
    options.label = input["label"].as_str().map(str::to_string);
    if let Some(metadata) = input["metadata"].as_object() {
        options.metadata = metadata
            .iter()
            .map(|(key, value)| (key.clone(), value.as_str().unwrap().to_string()))
            .collect::<HashMap<_, _>>();
    }
    options.symbols = descriptors(input).unwrap_or_default();
    options
}

fn scene_json(scene: &paint_instructions::PaintScene) -> Value {
    let rectangles = scene
        .instructions
        .iter()
        .map(|instruction| match instruction {
            PaintInstruction::Rect(rectangle) => json!({
                "fill": rectangle.fill.as_deref().unwrap(),
                "height": rectangle.height as u64,
                "metadata": rectangle.base.metadata.as_ref().unwrap(),
                "width": rectangle.width as u64,
                "x": rectangle.x as u64,
                "y": rectangle.y as u64,
            }),
            _ => panic!("portable scene emitted a non-rectangle instruction"),
        })
        .collect::<Vec<_>>();
    let metadata = scene
        .metadata
        .as_ref()
        .unwrap()
        .iter()
        .map(|(key, value)| (key.clone(), Value::String(value.clone())))
        .collect::<Map<_, _>>();
    json!({
        "background": scene.background,
        "height": scene.height as u64,
        "metadata": metadata,
        "rectangles": rectangles,
        "width": scene.width as u64,
    })
}

fn execute(test_case: &Value) -> Result<Value, String> {
    let input = &test_case["input"];
    match test_case["operation"].as_str().unwrap() {
        "expand-binary" => {
            let pattern = materialize_pattern(input)?;
            let runs = expand_binary_v1(&pattern, &binary_options(input))?;
            Ok(runs_result(
                &runs,
                test_case["expected"].get("runDigest").is_some(),
            ))
        }
        "expand-width" => {
            let pattern = materialize_pattern(input)?;
            let runs = expand_width_v1(&pattern, &width_options(input))?;
            Ok(runs_result(
                &runs,
                test_case["expected"].get("runDigest").is_some(),
            ))
        }
        "compute-layout" => {
            let runs = materialize_runs(input)?;
            let symbols = descriptors(input)?;
            let layout = compute_layout_v1(
                &runs,
                input["quietZoneModules"].as_u64().unwrap() as u32,
                symbols.as_deref(),
            )?;
            Ok(json!({"layout": layout_json(&layout)}))
        }
        "project-scene" => {
            let runs = materialize_runs(input)?;
            let scene = project_scene_v1(&runs, &scene_options(input))?;
            Ok(json!({"scene": scene_json(&scene)}))
        }
        operation => panic!("unknown operation {operation}"),
    }
}

#[test]
fn executes_all_56_portable_v1_cases() {
    let encoded = read_bounded_fixture(&fixture_path()).unwrap();
    assert_eq!(sha256_hex(&encoded), CORPUS_SHA256);
    let document = load_corpus(&encoded).unwrap();
    let cases = document["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 56);
    let mut counts = HashMap::new();
    for test_case in cases {
        *counts
            .entry(test_case["operation"].as_str().unwrap())
            .or_insert(0usize) += 1;
        let expected = &test_case["expected"];
        match expected["error"].as_str() {
            Some(error_id) => assert_eq!(
                execute(test_case).unwrap_err(),
                error_id,
                "{}",
                test_case["id"]
            ),
            None => assert_eq!(
                execute(test_case).unwrap(),
                *expected,
                "{}",
                test_case["id"]
            ),
        }
    }
    assert_eq!(
        counts,
        HashMap::from([
            ("expand-binary", 12),
            ("expand-width", 12),
            ("compute-layout", 19),
            ("project-scene", 13)
        ])
    );
}

struct CountingResolver {
    calls: AtomicUsize,
}

impl CountingResolver {
    fn new() -> Self {
        Self {
            calls: AtomicUsize::new(0),
        }
    }
}

impl FontResolver for CountingResolver {
    type Handle = ();

    fn resolve(&self, _query: &FontQuery) -> Result<Self::Handle, FontResolutionError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(FontResolutionError::NoFamilyFound)
    }
}

#[test]
fn text_value_fails_before_native_resolution() {
    let runs = vec![Barcode1DRun {
        color: Barcode1DRunColor::Bar,
        modules: 1,
        source_label: "A".to_string(),
        source_index: 0,
        role: Barcode1DRunRole::Data,
    }];
    let resolver = CountingResolver::new();
    let supplied = PaintBarcode1DOptions {
        human_readable_text: Some("A".to_string()),
        ..PaintBarcode1DOptions::default()
    };
    assert_eq!(
        project_scene_v1_with_resolver_probe(&runs, &supplied, &resolver).unwrap_err(),
        "human-readable-text-unsupported"
    );
    assert_eq!(resolver.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn text_enabled_fails_before_native_resolution() {
    let runs = vec![Barcode1DRun {
        color: Barcode1DRunColor::Bar,
        modules: 1,
        source_label: "A".to_string(),
        source_index: 0,
        role: Barcode1DRunRole::Data,
    }];
    let resolver = CountingResolver::new();
    let mut enabled = PaintBarcode1DOptions::default();
    enabled.render_config.include_human_readable_text = true;
    enabled.render_config.module_width = 0.0;
    assert_eq!(
        project_scene_v1_with_resolver_probe(&runs, &enabled, &resolver).unwrap_err(),
        "human-readable-text-unsupported"
    );
    assert_eq!(resolver.calls.load(Ordering::SeqCst), 0);
}

fn write_temp_fixture(name: &str, encoded: &[u8]) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "barcode-layout-1d-{name}-{}-{}.json",
        std::process::id(),
        encoded.len()
    ));
    let mut file = File::create(&path).unwrap();
    file.write_all(encoded).unwrap();
    path
}

#[test]
fn bounded_fixture_loader_rejects_hostile_documents_before_dispatch() {
    let canonical = read_bounded_fixture(&fixture_path()).unwrap();

    let mut exact_max = canonical.clone();
    exact_max.resize(MAX_FIXTURE_BYTES, b' ');
    let exact_path = write_temp_fixture("exact-max", &exact_max);
    assert_eq!(
        read_bounded_fixture(&exact_path).unwrap().len(),
        MAX_FIXTURE_BYTES
    );
    assert!(load_corpus(&read_bounded_fixture(&exact_path).unwrap()).is_ok());
    fs::remove_file(exact_path).unwrap();

    let oversized_path = write_temp_fixture("oversized", &vec![b' '; MAX_FIXTURE_BYTES + 1]);
    assert_eq!(
        read_bounded_fixture(&oversized_path).unwrap_err(),
        "fixture-size-limit"
    );
    fs::remove_file(oversized_path).unwrap();

    assert_eq!(
        load_corpus(br#"{"a":1,"\u0061":2}"#).unwrap_err(),
        "fixture-duplicate-key"
    );
    let deep = format!(
        "{}0{}",
        "[".repeat(MAX_FIXTURE_DEPTH + 1),
        "]".repeat(MAX_FIXTURE_DEPTH + 1)
    );
    assert_eq!(
        load_corpus(deep.as_bytes()).unwrap_err(),
        "fixture-depth-limit"
    );
    assert_eq!(
        load_corpus(&[canonical.as_slice(), b"{}"].concat()).unwrap_err(),
        "fixture-invalid-json"
    );
    let scalar_error = load_corpus(br#"{"schema_version":1,"profile":"\ud800"}"#).unwrap_err();
    assert!(matches!(
        scalar_error.as_str(),
        "fixture-invalid-json" | "fixture-invalid-scalar"
    ));
    assert_eq!(load_corpus(b"[]").unwrap_err(), "fixture-schema-invalid");

    let mut too_many = load_corpus(&canonical).unwrap();
    let first = too_many["cases"][0].clone();
    too_many["cases"] = Value::Array(vec![first; MAX_FIXTURE_CASES + 1]);
    let too_many_encoded = serde_json::to_vec(&too_many).unwrap();
    assert_eq!(
        load_corpus(&too_many_encoded).unwrap_err(),
        "fixture-schema-invalid"
    );
}

#[test]
fn results_are_deep_owned() {
    let options = RunsFromBinaryPatternOptions {
        source_label: "A".to_string(),
        source_index: 0,
        role: Barcode1DRunRole::Data,
    };
    let mut first = expand_binary_v1("101", &options).unwrap();
    let second = expand_binary_v1("101", &options).unwrap();
    first[0].source_label = "mutated".to_string();
    assert_eq!(second[0].source_label, "A");
    assert_eq!(options.source_label, "A");
}
