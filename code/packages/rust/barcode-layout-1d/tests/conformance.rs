use barcode_layout_1d::{
    compute_layout_v1, expand_binary_v1, expand_width_v1, project_scene_v1, Barcode1DRun,
    Barcode1DRunColor, Barcode1DRunRole, Barcode1DSymbolDescriptor, Barcode1DSymbolRole,
    PaintBarcode1DOptions, RunsFromBinaryPatternOptions, RunsFromWidthPatternOptions,
};
use coding_adventures_sha256::sha256_hex;
use paint_instructions::PaintInstruction;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

const RAW_CASES: &[u8] =
    include_bytes!("../../../../specs/fixtures/barcode-layout-1d-v1/cases.json");
const CORPUS_SHA256: &str = "be95aa0381041ef3bd729b36bb4292f7a20692e139b53adca97af4e157cb7388";

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
    assert_eq!(sha256_hex(RAW_CASES), CORPUS_SHA256);
    let document: Value = serde_json::from_slice(RAW_CASES).unwrap();
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

#[test]
fn rejects_both_text_forms_before_native_dispatch() {
    let runs = vec![Barcode1DRun {
        color: Barcode1DRunColor::Bar,
        modules: 0,
        source_label: "A".to_string(),
        source_index: 0,
        role: Barcode1DRunRole::Data,
    }];
    let mut enabled = PaintBarcode1DOptions::default();
    enabled.render_config.include_human_readable_text = true;
    assert_eq!(
        project_scene_v1(&runs, &enabled).unwrap_err(),
        "human-readable-text-unsupported"
    );
    let supplied = PaintBarcode1DOptions {
        human_readable_text: Some("A".to_string()),
        ..PaintBarcode1DOptions::default()
    };
    assert_eq!(
        project_scene_v1(&runs, &supplied).unwrap_err(),
        "human-readable-text-unsupported"
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
