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
    assert_eq!(package.host_assets.files.len(), 15);
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
        "host/flutter/mosaic_host.dart"
    );
    assert_eq!(package.host_assets.files[2].target, "lib/mosaic_host.dart");
    assert_eq!(package.host_assets.files[3].backend, "flutter");
    assert_eq!(
        package.host_assets.files[3].source,
        "host/flutter/venture_chrome_interaction_test.dart"
    );
    assert_eq!(
        package.host_assets.files[3].target,
        "test/venture_chrome_interaction_test.dart"
    );
    assert_eq!(package.host_assets.files[4].backend, "qt");
    assert_eq!(
        package.host_assets.files[4].source,
        "host/qt/MosaicHost.cpp"
    );
    assert_eq!(package.host_assets.files[4].target, "MosaicHost.cpp");
    assert_eq!(package.host_assets.files[5].backend, "qt");
    assert_eq!(package.host_assets.files[5].source, "host/qt/MosaicHost.h");
    assert_eq!(package.host_assets.files[5].target, "MosaicHost.h");
    assert_eq!(package.host_assets.files[6].backend, "qt");
    assert_eq!(
        package.host_assets.files[6].source,
        "host/qt/tst_venture_chrome.qml"
    );
    assert_eq!(
        package.host_assets.files[6].target,
        "test/tst_venture_chrome.qml"
    );
    assert_eq!(package.host_assets.files[7].backend, "compose");
    assert_eq!(
        package.host_assets.files[7].source,
        "host/compose/MosaicHost.kt"
    );
    assert_eq!(
        package.host_assets.files[7].target,
        "src/main/kotlin/MosaicHost.kt"
    );
    assert_eq!(package.host_assets.files[8].backend, "compose");
    assert_eq!(
        package.host_assets.files[8].source,
        "host/compose/VentureChromeInteractionTest.kt"
    );
    assert_eq!(
        package.host_assets.files[8].target,
        "src/test/kotlin/VentureChromeInteractionTest.kt"
    );
    for index in [9, 10] {
        assert_eq!(
            package.host_assets.files[index].source,
            "host/react/VentureChromeInteraction.test.tsx"
        );
        assert_eq!(
            package.host_assets.files[index].target,
            "src/VentureChromeInteraction.test.tsx"
        );
    }
    assert_eq!(package.host_assets.files[9].backend, "react");
    assert_eq!(package.host_assets.files[10].backend, "electron");
    for index in [11, 13] {
        assert_eq!(
            package.host_assets.files[index].source,
            "host/web/package.json"
        );
        assert_eq!(package.host_assets.files[index].target, "package.json");
    }
    for index in [12, 14] {
        assert_eq!(
            package.host_assets.files[index].source,
            "host/web/VentureChromeInteraction.test.js"
        );
        assert_eq!(
            package.host_assets.files[index].target,
            "test/VentureChromeInteraction.test.js"
        );
    }
    assert_eq!(package.host_assets.files[11].backend, "html");
    assert_eq!(package.host_assets.files[12].backend, "html");
    assert_eq!(package.host_assets.files[13].backend, "webcomponent");
    assert_eq!(package.host_assets.files[14].backend, "webcomponent");

    let host = read_package_file("host/swiftui/MosaicHost.swift");
    for symbol in [
        "venture_browser_macos_apply_props",
        "venture_browser_macos_handle_event",
        "venture_browser_macos_render",
        "venture_browser_macos_scroll",
        "venture_browser_macos_scroll_command",
        "venture_browser_macos_control_key",
        "venture_browser_macos_control_text",
        "venture_browser_macos_control_copy",
        "venture_browser_macos_control_cut",
        "venture_browser_macos_control_paste",
        "venture_browser_macos_caret_tick",
        "venture_browser_macos_ime_candidate_rect",
        "venture_browser_macos_scroll_metrics",
        "venture_browser_macos_scroll_to",
        "venture_browser_macos_activate_link",
        "venture_browser_macos_update_hover",
        "venture_browser_macos_resize",
        "setPropsChangedHandler",
        "propsChangedHandler?()",
        "host?.resize(width: bounds.width, height: bounds.height)",
        "override func keyDown",
        "host?.controlKey",
        "host?.controlText",
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
        "venture_browser_windows_control_key",
        "venture_browser_windows_control_text",
        "venture_browser_windows_control_copy",
        "venture_browser_windows_control_cut",
        "venture_browser_windows_control_paste",
        "venture_browser_windows_caret_tick",
        "venture_browser_windows_ime_candidate_rect",
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
        "CharacterReceived += OnCharacterReceived",
        "Native.ControlKey",
        "Native.ControlText",
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
    for symbol in [
        "file_picker_request",
        "control_file",
        "presentFilePickerIfRequested",
    ] {
        assert!(
            swift_host.contains(symbol),
            "SwiftUI file adapter omits {symbol}"
        );
    }
    for symbol in ["FilePickerRequest", "ControlFile", "FilePickerRequested"] {
        assert!(
            xaml_host.contains(symbol),
            "XAML file adapter omits {symbol}"
        );
    }
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

    let flutter_host = read_package_file("host/flutter/mosaic_host.dart");
    for symbol in [
        "venture_browser_flutter_new",
        "venture_browser_flutter_handle_event",
        "venture_browser_flutter_control_key",
        "venture_browser_flutter_control_text",
        "venture_browser_flutter_control_copy",
        "venture_browser_flutter_control_cut",
        "venture_browser_flutter_control_paste",
        "venture_browser_flutter_caret_tick",
        "venture_browser_flutter_ime_candidate_rect",
        "venture_browser_flutter_file_picker_request",
        "venture_browser_flutter_control_file",
        "submitPickedFile",
        "venture_browser_flutter_render_rgba",
        "VentureContentSurface",
        "PointerScrollEvent",
        "activateLink",
        "updateHover",
        "setPropsChangedHandler",
        "RawImage",
    ] {
        assert!(
            flutter_host.contains(symbol),
            "Flutter live host omits {symbol}"
        );
    }

    let flutter_acceptance = read_package_file("host/flutter/venture_chrome_interaction_test.dart");
    for symbol in [
        "MosaicApp(mosaicHost: host)",
        "package-owned Flutter shell drives the live shared browser and Cairo page",
        "MosaicHost.open",
        "tester.testTextInput.receiveAction(TextInputAction.done)",
        "PointerScrollEvent",
        "PointerHoverEvent",
        "Flutter Link Target",
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
        "MosaicHost::controlKey",
        "MosaicHost::controlText",
        "controlCopy_",
        "controlCut_",
        "controlPaste_",
        "caretTick_",
        "imeCandidateRect_",
        "filePickerRequest_",
        "controlFile_",
        "MosaicHost::presentFilePicker",
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

    let compose_host = read_package_file("host/compose/MosaicHost.kt");
    for symbol in [
        "venture_browser_compose_new",
        "venture_browser_compose_handle_event",
        "venture_browser_compose_control_key",
        "venture_browser_compose_control_text",
        "venture_browser_compose_control_copy",
        "venture_browser_compose_control_cut",
        "venture_browser_compose_control_paste",
        "venture_browser_compose_caret_tick",
        "venture_browser_compose_ime_candidate_rect",
        "venture_browser_compose_file_picker_request",
        "venture_browser_compose_control_file",
        "presentFilePickerIfRequested",
        "venture_browser_compose_render_rgba",
        "VentureContentSurface",
        "PointerEventType.Scroll",
        "activateLink",
        "updateHover",
        "setPropsChangedHandler",
        "toComposeImageBitmap",
    ] {
        assert!(
            compose_host.contains(symbol),
            "Compose live host omits {symbol}"
        );
    }

    let compose_acceptance = read_package_file("host/compose/VentureChromeInteractionTest.kt");
    for symbol in [
        "MosaicApp(host)",
        "packageOwnedComposeShellDrivesTheLiveSharedBrowserAndCairoPage",
        "MosaicHost.open",
        "performTextReplacement(targetUrl)",
        "performImeAction()",
        "performMouseInput",
        "Compose Link Target",
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
fn real_page_visual_fixture_remains_a_package_acceptance_dependency() {
    let capture = venture_browser_visual_fixtures::capture("http://venture.test")
        .expect("capture Venture's deterministic real-page fixture");
    capture.assert_valid();
    let controls = venture_browser_visual_fixtures::load_form_controls_page("http://venture.test")
        .expect("load Venture's deterministic form-control fixture");
    assert_eq!(controls.paint.controls.len(), 17);
    assert!(controls
        .paint
        .controls
        .iter()
        .any(|control| control.disabled));
}

#[test]
fn typed_input_fixture_uses_the_shared_host_neutral_value_contract() {
    use venture_browser_core::{BrowserControlModel, ControlAccessibilityAction, ControlKey};

    let tree = coding_adventures_html_parser::parse_browser_render_tree(
        "<input id='name' maxlength='3' value='é'>\
         <input id='mail' type='email' value='invalid'>\
         <input id='site' type='url' value='http://['>\
         <input id='count' type='number' min='0' max='6' step='2' value='3'>",
    )
    .expect("parse deterministic typed-input fixture");
    let mut controls = BrowserControlModel::from_render_tree(&tree);

    controls.focus("control:0:id:name");
    controls.text_input("abcd");
    assert_eq!(controls.control("control:0:id:name").unwrap().value, "éab");
    assert_eq!(
        controls
            .value_state("control:1:id:mail")
            .unwrap()
            .diagnostics[0]
            .code,
        "type-mismatch"
    );
    assert_eq!(
        controls
            .value_state("control:2:id:site")
            .unwrap()
            .diagnostics[0]
            .code,
        "type-mismatch"
    );

    controls.focus("control:3:id:count");
    assert!(
        !controls
            .value_state("control:3:id:count")
            .unwrap()
            .selection_supported
    );
    controls.key_down(ControlKey::ArrowUp);
    controls.accessibility_action(ControlAccessibilityAction::Increment);
    let number = controls.value_state("control:3:id:count").unwrap();
    assert_eq!(number.numeric_value, Some(6.0));
    assert!(number.is_valid());
}

#[test]
fn choice_range_fixture_uses_the_shared_host_neutral_contract() {
    use venture_browser_core::{BrowserControlModel, ControlAccessibilityAction, ControlKey};

    let tree = coding_adventures_html_parser::parse_browser_render_tree(
        "<select id='tags' multiple><option value='a' selected>A</option>\
         <optgroup disabled><option value='b'>B</option></optgroup>\
         <option value='c'>C</option></select>\
         <input id='check' type='checkbox'>\
         <input id='level' type='range' min='0' max='10' step='2' value='3'>",
    )
    .expect("parse deterministic choice/range fixture");
    let mut controls = BrowserControlModel::from_render_tree(&tree);

    controls.focus("control:0:id:tags");
    controls.key_down_with_shift(ControlKey::ArrowDown, true);
    assert_eq!(
        controls.selected_values("control:0:id:tags").unwrap(),
        vec!["a", "c"]
    );
    assert!(controls.choice_state("control:0:id:tags").unwrap().options[1].disabled);

    controls.focus("control:1:id:check");
    controls.accessibility_action(ControlAccessibilityAction::SetIndeterminate(true));
    controls.accessibility_action(ControlAccessibilityAction::Toggle);
    let checkbox = controls.choice_state("control:1:id:check").unwrap();
    assert_eq!(checkbox.checked, Some(true));
    assert!(!checkbox.indeterminate);

    controls.focus("control:2:id:level");
    controls.accessibility_action(ControlAccessibilityAction::Increment);
    let range = controls.choice_state("control:2:id:level").unwrap();
    assert_eq!(range.role, "slider");
    assert_eq!(range.value, Some(6.0));
}

#[test]
fn temporal_color_fixture_uses_the_shared_host_neutral_contract() {
    use venture_browser_core::{BrowserControlModel, ControlAccessibilityAction, ControlKey};

    let tree = coding_adventures_html_parser::parse_browser_render_tree(
        "<input id='day' type='date' min='2024-01-01' max='2024-01-09' step='2' value='2024-01-02'>\
         <input id='month' type='month' value='2024-07'>\
         <input id='week' type='week' value='2020-W53'>\
         <input id='clock' type='time' value='09:30:05.120' step='0.5'>\
         <input id='local' type='datetime-local' value='2024-02-29T09:30'>\
         <input id='ink' type='color' value='#A0b1C2'>",
    )
    .expect("parse deterministic temporal/color fixture");
    let mut controls = BrowserControlModel::from_render_tree(&tree);

    controls.focus("control:0:id:day");
    assert_eq!(
        controls
            .value_state("control:0:id:day")
            .unwrap()
            .diagnostics[0]
            .code,
        "step-mismatch"
    );
    controls.key_down(ControlKey::ArrowUp);
    controls.accessibility_action(ControlAccessibilityAction::Increment);
    assert_eq!(
        controls.control("control:0:id:day").unwrap().value,
        "2024-01-05"
    );
    assert_eq!(
        controls
            .value_state("control:3:id:clock")
            .unwrap()
            .value_text
            .as_deref(),
        Some("09:30:05.12")
    );
    assert_eq!(
        controls
            .value_state("control:4:id:local")
            .unwrap()
            .value_text
            .as_deref(),
        Some("2024-02-29T09:30")
    );

    controls.focus("control:5:id:ink");
    controls.accessibility_action(ControlAccessibilityAction::SetValue("#00FF7f".into()));
    assert_eq!(
        controls
            .value_state("control:5:id:ink")
            .unwrap()
            .value_text
            .as_deref(),
        Some("#00ff7f")
    );
}

#[test]
fn file_fixture_uses_the_shared_path_free_picker_contract() {
    use venture_browser_core::{
        BrowserControlModel, ControlEffect, FileAcceptFilter, HostFileSelection,
    };

    let tree = coding_adventures_html_parser::parse_browser_render_tree(
        "<form><input id='asset' name='asset' type='file' accept='image/*,.txt' multiple required></form>",
    )
    .expect("parse deterministic file fixture");
    let mut controls = BrowserControlModel::from_render_tree(&tree);
    let key = "control:0:id:asset";
    let request = controls.file_picker_request(key).unwrap();
    assert_eq!(
        request.accept,
        vec![
            FileAcceptFilter::MediaRange("image".into()),
            FileAcceptFilter::Extension(".txt".into()),
        ]
    );
    assert!(request.multiple);

    assert!(matches!(
        controls.apply_file_selection(
            key,
            vec![HostFileSelection::new(
                "fixture:1",
                "/host/private/receipt.txt",
                Some("text/plain".into()),
                b"venture".to_vec(),
            )],
        ),
        Some(ControlEffect::FilesChanged { .. })
    ));
    let state = controls.file_state(key).unwrap();
    assert_eq!(state.files[0].name, "receipt.txt");
    assert_eq!(state.files[0].size, 7);
    assert!(!controls
        .control(key)
        .unwrap()
        .value
        .contains("/host/private"));
}

#[test]
fn image_submit_and_dirname_fixture_uses_shared_semantics() {
    use venture_browser_core::{
        BrowserControlModel, ControlAccessibilityAction, ControlEffect, ControlTextDirection,
        ImageSubmitCoordinates,
    };

    let tree = coding_adventures_html_parser::parse_browser_render_tree(
        "<form dir='rtl'><input id='q' name='q' dirname='q.dir' dir='auto' value='שלום'>\
         <input id='pin' type='image' name='pin' alt='Choose location'></form>",
    )
    .expect("parse deterministic image/dirname fixture");
    let mut controls = BrowserControlModel::from_render_tree(&tree);
    assert_eq!(
        controls.directionality("control:0:id:q"),
        Some(ControlTextDirection::Rtl)
    );
    controls.focus("control:0:id:q");
    controls.accessibility_action(ControlAccessibilityAction::SetValue("Venture".into()));
    assert_eq!(
        controls.directionality("control:0:id:q"),
        Some(ControlTextDirection::Ltr)
    );
    controls.focus("control:1:id:pin");
    assert!(matches!(
        controls.accessibility_action(ControlAccessibilityAction::Activate),
        Some(ControlEffect::Activated(_))
    ));
    assert_eq!(
        ImageSubmitCoordinates::from_local_point(18.8, 4.2),
        ImageSubmitCoordinates { x: 18, y: 4 }
    );
}

#[test]
fn form_associated_custom_element_fixture_uses_shared_internals() {
    use venture_browser_core::{
        BrowserControlModel, CustomElementAccessibilityAction,
        CustomElementAccessibilityProjection, CustomElementAccessibilityValue,
        CustomElementFormValue, CustomElementLifecycleEvent, CustomElementValidity,
    };

    let tree = coding_adventures_html_parser::parse_browser_render_tree(
        "<label for='rating'>Rating</label><form id='review'>\
         <input name='before' value='a'>\
         <x-rating id='rating' name='score' role='slider'></x-rating>\
         <input name='after' value='z'></form>",
    )
    .expect("parse deterministic form-associated custom-element fixture");
    let mut controls = BrowserControlModel::from_render_tree(&tree);
    let key = controls.form_associated_custom_elements()[0].key.clone();
    controls
        .attach_form_associated_custom_element(&key)
        .unwrap();
    controls
        .set_custom_element_form_value(
            &key,
            Some(CustomElementFormValue::Text("3".into())),
            Some(CustomElementFormValue::Text("rating:3".into())),
        )
        .unwrap();
    controls
        .set_custom_element_accessibility_projection(
            &key,
            CustomElementAccessibilityProjection {
                role: Some("spinbutton".into()),
                name: Some("Review rating".into()),
                description: None,
            },
        )
        .unwrap();
    controls
        .set_custom_element_accessibility_value(
            &key,
            CustomElementAccessibilityValue {
                value_text: Some("3 of 5".into()),
                minimum: Some(1.0),
                maximum: Some(5.0),
                step: Some(1.0),
            },
        )
        .unwrap();
    controls
        .custom_element_accessibility_action(&key, CustomElementAccessibilityAction::Increment)
        .unwrap();
    let state = controls.custom_element_accessibility_state(&key).unwrap();
    assert_eq!(state.role.as_deref(), Some("spinbutton"));
    assert_eq!(state.value.as_deref(), Some("4"));
    assert_eq!(state.labels, vec!["Rating"]);

    controls
        .set_custom_element_validity(
            &key,
            CustomElementValidity {
                custom_error: true,
                ..CustomElementValidity::default()
            },
            Some("Choose five stars".into()),
            Some("rating".into()),
        )
        .unwrap();
    assert_eq!(
        controls.custom_element_diagnostics(Some("review"), 0).len(),
        1
    );
    controls.reset_form(Some("review"), Some(0));
    assert!(controls
        .take_custom_element_lifecycle_events()
        .iter()
        .any(|event| matches!(event, CustomElementLifecycleEvent::FormReset { .. })));
}

#[test]
fn scripted_form_lifecycle_fixture_is_host_neutral() {
    use coding_adventures_html_parser::{BrowserDocument, BrowserRenderTree};
    use venture_browser_core::{
        dispatch_request_submit, BrowserControlModel, FormActivation, FormDataEntry, FormDataValue,
        FormLifecycleEvent,
    };

    let source = "<form id='search' action='/find'><input name='q' value='venture'>\
                  <button id='go' name='intent' value='search'>Search</button></form>";
    let parsed = coding_adventures_html_parser::parse_html(source).unwrap();
    let tree =
        BrowserRenderTree::from_document_with_document_url(&parsed, "http://example.test/form");
    let document = BrowserDocument::from_document(&parsed);
    let controls = BrowserControlModel::from_render_tree(&tree);
    let outcome = dispatch_request_submit(
        &document,
        &controls,
        0,
        Some("control:1:id:go"),
        "http://example.test/form",
        |event| {
            if let Some(entries) = event.form_data_mut() {
                entries.push(FormDataEntry {
                    name: "host".into(),
                    value: FormDataValue::Text("shared".into()),
                });
            }
        },
    )
    .unwrap();
    assert!(matches!(
        outcome.events[0],
        FormLifecycleEvent::Submit { .. }
    ));
    assert!(matches!(
        outcome.events[1],
        FormLifecycleEvent::FormData { .. }
    ));
    let FormActivation::Navigate(navigation) = outcome.activation else {
        panic!("expected scripted navigation");
    };
    assert_eq!(
        navigation.url,
        "http://example.test/find?q=venture&intent=search&host=shared"
    );
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
    assert!(
        powershell.contains("cmd.exe /d /c \"java -version 2>&1\""),
        "Windows matrix must capture Java's stderr without turning it into a terminating error"
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
        "venture-browser-cairo",
        "libventure_browser_cairo",
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
        "venture-browser-cairo",
        "venture_browser_cairo",
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
