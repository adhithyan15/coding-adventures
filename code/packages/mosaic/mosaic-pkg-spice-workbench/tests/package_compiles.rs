use std::{fs, path::PathBuf};

use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn source(name: &str) -> String {
    fs::read_to_string(root().join("src").join(name)).unwrap()
}

#[test]
fn manifest_and_component_contract_are_complete() {
    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(root().join("mosaic-package.toml")).unwrap()).unwrap();
    assert_eq!(
        manifest["package"]["name"].as_str(),
        Some("mosaic-pkg-spice-workbench")
    );
    assert_eq!(
        manifest["components"]["exports"].as_array().unwrap()[0].as_str(),
        Some("SpiceWorkbench")
    );

    let component = mosmodel_compiler::compile(&source("SpiceWorkbench.mil"))
        .unwrap()
        .component;
    assert_eq!(component.component, "SpiceWorkbench");
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "netlist-text"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "result-columns"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "diagnostic-rows"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "waveform-segments"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "schematic-wire-segments"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "schematic-analysis-controls"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "schematic-analysis-card-rows"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "schematic-analysis-source-options"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "schematic-analysis-parameter-three-disabled"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "schematic-value-disabled"));
    assert!(component.emits.iter().any(|emit| emit.name == "onRun"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onSelectWaveform"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onPlaceSchematicComponent"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onRouteToSchematicComponent"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onSelectSchematicAnalysis"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onAddSchematicAnalysis"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onSelectSchematicAnalysisCard"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onRemoveSchematicAnalysisCard"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onSelectSchematicAnalysisSource"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onSchematicAnalysisParameterThreeChange"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onSchematicValueChange"));
}

#[test]
fn multiline_workbench_compiles_in_both_themes() {
    let model = mosmodel_compiler::compile(&source("SpiceWorkbench.mil")).unwrap();
    let layout =
        moslayout_compiler::compile(&source("SpiceWorkbench.mll"), Some(&model.descriptor_json))
            .unwrap();
    assert!(source("SpiceWorkbench.mll").contains("Input [ netlist-input ]"));
    assert!(source("SpiceWorkbench.mll").contains("multiline : true"));
    assert!(source("SpiceWorkbench.mll").contains("HostTable [ result-table ]"));
    assert!(source("SpiceWorkbench.mll").contains("Path [ waveform-segment ]"));
    assert!(source("SpiceWorkbench.mll").contains("Path [ schematic-wire-segment ]"));
    assert!(source("SpiceWorkbench.mll").contains("Path [ schematic-terminal ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostButton [ schematic-analysis-control ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostButton [ schematic-analysis-card ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostButton [ remove-schematic-analysis-card ]"));
    assert!(
        source("SpiceWorkbench.mll").contains("HostButton [ schematic-analysis-source-option ]")
    );
    assert!(source("SpiceWorkbench.mll")
        .contains("HostInput [ schematic-analysis-parameter-one-input ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostInput [ schematic-value-input ]"));
    assert!(source("SpiceWorkbench.mll").contains("segment[0]"));
    assert!(source("SpiceWorkbench.mll").contains("slot: diagnostic-rows"));
    assert!(source("SpiceWorkbench.mll").contains("onClick : emit: onRun"));
    for theme in ["SpiceWorkbench.dark.msl", "SpiceWorkbench.light.msl"] {
        let style =
            mosstyle_compiler::compile(&source(theme), Some(&layout.part_map_json)).unwrap();
        assert_eq!(style.def.component_name, "SpiceWorkbench");
    }
}

#[test]
fn emitted_web_workbench_preserves_the_multiline_editor_and_actions() {
    let temporary = tempfile::tempdir().unwrap();
    for (backend, artifact) in [
        (Backend::Html, "html/SpiceWorkbench.html"),
        (Backend::React, "react/SpiceWorkbench.tsx"),
    ] {
        let result = build_package(&BuildOptions {
            package_root: root(),
            output_root: temporary.path().to_path_buf(),
            backend,
            emit_project: false,
            theme: Some("dark".to_owned()),
        })
        .unwrap();
        assert_eq!(result.components_built, vec!["SpiceWorkbench"]);
        let output = fs::read_to_string(temporary.path().join(artifact)).unwrap();
        assert!(output.contains("textarea"));
        match backend {
            Backend::Html => {
                assert!(output.contains("<table"));
                assert!(output.contains("<svg"));
                assert!(output.contains("x1=\"{{segment.0}}\""));
                assert!(output.contains("data-on-change=\"onNetlistChange\""));
                assert!(output.contains("data-on-click=\"onRun\""));
                assert!(output.contains("data-on-click=\"onPlaceSchematicComponent\""));
                assert!(output.contains("data-on-click=\"onAddSchematicAnalysis\""));
                assert!(output.contains("data-on-click=\"onSelectSchematicAnalysisCard\""));
                assert!(output.contains("data-on-click=\"onRemoveSchematicAnalysisCard\""));
                assert!(output.contains("data-on-click=\"onSelectSchematicAnalysisSource\""));
                assert!(output.contains("data-on-change=\"onSchematicAnalysisParameterOneChange\""));
                assert!(output.contains("data-on-change=\"onSchematicValueChange\""));
            }
            Backend::React => {
                assert!(output.contains("<svg aria-hidden=\"true\""));
                assert!(output.contains("x1={( segment [ 0 ] )}"));
                assert!(output.contains("type: \"netlistChange\""));
                assert!(output.contains("type: \"run\""));
                assert!(output.contains("type: \"placeSchematicComponent\""));
                assert!(output.contains("type: \"selectSchematicAnalysis\""));
                assert!(output.contains("type: \"selectSchematicAnalysisSource\""));
                assert!(output.contains("type: \"schematicAnalysisParameterOneChange\""));
                assert!(output.contains("type: \"schematicValueChange\""));
            }
            _ => unreachable!("the test only builds HTML and React artifacts"),
        }
    }
}
