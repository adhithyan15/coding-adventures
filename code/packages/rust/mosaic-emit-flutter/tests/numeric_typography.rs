use mosaic_emit_flutter::pipeline::{from_pipeline, PipelineEmitError};

fn fixture(value: &str, slot_type: &str) -> Result<String, PipelineEmitError> {
    let model = mosmodel_compiler::compile(&format!(
        "component Typography {{ slot text-size : {slot_type} ; slot content : text ; emit onEdit (value : text) ; emit onAction ; }}"
    )).unwrap();
    let layout = moslayout_compiler::compile(
        &format!(
            r#"
layout Typography {{ Column {{
    Text [ title ] (content: "Title", font-size: {value}, a11y-role: heading)
    HostInput [ editor ] (value: slot: content, font-size: {value}, onChange: emit: onEdit)
    Input [ legacy ] (value: "Legacy", font-size: {value})
    HostButton [ action ] (label: "Action", font-size: {value}, onClick: emit: onAction)
    Text (content: "Inherited", font-size: {value})
}} }}"#
        ),
        Some(&model.descriptor_json),
    )
    .unwrap();
    let style = mosstyle_compiler::compile(
        r##"style Typography {
        part title { font-size: 18; color: "#123456"; font-family: monospace; }
        part editor { font-size: 16; font-family: monospace; }
        part legacy { font-size: 15; }
        part action { font-size: 14; }
    }"##,
        None,
    )
    .unwrap();
    from_pipeline(&model.component, &layout.def, &style.def).map(|r| r.output)
}

#[test]
fn generate_numeric_typography_fixture() {
    let output = fixture("slot: text-size", "number").unwrap();
    assert_eq!(
        output
            .matches("copyWith(fontSize: _mosaicFontSize(this.textSize))")
            .count(),
        5
    );
    assert!(output.contains("fontFamily: \"monospace\""));
    assert!(output.contains("header: true"));
    if let Some(path) = std::env::var_os("MOSAIC_FLUTTER_TYPOGRAPHY_OUTPUT") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(path.join("lib")).unwrap();
        std::fs::create_dir_all(path.join("test")).unwrap();
        std::fs::write(path.join("lib/typography.dart"), output).unwrap();
        std::fs::write(path.join("pubspec.yaml"), "name: mosaic_typography\nenvironment:\n  sdk: '>=3.5.0 <4.0.0'\ndependencies:\n  flutter:\n    sdk: flutter\ndev_dependencies:\n  flutter_test:\n    sdk: flutter\n").unwrap();
        std::fs::write(
            path.join("test/typography_test.dart"),
            include_str!("fixtures/typography_test.dart"),
        )
        .unwrap();
        std::fs::write(path.join("lib/editors.dart"), editor_fixture()).unwrap();
        std::fs::write(
            path.join("test/input_lifecycle_test.dart"),
            include_str!("fixtures/input_lifecycle_test.dart"),
        )
        .unwrap();
    }
}

fn editor_fixture() -> String {
    let model =
        mosmodel_compiler::compile("component Editors { slot rows : list<text> ; }").unwrap();
    let layout = moslayout_compiler::compile(
        r#"layout Editors {
        HostTable { HostTableBody { For (each: slot: rows, as: row) {
            Row { HostInput (value: (row), a11y-label: "Cell editor") }
        } } }
    }"#,
        Some(&model.descriptor_json),
    )
    .unwrap();
    let style = mosstyle_compiler::compile("style Editors {}", None).unwrap();
    from_pipeline(&model.component, &layout.def, &style.def)
        .unwrap()
        .output
}

#[test]
fn table_editors_share_the_controller_lowering() {
    let output = editor_fixture();
    assert!(output.contains("value: ( row ),"));
    assert!(output.contains("controller: _mosaicController,"));
    assert!(!output.contains("TextEditingController(text: row)"));
}

#[test]
fn numeric_literals_and_invalid_authored_values() {
    assert!(fixture("22.5", "number")
        .unwrap()
        .contains("copyWith(fontSize: 22.5)"));
    for value in ["0", "\"18px\"", "(textSize)", "auto"] {
        assert!(fixture(value, "number").is_err(), "{value}");
    }
    assert!(fixture("slot: text-size", "text").is_err());
}

#[test]
fn nonfinite_and_negative_literals_are_rejected_by_projection() {
    use moslayout_compiler::{LayoutNode, LayoutProp, LayoutPropValue};
    for value in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let node = LayoutNode {
            tag: "Text".into(),
            part_name: None,
            children: vec![],
            props: vec![LayoutProp {
                name: "font-size".into(),
                value: LayoutPropValue::Number(value),
            }],
        };
        assert!(!mosaic_emit_flutter::pipeline::has_native_font_size(&node));
        let model = mosmodel_compiler::compile("component Typography {} ").unwrap();
        let style = mosstyle_compiler::compile("style Typography {}", None).unwrap();
        let layout = moslayout_compiler::LayoutDef {
            component_name: "Typography".into(),
            root: node,
        };
        assert!(from_pipeline(&model.component, &layout, &style.def).is_err());
    }
}
