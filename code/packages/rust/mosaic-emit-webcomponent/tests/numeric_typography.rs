use mosaic_emit_webcomponent::pipeline::{
    from_pipeline_with_options, EmitOptions, PipelineEmitError, PipelineEmitResultWithProject,
};

fn fixture(
    value: &str,
    slot_type: &str,
) -> Result<PipelineEmitResultWithProject, PipelineEmitError> {
    let model = mosmodel_compiler::compile(&format!("component Typography {{ slot text-size : {slot_type} ; slot rows : list<text> ; emit onAction ; }}")).unwrap();
    let layout = moslayout_compiler::compile(
        &format!(
            r#"layout Typography {{ Column {{
        Text [ title ] (content: "Title", font-size: {value})
        HostInput [ editor ] (value: "Workbook", font-size: {value})
        Input [ legacy ] (value: "Notes", multiline: true, font-size: {value})
        HostButton [ action ] (label: "Action", font-size: {value}, onClick: emit: onAction)
        For (each: slot: rows, as: textSize) {{ Text (content: (textSize), font-size: {value}) }}
        HostTable {{
            HostTableHead {{ Row {{ Text [ header ] (content: "Header", font-size: {value}) }} }}
            HostTableBody {{
                Row {{ Text [ body ] (content: "Body", font-size: {value}) }}
                For (each: slot: rows, as: textSize) {{ Row {{ Text [ rowlabel ] (content: (textSize), font-size: {value}) }} }}
                Row {{ For (each: slot: rows, as: textSize) {{ Text [ celllabel ] (content: (textSize), font-size: {value}) }} }}
                Row {{ Text [ staticlabel ] (content: "Static") }}
            }}
            HostTableFoot {{ Row {{ Text [ footer ] (content: "Footer", font-size: {value}) }} }}
        }}
    }} }}"#
        ),
        Some(&model.descriptor_json),
    )
    .unwrap();
    let style = mosstyle_compiler::compile(
        r##"style Typography {
        part title { font-size: 18; color: "#123456"; font-family: monospace; }
        part header { font-size: 18; color: "#123456"; font-family: monospace; }
        part body { font-size: 18; color: "#123456"; font-family: monospace; }
        part rowlabel { font-size: 18; color: "#123456"; font-family: monospace; }
        part celllabel { font-size: 18; color: "#123456"; font-family: monospace; }
        part staticlabel { font-size: 18; color: "#123456"; font-family: monospace; }
        part footer { font-size: 18; color: "#123456"; font-family: monospace; }
        part editor { font-size: 16; }
        part legacy { font-size: 15; }
        part action { font-size: 14; }
    }"##,
        None,
    )
    .unwrap();
    from_pipeline_with_options(
        &model.component,
        &layout.def,
        &style.def,
        &EmitOptions {
            emit_project: true,
            ..Default::default()
        },
    )
}

#[test]
fn generate_numeric_typography_fixture() {
    let result = fixture("slot: text-size", "number").unwrap();
    assert!(result.output.contains("data-mosaic-font-size-slot"));
    if let Some(path) = std::env::var_os("MOSAIC_WC_TYPOGRAPHY_OUTPUT") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("Typography.js"), &result.output).unwrap();
        let project = result.project.unwrap();
        std::fs::write(path.join("index.html"), project.index_html).unwrap();
        std::fs::write(path.join("main.js"), project.main_js).unwrap();
        std::fs::write(
            path.join("acceptance.html"),
            include_str!("fixtures/acceptance.html"),
        )
        .unwrap();
    }
}

#[test]
fn literal_projection_and_invalid_authoring() {
    assert!(fixture("22.5", "number")
        .unwrap()
        .output
        .contains("font-size: 22.5px"));
    for bad in ["0", "\"18px\"", "(textSize)", "auto"] {
        assert!(fixture(bad, "number").is_err(), "{bad}");
    }
    assert!(fixture("slot: text-size", "text").is_err());
}
