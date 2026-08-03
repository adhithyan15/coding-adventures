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
    assert_eq!(package.host_assets.files.len(), 13);
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
    assert_eq!(package.host_assets.files[2].backend, "flutter");
    assert_eq!(
        package.host_assets.files[2].source,
        "host/flutter/venture_chrome_interaction_test.dart"
    );
    assert_eq!(
        package.host_assets.files[2].target,
        "test/venture_chrome_interaction_test.dart"
    );
    assert_eq!(package.host_assets.files[3].backend, "qt");
    assert_eq!(
        package.host_assets.files[3].source,
        "host/qt/MosaicHost.cpp"
    );
    assert_eq!(package.host_assets.files[3].target, "MosaicHost.cpp");
    assert_eq!(package.host_assets.files[4].backend, "qt");
    assert_eq!(package.host_assets.files[4].source, "host/qt/MosaicHost.h");
    assert_eq!(package.host_assets.files[4].target, "MosaicHost.h");
    assert_eq!(package.host_assets.files[5].backend, "qt");
    assert_eq!(
        package.host_assets.files[5].source,
        "host/qt/tst_venture_chrome.qml"
    );
    assert_eq!(
        package.host_assets.files[5].target,
        "test/tst_venture_chrome.qml"
    );
    assert_eq!(package.host_assets.files[6].backend, "compose");
    assert_eq!(
        package.host_assets.files[6].source,
        "host/compose/VentureChromeInteractionTest.kt"
    );
    assert_eq!(
        package.host_assets.files[6].target,
        "src/test/kotlin/VentureChromeInteractionTest.kt"
    );
    for index in [7, 8] {
        assert_eq!(
            package.host_assets.files[index].source,
            "host/react/VentureChromeInteraction.test.tsx"
        );
        assert_eq!(
            package.host_assets.files[index].target,
            "src/VentureChromeInteraction.test.tsx"
        );
    }
    assert_eq!(package.host_assets.files[7].backend, "react");
    assert_eq!(package.host_assets.files[8].backend, "electron");
    for index in [9, 11] {
        assert_eq!(
            package.host_assets.files[index].source,
            "host/web/package.json"
        );
        assert_eq!(package.host_assets.files[index].target, "package.json");
    }
    for index in [10, 12] {
        assert_eq!(
            package.host_assets.files[index].source,
            "host/web/VentureChromeInteraction.test.js"
        );
        assert_eq!(
            package.host_assets.files[index].target,
            "test/VentureChromeInteraction.test.js"
        );
    }
    assert_eq!(package.host_assets.files[9].backend, "html");
    assert_eq!(package.host_assets.files[10].backend, "html");
    assert_eq!(package.host_assets.files[11].backend, "webcomponent");
    assert_eq!(package.host_assets.files[12].backend, "webcomponent");

    let host = read_package_file("host/swiftui/MosaicHost.swift");
    for symbol in [
        "venture_browser_macos_apply_props",
        "venture_browser_macos_handle_event",
        "venture_browser_macos_render",
        "venture_browser_macos_scroll",
        "venture_browser_macos_scroll_command",
        "venture_browser_macos_scroll_metrics",
        "venture_browser_macos_scroll_to",
        "venture_browser_macos_activate_link",
        "venture_browser_macos_update_hover",
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
        "\"surfaceScrollbar\": \"native-projection\"",
        "NSScroller",
        "runScrollbarAcceptance",
        "performNativeSurfaceClick",
        "lastSurfaceHistoryEvent",
        "\"surfaceHistory\": \"back-forward\"",
        "\"surfaceHover\": \"status-and-cursor\"",
        "override func mouseMoved",
        "NSCursor.pointingHand.set()",
        "performNativeSurfaceResize",
        "\"surfaceResize\": \"native-reflow\"",
        "lastSurfaceRenderSize",
        "\"surfaceRepaint\": \"resized-frame\"",
        "initial native disabled navigation control dispatched",
        "failed navigation did not preserve the shared browser transaction",
        "\"failedNavigation\": \"transaction-retained\"",
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
        "venture_browser_windows_scroll_metrics",
        "venture_browser_windows_scroll_to",
        "venture_browser_windows_activate_link",
        "venture_browser_windows_update_hover",
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
        "surfaceScrollbar = \"native-projection\"",
        "RunScrollbarAcceptance",
        "ScrollBar",
        "RunHistoryKeyboardAcceptance",
        "surfaceHistory = \"back-forward\"",
        "surfaceHover = \"status-and-cursor\"",
        "PointerMoved += OnPointerMoved",
        "InputSystemCursorShape.Hand",
        "RunPointerAcceptance",
        "RunResizeAcceptance",
        "surfaceResize = \"native-reflow\"",
        "acceptedRenderBaselineWidth",
        "surfaceRepaint = \"resized-frame\"",
        "initial native navigation control state did not match",
        "native navigation controls did not update after Forward",
        "WaitForFailedNavigationAsync",
        "failedNavigation = \"transaction-retained\"",
        "navigationState = \"native-disabled-transitions\"",
        "ActivateSurfacePoint",
        "Focus(FocusState.Pointer)",
        "VentureContentSurface : ContentControl",
        "VENTURE_BROWSER_ACCEPTANCE_PATH",
        "\\\"backend\\\":\\\"xaml\\\"",
    ] {
        assert!(host.contains(symbol), "XAML host omits {symbol}");
    }
    let hover_acceptance = host
        .split("internal bool RunHoverAcceptance(string linkUrl)")
        .nth(1)
        .and_then(|source| source.split("internal async").next())
        .expect("extract WinUI hover acceptance");
    assert!(
        hover_acceptance.contains("HandleKey(VirtualKey.Home")
            && hover_acceptance.contains("UpdateHoverAt"),
        "WinUI must reset the shared viewport before checking the top-of-document link"
    );
    let pointer_acceptance = host
        .split("internal bool RunPointerAcceptance()")
        .nth(1)
        .and_then(|source| source.split("internal bool RunHoverAcceptance").next())
        .expect("extract WinUI pointer acceptance");
    assert!(
        pointer_acceptance.contains("ActivateSurfacePoint")
            && !pointer_acceptance.contains("HandleKey(VirtualKey.Home"),
        "WinUI click acceptance must reuse the viewport position established by hover"
    );
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

    let flutter_acceptance = read_package_file("host/flutter/venture_chrome_interaction_test.dart");
    for symbol in [
        "MosaicApp(mosaicHost: host)",
        "disabled native controls suppress Mosaic dispatch",
        "address edit, Return, and Go cross the Mosaic host seam",
        "tester.testTextInput.receiveAction(TextInputAction.done)",
        "Navigated through MosaicHost",
    ] {
        assert!(
            flutter_acceptance.contains(symbol),
            "Flutter interaction acceptance omits {symbol}"
        );
    }

    let qt_acceptance = read_package_file("host/qt/tst_venture_chrome.qml");
    for symbol in [
        "disabled_native_controls_suppress_dispatch",
        "address_return_crosses_the_mosaic_host_seam",
        "go_crosses_the_mosaic_host_seam",
        "keyClick(Qt.Key_Return)",
        "keyClick(Qt.Key_Space)",
        "Navigated through MosaicHost",
    ] {
        assert!(
            qt_acceptance.contains(symbol),
            "Qt interaction acceptance omits {symbol}"
        );
    }

    let qt_host = read_package_file("host/qt/MosaicHost.cpp");
    for symbol in [
        "VentureContentSurface::paint",
        "venture_browser_qt_",
        "MosaicHost::handleEvent",
        "MosaicHost::publishProps",
        "MosaicHost::activateLink",
        "MosaicHost::scrollCommand",
        "MosaicHost::scrollOffset",
        "MosaicHost::runInteractionAcceptance",
        "MosaicHost::scheduleAcceptance",
        "VENTURE_BROWSER_ACCEPTANCE_PATH",
        "VENTURE_BROWSER_INTERACTION_URL",
        "VENTURE_BROWSER_INTERACTION_LINK_URL",
        "generated address/history controls or live surface are unavailable",
        "native wheel did not scroll the shared viewport",
        "native hover did not project the live link URL",
        "historyControls",
        "surfaceMounted",
    ] {
        assert!(qt_host.contains(symbol), "Qt live host omits {symbol}");
    }

    let compose_acceptance = read_package_file("host/compose/VentureChromeInteractionTest.kt");
    for symbol in [
        "MosaicApp(host)",
        "disabledNativeControlsSuppressMosaicDispatch",
        "addressReturnAndGoCrossTheMosaicHostSeam",
        "performTextReplacement(\"http://venture.test/next\")",
        "performImeAction()",
        "Navigated through MosaicHost",
    ] {
        assert!(
            compose_acceptance.contains(symbol),
            "Compose interaction acceptance omits {symbol}"
        );
    }

    let react_acceptance = read_package_file("host/react/VentureChromeInteraction.test.tsx");
    for symbol in [
        "React and Electron renderer controls cross the Mosaic host seam",
        "button.click()",
        "enabledAddress.dispatchEvent",
        "mosaic-host-ready",
        "Navigated through MosaicHost",
    ] {
        assert!(
            react_acceptance.contains(symbol),
            "React/Electron interaction acceptance omits {symbol}"
        );
    }

    let web_acceptance = read_package_file("host/web/VentureChromeInteraction.test.js");
    for symbol in [
        "controls cross the Mosaic host seam",
        "disabled native buttons must suppress dispatch",
        "mosaic-host-ready",
        "addressChange",
        "Handled navigate through MosaicHost",
    ] {
        assert!(
            web_acceptance.contains(symbol),
            "HTML/Web Component interaction acceptance omits {symbol}"
        );
    }
}

#[test]
fn backend_build_scripts_cover_the_complete_matrix_and_direct_builds() {
    let build = read_package_file("BUILD");
    let build_windows = read_package_file("BUILD_windows");
    let shell = read_package_file("scripts/build-all.sh");
    let powershell = read_package_file("scripts/build-all.ps1");
    assert!(
        build.contains("./scripts/build-all.sh"),
        "POSIX BUILD must execute the generated-shell matrix"
    );
    assert!(
        build.starts_with("#!/bin/sh\n")
            && !build.contains("BASH_SOURCE")
            && !build.contains("pipefail"),
        "POSIX BUILD must remain compatible with the build tool's /bin/sh executor"
    );
    assert!(
        build_windows.contains("scripts\\build-all.ps1"),
        "Windows BUILD must execute the generated-shell matrix"
    );
    assert!(
        shell.contains("venture-browser-macos") && shell.contains("swiftui_project_launch"),
        "POSIX matrix must run the primary macOS direct-launch gate"
    );
    assert!(
        powershell.contains("venture-browser-windows") && powershell.contains("xaml_project_build"),
        "Windows matrix must run the primary Windows direct-launch gate"
    );
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
        "npm test",
        "npm audit --audit-level=high",
        "node --check",
        "swift build",
        "cargo \"${bridge_args[@]}\"",
        "libventure_browser_macos.dylib",
        "venture-browser-qt",
        "libventure_browser_qt",
        "qt_project_launch",
        "VENTURE_QT_ACCEPTANCE_REQUIRED=1",
        "cmake --build",
        "qmltestrunner -platform offscreen -style Basic -input test -import .",
        "dotnet build",
        "flutter build",
        "flutter test test/venture_chrome_interaction_test.dart",
        "gradle --no-daemon test build",
        "--strict",
    ] {
        assert!(shell.contains(required), "POSIX build omits {required}");
    }
    for required in [
        "--emit-project",
        "npm",
        "@(\"test\")",
        "@(\"audit\", \"--audit-level=high\")",
        "node",
        "swift",
        "venture-browser-macos",
        "libventure_browser_macos.dylib",
        "venture-browser-qt",
        "venture_browser_qt",
        "qt_project_launch",
        "VENTURE_QT_ACCEPTANCE_REQUIRED",
        "venture-browser-windows",
        "venture_browser_windows.dll",
        "-p:Platform=x64",
        "cmake",
        "qmltestrunner",
        "dotnet",
        "flutter",
        "venture_chrome_interaction_test.dart",
        "gradle",
        "@(\"--no-daemon\", \"test\", \"build\")",
        "$Strict",
    ] {
        assert!(
            powershell.contains(required),
            "PowerShell build omits {required}"
        );
    }
}
