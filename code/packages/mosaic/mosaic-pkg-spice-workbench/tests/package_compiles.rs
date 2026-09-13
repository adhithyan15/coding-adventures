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
        .any(|slot| slot.name == "schematic-title-disabled"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "schematic-wire-rows"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "open-schematic-label"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "undo-schematic-disabled"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "redo-schematic-disabled"));
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
        .any(|slot| slot.name == "schematic-analysis-kind-controls"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "schematic-analysis-kind-disabled"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "move-schematic-analysis-card-later-disabled"));
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
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "schematic-reference-disabled"));
    assert!(component
        .slots
        .iter()
        .any(|slot| slot.name == "remove-schematic-component-disabled"));
    assert!(component.emits.iter().any(|emit| emit.name == "onRun"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onSchematicTitleChange"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onOpenSchematic"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onSaveSchematic"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onUndoSchematic"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onRedoSchematic"));
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
        .any(|emit| emit.name == "onMoveSchematicAnalysisCardEarlier"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onMoveSchematicAnalysisCardLater"));
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
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onSchematicReferenceChange"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onRemoveSchematicComponent"));
    assert!(component
        .emits
        .iter()
        .any(|emit| emit.name == "onRemoveSchematicWire"));
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
    assert!(source("SpiceWorkbench.mll").contains("HostButton [ open-schematic ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostButton [ save-schematic ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostButton [ undo-schematic ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostButton [ redo-schematic ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostInput [ schematic-title-input ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostButton [ schematic-analysis-control ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostButton [ schematic-analysis-card ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostButton [ schematic-analysis-kind-control ]"));
    assert!(source("SpiceWorkbench.mll")
        .contains("HostButton [ move-schematic-analysis-card-earlier ]"));
    assert!(source("SpiceWorkbench.mll")
        .contains("HostButton [ move-schematic-analysis-card-later ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostButton [ remove-schematic-analysis-card ]"));
    assert!(
        source("SpiceWorkbench.mll").contains("HostButton [ schematic-analysis-source-option ]")
    );
    assert!(source("SpiceWorkbench.mll")
        .contains("HostInput [ schematic-analysis-parameter-one-input ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostInput [ schematic-value-input ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostInput [ schematic-reference-input ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostButton [ schematic-wire-remove ]"));
    assert!(source("SpiceWorkbench.mll").contains("HostButton [ remove-schematic-component ]"));
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
                assert!(output.contains("data-on-click=\"onOpenSchematic\""));
                assert!(output.contains("data-on-click=\"onSaveSchematic\""));
                assert!(output.contains("data-on-click=\"onUndoSchematic\""));
                assert!(output.contains("data-on-click=\"onRedoSchematic\""));
                assert!(output.contains("data-on-change=\"onSchematicTitleChange\""));
                assert!(output.contains("data-on-click=\"onPlaceSchematicComponent\""));
                assert!(output.contains("data-on-click=\"onAddSchematicAnalysis\""));
                assert!(output.contains("data-on-click=\"onSelectSchematicAnalysisCard\""));
                assert!(output.contains("data-on-click=\"onSelectSchematicAnalysis\""));
                assert!(output.contains("data-on-click=\"onMoveSchematicAnalysisCardEarlier\""));
                assert!(output.contains("data-on-click=\"onMoveSchematicAnalysisCardLater\""));
                assert!(output.contains("data-on-click=\"onRemoveSchematicAnalysisCard\""));
                assert!(output.contains("data-on-click=\"onSelectSchematicAnalysisSource\""));
                assert!(output.contains("data-on-change=\"onSchematicAnalysisParameterOneChange\""));
                assert!(output.contains("data-on-change=\"onSchematicValueChange\""));
                assert!(output.contains("data-on-change=\"onSchematicReferenceChange\""));
                assert!(output.contains("data-on-click=\"onRemoveSchematicComponent\""));
                assert!(output.contains("data-on-click=\"onRemoveSchematicWire\""));
            }
            Backend::React => {
                assert!(output.contains("<svg aria-hidden=\"true\""));
                assert!(output.contains("x1={( segment [ 0 ] )}"));
                assert!(output.contains("type: \"netlistChange\""));
                assert!(output.contains("type: \"run\""));
                assert!(output.contains("type: \"openSchematic\""));
                assert!(output.contains("type: \"saveSchematic\""));
                assert!(output.contains("type: \"undoSchematic\""));
                assert!(output.contains("type: \"redoSchematic\""));
                assert!(output.contains("type: \"schematicTitleChange\""));
                assert!(output.contains("type: \"placeSchematicComponent\""));
                assert!(output.contains("type: \"selectSchematicAnalysis\""));
                assert!(output.contains("type: \"moveSchematicAnalysisCardEarlier\""));
                assert!(output.contains("type: \"moveSchematicAnalysisCardLater\""));
                assert!(output.contains("type: \"selectSchematicAnalysisSource\""));
                assert!(output.contains("type: \"schematicAnalysisParameterOneChange\""));
                assert!(output.contains("type: \"schematicValueChange\""));
                assert!(output.contains("type: \"schematicReferenceChange\""));
                assert!(output.contains("type: \"removeSchematicComponent\""));
                assert!(output.contains("type: \"removeSchematicWire\""));
            }
            _ => unreachable!("the test only builds HTML and React artifacts"),
        }
    }
}
