//! Venture browser-chrome acceptance gate for the XAML backend.
//!
//! Venture authors its browser controls once in Mosaic. This test ensures the
//! checked-in package keeps lowering to native WinUI controls and a complete
//! generated project shell without app-specific Win32 chrome.

use std::fs;
use std::path::PathBuf;

fn venture_src_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(|path| {
            path.join("programs")
                .join("mosaic")
                .join("venture-browser")
                .join("src")
        })
        .expect("derive Venture source root from CARGO_MANIFEST_DIR")
}

#[test]
fn venture_chrome_lowers_to_native_xaml_controls_and_project_shell() {
    let root = venture_src_root();
    let mil = fs::read_to_string(root.join("VentureChrome.mil")).expect("read VentureChrome.mil");
    let mll = fs::read_to_string(root.join("VentureChrome.mll")).expect("read VentureChrome.mll");
    let interface = mosmodel_compiler::compile(&mil).expect("compile Venture interface");
    let layout = moslayout_compiler::compile(&mll, Some(&interface.descriptor_json))
        .expect("compile Venture layout");

    for theme in ["VentureChrome.light.msl", "VentureChrome.dark.msl"] {
        let msl = fs::read_to_string(root.join(theme)).expect("read VentureChrome theme");
        let style = mosstyle_compiler::compile(&msl, Some(&layout.part_map_json))
            .expect("compile Venture style");
        let options = mosaic_emit_xaml::EmitOptions {
            emit_project: true,
            ..Default::default()
        };
        let result = mosaic_emit_xaml::from_pipeline(
            &interface.component,
            &layout.def,
            &style.def,
            None,
            &options,
        )
        .expect("emit Venture XAML project");

        assert!(result.xaml.contains(
            "<Button x:Name=\"BackButton\" AutomationProperties.AutomationId=\"back-button\" Content=\"Back\" IsEnabled=\"{x:Bind Not(BackDisabled)}\""
        ));
        assert!(result.xaml.contains(
            "<TextBox x:Name=\"AddressInput\" AutomationProperties.AutomationId=\"address-input\" Text=\"{x:Bind Address, Mode=TwoWay}\" IsReadOnly=\"{x:Bind NavigationDisabled}\""
        ));
        assert!(result
            .xaml
            .contains("TextChanged=\"AddressInput_TextChanged\""));
        assert!(result.xaml.contains("KeyDown=\"AddressInput_KeyDown\""));
        for part in ["back-button", "address-input", "go-button"] {
            assert!(
                result
                    .xaml
                    .contains(&format!("AutomationProperties.AutomationId=\"{part}\"")),
                "generated XAML chrome omits the native identifier for {part}"
            );
        }
        assert!(result
            .xaml
            .contains("Binding IsPressed, ElementName=GoButton"));
        assert!(result
            .xaml
            .contains("Binding FocusState, ElementName=AddressInput"));
        assert!(result
            .xaml
            .contains("<ContentPresenter Content=\"{x:Bind ContentSurface, Mode=OneWay}\"/>"));

        let project = result.project.expect("XAML project shell");
        assert!(project.csproj.contains("Microsoft.WindowsAppSDK"));
        assert!(project.main_window_xaml.contains("<gen:VentureChrome"));
        assert!(project
            .main_window_cs
            .contains("case VentureChromeEvent.AddressChange(var payload0) c:"));
        assert!(project
            .main_window_cs
            .contains("await TryHandleMosaicHostEvent(this.Component, ev)"));
        assert!(project.build_script.contains("dotnet build"));
    }
}
