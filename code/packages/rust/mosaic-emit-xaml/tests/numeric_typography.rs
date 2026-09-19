use mosaic_emit_xaml::{from_pipeline, EmitOptions, XamlEmitResult};

fn fixture(name: &str, value: &str) -> Result<XamlEmitResult, mosaic_emit_xaml::PipelineEmitError> {
    let model = mosmodel_compiler::compile(&format!(
        "component {name} {{ slot text-size : number ; slot headers : list<text> ; slot rows : list<list<text>> ; emit onAction ; }}"
    ))
    .unwrap();
    let layout = moslayout_compiler::compile(
        &format!(
            r#"layout {name} {{ Column {{
        Text [ title ] (content: "Title", font-size: {value})
        HostInput [ input ] (font-size: {value})
        HostButton [ button ] (label: "Action", font-size: {value})
        HostTable [ sheet ] (font-size: {value}) {{
            HostTableHead {{ Row {{ For (each: slot: headers, as: h, index: hi) {{
                Box {{ Text [ header-title ] (content: (h)) }}
            }} }} }}
            HostTableBody {{ For (each: slot: rows, as: row, index: ri) {{ Row {{
                For (each: row, as: cell, index: ci) {{ Box {{
                    Text [ cell-label ] (content: (cell))
                    HostInput [ cell-editor ] (value: (cell))
                    Box [ fixed-label ] {{ Text (content: "Fixed") }}
                }} }}
            }} }} }}
        }}
        Text [ outside ] (content: "Outside")
    }} }}"#
        ),
        Some(&model.descriptor_json),
    )
    .unwrap();
    let style = mosstyle_compiler::compile(&format!("style {name} {{ part title {{ font-size: 18; }} part input {{ font-size: 16; }} part button {{ font-size: 14; }} part cell-label {{ font-family: \"Consolas\"; }} part fixed-label {{ font-size: 22; }} }}"), Some(&layout.part_map_json)).unwrap();
    from_pipeline(
        &model.component,
        &layout.def,
        &style.def,
        None,
        &EmitOptions {
            emit_project: true,
            ..Default::default()
        },
    )
}

#[test]
fn numeric_font_binding_preserves_static_fallback_and_namespaces_helpers() {
    for name in ["Typography", "TypographyPeer"] {
        let result = fixture(name, "slot: text-size").unwrap();
        assert_eq!(
            result
                .xaml
                .matches(&format!(
                    "local:{name}MosaicFontSize.Value=\"{{x:Bind TextSize, Mode=OneWay}}\""
                ))
                .count(),
            3
        );
        assert_eq!(
            result
                .xaml
                .matches("MosaicFontSize.Value=\"{x:Bind Owner.TextSize, Mode=OneWay}\"")
                .count(),
            3
        );
        assert!(result.xaml.contains("FontFamily=\"Consolas\""));
        assert!(result.xaml.contains("Property=\"FontSize\" Value=\"22\""));
        for file in &result.for_view_models {
            if let Some(dir) = std::env::var_os("MOSAIC_TYPOGRAPHY_OUTPUT") {
                let dir = std::path::PathBuf::from(dir);
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(dir.join(&file.filename), &file.source).unwrap();
            }
        }
        for size in [18, 16, 14] {
            assert!(result.xaml.contains(&format!("FontSize=\"{size}\"")));
        }
        assert!(result
            .code_behind
            .contains("double.IsFinite(value) && value > 0"));
        assert!(result.code_behind.contains("target.ClearValue(property)"));
        assert!(result
            .code_behind
            .contains("target.ReadLocalValue(property)"));
        // Optional export for the real Windows XAML compiler acceptance job.
        if let Some(dir) = std::env::var_os("MOSAIC_TYPOGRAPHY_OUTPUT") {
            let dir = std::path::PathBuf::from(dir);
            std::fs::create_dir_all(&dir).unwrap();
            for (file, source) in [
                (format!("{name}.xaml"), &result.xaml),
                (format!("{name}.xaml.cs"), &result.code_behind),
                (format!("{name}.Event.cs"), &result.events),
            ] {
                std::fs::write(dir.join(file), source).unwrap();
            }
            if name == "Typography" {
                std::fs::write(
                    dir.join("MosaicHost.cs"),
                    include_str!("fixtures/numeric_typography_host.cs"),
                )
                .unwrap();
                let p = result.project.unwrap();
                for (file, source) in [
                    ("global.json", p.global_json),
                    ("Typography.csproj", p.csproj),
                    ("App.xaml", p.app_xaml),
                    ("App.xaml.cs", p.app_xaml_cs),
                    ("MainWindow.xaml", p.main_window_xaml),
                    ("MainWindow.xaml.cs", p.main_window_cs),
                    ("app.manifest", p.package_manifest),
                ] {
                    std::fs::write(dir.join(file), source).unwrap();
                }
            }
        }
    }
}

#[test]
fn numeric_font_literals_are_positive_and_finite() {
    assert!(fixture("Typography", "24")
        .unwrap()
        .xaml
        .contains("MosaicFontSize.Value=\"24\""));
    assert!(fixture("Typography", "0").is_err());
}
