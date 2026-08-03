use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn read(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn read_package_file(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

#[test]
fn venture_chrome_sources_compile_with_matching_theme_topology() {
    let interface =
        mosmodel_compiler::compile(&read("VentureChrome.mil")).expect("compile interface");
    let layout =
        moslayout_compiler::compile(&read("VentureChrome.mll"), Some(&interface.descriptor_json))
            .expect("compile layout");
    let light = mosstyle_compiler::compile(
        &read("VentureChrome.light.msl"),
        Some(&layout.part_map_json),
    )
    .expect("compile light theme");
    let dark =
        mosstyle_compiler::compile(&read("VentureChrome.dark.msl"), Some(&layout.part_map_json))
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
        &slots[..venture_browser_core::VENTURE_CHROME_SLOT_NAMES.len()],
        venture_browser_core::VENTURE_CHROME_SLOT_NAMES
    );
    assert_eq!(
        slots.last().copied(),
        Some(venture_browser_core::VENTURE_CHROME_HOST_SURFACE_SLOT_NAME)
    );
    assert!(matches!(
        interface.component.slots.last().map(|slot| &slot.r#type),
        Some(mosmodel_compiler::SlotType::Node)
    ));
    let events: Vec<_> = interface
        .component
        .emits
        .iter()
        .map(|event| event.name.as_str())
        .collect();
    assert_eq!(events, venture_browser_core::VENTURE_CHROME_EVENT_NAMES);

    let manifest =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("mosaic-package.toml"))
            .expect("read manifest");
    let package = mosaic_package_manifest::parse(&manifest).expect("parse manifest");
    assert_eq!(package.package.name, "venture-browser");
    assert_eq!(package.components.exports, ["VentureChrome"]);
    assert_eq!(package.host_assets.files.len(), 2);
    assert_eq!(package.host_assets.files[0].backend, "swiftui");
    assert_eq!(
        package.host_assets.files[0].target,
        "Sources/App/MosaicHost.swift"
    );
    assert_eq!(package.host_assets.files[1].backend, "xaml");
    assert_eq!(
        package.host_assets.files[1].source,
        "host/xaml/MosaicHost.cs"
    );
    assert_eq!(package.host_assets.files[1].target, "MosaicHost.cs");

    let host = read_package_file("host/swiftui/MosaicHost.swift");
    for symbol in [
        "venture_browser_macos_apply_props",
        "venture_browser_macos_handle_event",
        "venture_browser_macos_render",
        "venture_browser_macos_scroll",
        "venture_browser_macos_scroll_command",
        "venture_browser_macos_activate_link",
        "venture_browser_macos_resize",
        "setPropsChangedHandler",
        "propsChangedHandler?()",
        "host?.resize(width: bounds.width, height: bounds.height)",
        "override func keyDown",
        "performNativeSurfaceWheel",
        "performNativeAddressCommit",
        "\"addressCommit\": \"native-return\"",
        "focusNativeSurface",
        "\"surfaceFocus\": \"native\"",
        "\"surfaceWheel\": \"scroll\"",
        "performNativeSurfaceClick",
        "lastSurfaceHistoryEvent",
        "\"surfaceHistory\": \"back-forward\"",
        "performNativeSurfaceResize",
        "\"surfaceResize\": \"native-reflow\"",
        "lastSurfaceRenderSize",
        "\"surfaceRepaint\": \"resized-frame\"",
        "native disabled Forward button dispatched",
        "native disabled Back button dispatched",
        "\"navigationState\": \"native-disabled-transitions\"",
        "navigateHistory(eventName:",
        "VENTURE_BROWSER_ACCEPTANCE_PATH",
        "\"backend\": \"swiftui\"",
    ] {
        assert!(host.contains(symbol), "SwiftUI host omits {symbol}");
    }

    let host = read_package_file("host/xaml/MosaicHost.cs");
    for symbol in [
        "venture_browser_windows_apply_props",
        "venture_browser_windows_handle_event",
        "venture_browser_windows_render_bgra",
        "venture_browser_windows_scroll",
        "venture_browser_windows_scroll_command",
        "venture_browser_windows_activate_link",
        "venture_browser_windows_resize",
        "WriteableBitmap",
        "component.ContentSurface",
        "SizeChanged += OnSizeChanged",
        "Native.Resize(browser, e.NewSize.Width, e.NewSize.Height)",
        "private void OnKeyDown",
        "RunFocusAcceptance",
        "CommitAddressWithEnter",
        "GetFocus()",
        "PostMessage(",
        "addressCommit = \"native-return\"",
        "FocusState != Microsoft.UI.Xaml.FocusState.Unfocused",
        "surfaceFocus = \"native\"",
        "RunWheelAcceptance",
        "ScrollByWheelDelta",
        "surfaceWheel = \"scroll\"",
        "RunHistoryKeyboardAcceptance",
        "surfaceHistory = \"back-forward\"",
        "RunPointerAcceptance",
        "RunResizeAcceptance",
        "surfaceResize = \"native-reflow\"",
        "acceptedRenderBaselineWidth",
        "surfaceRepaint = \"resized-frame\"",
        "initial native navigation control state did not match",
        "native navigation controls did not update after Forward",
        "navigationState = \"native-disabled-transitions\"",
        "ActivateSurfacePoint",
        "Focus(FocusState.Pointer)",
        "VentureContentSurface : ContentControl",
        "VENTURE_BROWSER_ACCEPTANCE_PATH",
        "\\\"backend\\\":\\\"xaml\\\"",
    ] {
        assert!(host.contains(symbol), "XAML host omits {symbol}");
    }
    let swift_host = read_package_file("host/swiftui/MosaicHost.swift");
    let xaml_host = read_package_file("host/xaml/MosaicHost.cs");
    for command in venture_browser_core::VENTURE_SCROLL_COMMAND_NAMES {
        assert!(
            swift_host.contains(command),
            "SwiftUI host omits shared scroll command {command}"
        );
        assert!(
            xaml_host.contains(command),
            "XAML host omits shared scroll command {command}"
        );
    }
}

#[test]
fn backend_build_scripts_cover_the_complete_matrix_and_direct_builds() {
    let shell = read_package_file("scripts/build-all.sh");
    let powershell = read_package_file("scripts/build-all.ps1");
    let backends = [
        "react",
        "electron",
        "swiftui",
        "qt",
        "webcomponent",
        "html",
        "xaml",
        "flutter",
        "compose",
    ];
    assert_eq!(
        backends.len(),
        mosaic_package_artifact_builder::Backend::ALL.len(),
        "the Venture build matrix must track Mosaic's exhaustive backend list"
    );

    for backend in backends {
        assert!(shell.contains(backend), "POSIX build omits {backend}");
        assert!(
            powershell.contains(backend),
            "PowerShell build omits {backend}"
        );
    }

    for required in [
        "--emit-project",
        "npm run build",
        "node --check",
        "swift build",
        "cargo \"${bridge_args[@]}\"",
        "libventure_browser_macos.dylib",
        "cmake --build",
        "dotnet build",
        "flutter build",
        "gradle --no-daemon build",
        "--strict",
    ] {
        assert!(shell.contains(required), "POSIX build omits {required}");
    }
    for required in [
        "--emit-project",
        "npm",
        "node",
        "swift",
        "venture-browser-macos",
        "libventure_browser_macos.dylib",
        "venture-browser-windows",
        "venture_browser_windows.dll",
        "-p:Platform=x64",
        "cmake",
        "dotnet",
        "flutter",
        "gradle",
        "$Strict",
    ] {
        assert!(
            powershell.contains(required),
            "PowerShell build omits {required}"
        );
    }
}
