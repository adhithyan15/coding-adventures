use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use engram_core_wasm::EngramSession;
use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};
use mosaic_package_resolver::{Resolution, Resolver};
use serde_json::Value;

const COMPONENTS: &[&str] = &["EngramApp"];

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn src_path(name: &str) -> PathBuf {
    package_root().join("src").join(name)
}

fn read_source(name: &str) -> String {
    let path = src_path(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

fn package_search_root() -> PathBuf {
    package_root()
        .join("..")
        .join("..")
        .join("..")
        .join("packages")
}

fn dependency_resolver() -> Resolver {
    mosaic_package_resolver::build(&package_root(), &[package_search_root()])
        .expect("Engram app dependencies should resolve")
}

#[test]
fn manifest_declares_app_package_boundary() {
    let manifest_src = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("mosaic-package.toml must exist");
    let package =
        mosaic_package_manifest::parse(&manifest_src).expect("manifest must parse and validate");

    assert_eq!(package.package.name, "engram-app");
    assert_eq!(package.components.exports, COMPONENTS);
    assert_eq!(
        package.dependencies.get("mosaic-pkg-deck-stats"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(
        package.dependencies.get("mosaic-pkg-review-card"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(
        package.dependencies.get("mosaic-pkg-session-progress"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(package.kernel.version, "1");
}

#[test]
fn app_sources_compile_without_owning_review_card_component() {
    let mil = mosmodel_compiler::compile(&read_source("EngramApp.mil"))
        .expect("EngramApp.mil should compile");
    let mll =
        moslayout_compiler::compile(&read_source("EngramApp.mll"), Some(&mil.descriptor_json))
            .expect("EngramApp.mll should compile against EngramApp.mil");
    let msl =
        mosstyle_compiler::compile(&read_source("EngramApp.dark.msl"), Some(&mll.part_map_json))
            .expect("EngramApp.dark.msl should compile against EngramApp.mll parts");

    assert_eq!(mil.component.component, "EngramApp");
    assert_eq!(mll.def.component_name, "EngramApp");
    assert_eq!(msl.def.component_name, "EngramApp");

    let source = read_source("EngramApp.mll");
    assert!(source.contains("pkg::mosaic-pkg-deck-stats::DeckStatsPanel"));
    assert!(source.contains("pkg::mosaic-pkg-review-card::ReviewCard"));
    assert!(source.contains("pkg::mosaic-pkg-session-progress::SessionProgress"));
    assert!(!source.contains("layout DeckStatsPanel"));
    assert!(!source.contains("layout ReviewCard"));
    assert!(!source.contains("layout SessionProgress"));
}

#[test]
fn shared_engram_app_props_match_mosaic_slots() {
    let mil = mosmodel_compiler::compile(&read_source("EngramApp.mil"))
        .expect("EngramApp.mil should compile");
    let expected_slots = mil
        .component
        .slots
        .iter()
        .map(|slot| slot.name.as_str())
        .collect::<BTreeSet<_>>();

    let session = EngramSession::new();
    let props: Value = serde_json::from_str(&session.engram_app_props("", 0))
        .expect("Engram app props should be valid JSON");
    let prop_keys = props["props"]
        .as_object()
        .expect("props should be a JSON object")
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    assert_eq!(props["ok"], true);
    assert_eq!(prop_keys, expected_slots);
    assert_eq!(props["props"]["answer-visible"], false);
}

#[test]
fn app_manifest_resolves_deck_stats_dependency() {
    let resolver = dependency_resolver();

    match resolver.resolve("DeckStatsPanel") {
        Some(Resolution::Component {
            package,
            component,
            package_path,
        }) => {
            assert_eq!(package, "mosaic-pkg-deck-stats");
            assert_eq!(component, "DeckStatsPanel");
            assert!(package_path.ends_with("mosaic-pkg-deck-stats"));
        }
        other => panic!("expected DeckStatsPanel component resolution, got {other:?}"),
    }
}

#[test]
fn app_manifest_resolves_review_card_dependency() {
    let resolver = dependency_resolver();

    match resolver.resolve("ReviewCard") {
        Some(Resolution::Component {
            package,
            component,
            package_path,
        }) => {
            assert_eq!(package, "mosaic-pkg-review-card");
            assert_eq!(component, "ReviewCard");
            assert!(package_path.ends_with("mosaic-pkg-review-card"));
        }
        other => panic!("expected ReviewCard component resolution, got {other:?}"),
    }
}

#[test]
fn app_manifest_resolves_session_progress_dependency() {
    let resolver = dependency_resolver();

    match resolver.resolve("SessionProgress") {
        Some(Resolution::Component {
            package,
            component,
            package_path,
        }) => {
            assert_eq!(package, "mosaic-pkg-session-progress");
            assert_eq!(component, "SessionProgress");
            assert!(package_path.ends_with("mosaic-pkg-session-progress"));
        }
        other => panic!("expected SessionProgress component resolution, got {other:?}"),
    }
}

#[test]
fn app_package_emits_multi_backend_artifacts_from_component_dependency() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let backends = [
        (Backend::Html, "html/EngramApp.html"),
        (Backend::React, "react/EngramApp.tsx"),
        (Backend::SwiftUI, "swiftui/EngramApp.swift"),
        (Backend::Qt, "qt/EngramApp.qml"),
        (Backend::Xaml, "xaml/EngramApp.xaml"),
        (Backend::Flutter, "flutter/EngramApp.dart"),
    ];

    for (backend, expected_artifact) in backends {
        let result = build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: false,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should build EngramApp: {e}"));

        assert_eq!(result.components_built, vec!["EngramApp"]);
        assert!(
            tmp.path().join(expected_artifact).exists(),
            "{backend:?} did not write {expected_artifact}"
        );
    }

    let html = fs::read_to_string(tmp.path().join("html").join("EngramApp.html"))
        .expect("EngramApp HTML artifact should be readable");
    assert!(
        html.contains("#2563eb"),
        "DeckStatsPanel package styles should reach EngramApp HTML"
    );
    assert!(
        html.contains("#e94560"),
        "ReviewCard package styles should reach EngramApp HTML"
    );
    assert!(
        html.contains("#0f766e"),
        "SessionProgress package styles should reach EngramApp HTML"
    );
    assert!(
        html.contains("#f87171"),
        "nested RatingControls package styles should reach EngramApp HTML"
    );
}

#[test]
fn app_package_emits_native_project_shells() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let shells = [
        (
            Backend::Qt,
            "qt",
            vec![
                "EngramApp.qml",
                "CMakeLists.txt",
                "main.cpp",
                "qmldir",
                "README.md",
            ],
        ),
        (
            Backend::SwiftUI,
            "swiftui",
            vec![
                "EngramApp.swift",
                "Package.swift",
                "README.md",
                "Sources/App/App.swift",
            ],
        ),
        (
            Backend::Xaml,
            "xaml",
            vec![
                "EngramApp.xaml",
                "EngramApp.xaml.cs",
                "EngramApp.Event.cs",
                "MosaicPackage.props",
                "EngramApp.csproj",
                "App.xaml",
                "MainWindow.xaml",
                "build.ps1",
            ],
        ),
    ];

    for (backend, dir_name, expected_files) in shells {
        let result = build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: true,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should emit EngramApp project shell: {e}"));

        assert_eq!(result.components_built, vec!["EngramApp"]);
        let backend_dir = tmp.path().join(dir_name);
        for file in expected_files {
            assert!(
                backend_dir.join(file).exists(),
                "{backend:?} project shell did not write {file}"
            );
        }
    }
}

#[test]
fn native_project_shells_expose_engram_host_contract() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    for backend in [
        Backend::React,
        Backend::Flutter,
        Backend::Qt,
        Backend::SwiftUI,
        Backend::Xaml,
    ] {
        build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: true,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should emit EngramApp project shell: {e}"));
    }

    let react_app = fs::read_to_string(
        tmp.path()
            .join("react")
            .join("src")
            .join("main.tsx"),
    )
    .expect("react/src/main.tsx");
    assert_contains(&react_app, "<EngramApp");
    assert_contains(&react_app, "appTitle=\"Sample AppTitle\"");
    assert_contains(&react_app, "answerVisible={false}");
    assert_contains(
        &react_app,
        "dispatch={(ev) => console.log(\"event:\", ev)}",
    );

    let flutter_app =
        fs::read_to_string(tmp.path().join("flutter").join("lib").join("main.dart"))
            .expect("flutter/lib/main.dart");
    assert_contains(&flutter_app, "EngramApp(");
    assert_contains(&flutter_app, "appTitle: \"Sample AppTitle\",");
    assert_contains(&flutter_app, "answerVisible: false,");
    assert_contains(
        &flutter_app,
        "dispatch: (event) => debugPrint(\"event: $event\")",
    );

    let qml =
        fs::read_to_string(tmp.path().join("qt").join("EngramApp.qml")).expect("EngramApp.qml");
    assert_contains(&qml, "property string appTitle");
    assert_contains(&qml, "property bool answerVisible");
    assert_contains(&qml, "signal reveal()");
    assert_contains(&qml, "signal again()");
    assert_contains(&qml, "signal hard()");
    assert_contains(&qml, "signal good()");
    assert_contains(&qml, "signal easy()");

    let swift = fs::read_to_string(tmp.path().join("swiftui").join("EngramApp.swift"))
        .expect("EngramApp.swift");
    assert_contains(&swift, "enum EngramAppEvent {");
    assert_contains(&swift, "case reveal");
    assert_contains(&swift, "case again");
    assert_contains(&swift, "case hard");
    assert_contains(&swift, "case good");
    assert_contains(&swift, "case easy");
    assert_contains(&swift, "struct EngramAppView: View");
    assert_contains(&swift, "let appTitle: String");
    assert_contains(&swift, "let answerVisible: Bool");
    let swift_app = fs::read_to_string(
        tmp.path()
            .join("swiftui")
            .join("Sources")
            .join("App")
            .join("App.swift"),
    )
    .expect("Sources/App/App.swift");
    assert_contains(&swift_app, "EngramAppView(");
    assert_contains(&swift_app, "appTitle: \"Sample AppTitle\",");
    assert_contains(&swift_app, "answerVisible: false,");
    assert_contains(&swift_app, "dispatch: { event in");

    let xaml_code_behind = fs::read_to_string(tmp.path().join("xaml").join("EngramApp.xaml.cs"))
        .expect("EngramApp.xaml.cs");
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty AppTitleProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty AnswerVisibleProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public event EventHandler<EngramAppEvent>? Dispatch;",
    );

    let xaml_events = fs::read_to_string(tmp.path().join("xaml").join("EngramApp.Event.cs"))
        .expect("EngramApp.Event.cs");
    assert_contains(
        &xaml_events,
        "public sealed record Reveal() : EngramAppEvent;",
    );
    assert_contains(
        &xaml_events,
        "public sealed record Again() : EngramAppEvent;",
    );
    assert_contains(
        &xaml_events,
        "public sealed record Hard() : EngramAppEvent;",
    );
    assert_contains(
        &xaml_events,
        "public sealed record Good() : EngramAppEvent;",
    );
    assert_contains(
        &xaml_events,
        "public sealed record Easy() : EngramAppEvent;",
    );

    let capi_header = fs::read_to_string(
        package_root()
            .join("..")
            .join("..")
            .join("..")
            .join("packages")
            .join("rust")
            .join("engram-capi")
            .join("include")
            .join("engram.h"),
    )
    .expect("engram-capi header");
    assert_contains(&capi_header, "eg_handle_engram_app_event");
}

fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "expected generated artifact to contain `{needle}`"
    );
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = ["EngramApp.mil", "EngramApp.mll", "EngramApp.dark.msl"];
    for name in expected {
        let path = src_path(name);
        assert!(
            path.exists(),
            "expected source file missing: {}",
            path.display()
        );
    }
}
