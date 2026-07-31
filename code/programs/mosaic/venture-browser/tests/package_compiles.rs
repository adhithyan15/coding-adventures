use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn read(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

#[test]
fn venture_chrome_sources_compile_with_matching_theme_topology() {
    let interface =
        mosmodel_compiler::compile(&read("VentureChrome.mil")).expect("compile interface");
    let layout = moslayout_compiler::compile(
        &read("VentureChrome.mll"),
        Some(&interface.descriptor_json),
    )
    .expect("compile layout");
    let light = mosstyle_compiler::compile(
        &read("VentureChrome.light.msl"),
        Some(&layout.part_map_json),
    )
    .expect("compile light theme");
    let dark = mosstyle_compiler::compile(
        &read("VentureChrome.dark.msl"),
        Some(&layout.part_map_json),
    )
    .expect("compile dark theme");

    assert_eq!(interface.component.component, "VentureChrome");
    assert_eq!(layout.def.component_name, "VentureChrome");
    assert_eq!(light.def.component_name, "VentureChrome");
    assert_eq!(dark.def.component_name, "VentureChrome");

    let topology = |style: &mosstyle_compiler::StyleDef| {
        style
            .parts
            .iter()
            .map(|part| {
                let mut states: Vec<_> = part
                    .states
                    .iter()
                    .map(|state| state.state.clone())
                    .collect();
                states.sort_unstable();
                (part.name.clone(), states)
            })
            .collect::<BTreeMap<_, _>>()
    };
    assert_eq!(topology(&light.def), topology(&dark.def));
}

#[test]
fn interface_and_manifest_pin_the_browser_chrome_contract() {
    let interface =
        mosmodel_compiler::compile(&read("VentureChrome.mil")).expect("compile interface");
    let slots: Vec<_> = interface
        .component
        .slots
        .iter()
        .map(|slot| slot.name.as_str())
        .collect();
    assert_eq!(
        slots,
        [
            "address",
            "page-title",
            "status-text",
            "back-disabled",
            "forward-disabled",
            "navigation-disabled",
        ]
    );
    let events: Vec<_> = interface
        .component
        .emits
        .iter()
        .map(|event| event.name.as_str())
        .collect();
    assert_eq!(
        events,
        [
            "onBack",
            "onForward",
            "onHome",
            "onReload",
            "onAddressChange",
            "onNavigate",
        ]
    );

    let manifest = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("mosaic-package.toml"),
    )
    .expect("read manifest");
    let package = mosaic_package_manifest::parse(&manifest).expect("parse manifest");
    assert_eq!(package.package.name, "venture-browser");
    assert_eq!(package.components.exports, ["VentureChrome"]);
}
