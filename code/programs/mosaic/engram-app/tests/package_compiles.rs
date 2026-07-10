use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

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
    let packages_root = package_search_root();
    let mosaic_root = packages_root.join("mosaic");
    mosaic_package_resolver::build(&package_root(), &[packages_root, mosaic_root])
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
        package.dependencies.get("mosaic-pkg-card-browser"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(
        package.dependencies.get("mosaic-pkg-collection-actions"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(
        package.dependencies.get("mosaic-pkg-deck-options"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(
        package.dependencies.get("mosaic-pkg-deck-stats"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(
        package.dependencies.get("mosaic-pkg-note-editor"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(
        package.dependencies.get("mosaic-pkg-note-type-editor"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(
        package.dependencies.get("mosaic-pkg-review-card"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(
        package.dependencies.get("mosaic-pkg-review-actions"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(
        package.dependencies.get("mosaic-pkg-review-history"),
        Some(&"0.1.0".to_string())
    );
    assert_eq!(
        package.dependencies.get("mosaic-pkg-session-progress"),
        Some(&"0.1.0".to_string())
    );
    let host_assets = package
        .host_assets
        .files
        .iter()
        .map(|asset| {
            (
                asset.backend.as_str(),
                asset.source.as_str(),
                asset.target.as_str(),
            )
        })
        .collect::<BTreeSet<_>>();
    assert!(host_assets.contains(&("html", "host/web/engram-host.mjs", "engram-host.mjs")));
    assert!(host_assets.contains(&(
        "webcomponent",
        "host/web/engram-host.mjs",
        "engram-host.mjs"
    )));
    assert!(host_assets.contains(&("react", "host/web/engram-host.ts", "src/engram-host.ts")));
    assert!(host_assets.contains(&("electron", "host/electron/host.js", "electron/host.js")));
    assert!(host_assets.contains(&("qt", "host/qt/MosaicHost.cpp", "MosaicHost.cpp")));
    assert!(host_assets.contains(&(
        "swiftui",
        "host/swiftui/MosaicHost.swift",
        "Sources/App/MosaicHost.swift"
    )));
    assert!(host_assets.contains(&(
        "compose",
        "host/compose/MosaicHost.kt",
        "src/main/kotlin/MosaicHost.kt"
    )));
    assert!(host_assets.contains(&(
        "flutter",
        "host/flutter/mosaic_host.dart",
        "lib/mosaic_host.dart"
    )));
    assert!(host_assets.contains(&("xaml", "host/xaml/MosaicHost.cs", "MosaicHost.cs")));
    assert_eq!(package.kernel.version, "1");
}

#[test]
fn app_sources_compile_without_owning_review_card_component() {
    let mil = mosmodel_compiler::compile(&read_source("EngramApp.mil"))
        .expect("EngramApp.mil should compile");
    let mll =
        moslayout_compiler::compile(&read_source("EngramApp.mll"), Some(&mil.descriptor_json))
            .expect("EngramApp.mll should compile against EngramApp.mil");
    let touch_mll = moslayout_compiler::compile(
        &read_source("EngramApp.touch.mll"),
        Some(&mil.descriptor_json),
    )
    .expect("EngramApp.touch.mll should compile against EngramApp.mil");
    let dark_msl =
        mosstyle_compiler::compile(&read_source("EngramApp.dark.msl"), Some(&mll.part_map_json))
            .expect("EngramApp.dark.msl should compile against EngramApp.mll parts");
    let light_msl = mosstyle_compiler::compile(
        &read_source("EngramApp.light.msl"),
        Some(&mll.part_map_json),
    )
    .expect("EngramApp.light.msl should compile against EngramApp.mll parts");
    mosstyle_compiler::compile(
        &read_source("EngramApp.light.msl"),
        Some(&touch_mll.part_map_json),
    )
    .expect("EngramApp.light.msl should compile against EngramApp.touch.mll parts");

    assert_eq!(mil.component.component, "EngramApp");
    assert_eq!(mll.def.component_name, "EngramApp");
    assert_eq!(touch_mll.def.component_name, "EngramApp");
    assert_eq!(dark_msl.def.component_name, "EngramApp");
    assert_eq!(light_msl.def.component_name, "EngramApp");

    let source = read_source("EngramApp.mll");
    assert!(source.contains("pkg::mosaic-pkg-card-browser::CardBrowser"));
    assert!(source.contains("pkg::mosaic-pkg-collection-actions::CollectionActions"));
    assert!(source.contains("pkg::mosaic-pkg-deck-options::DeckOptionsPanel"));
    assert!(source.contains("pkg::mosaic-pkg-deck-stats::DeckStatsPanel"));
    assert!(source.contains("pkg::mosaic-pkg-note-editor::NoteEditor"));
    assert!(source.contains("pkg::mosaic-pkg-note-type-editor::NoteTypeEditor"));
    assert!(source.contains("pkg::mosaic-pkg-review-card::ReviewCard"));
    assert!(source.contains("pkg::mosaic-pkg-review-actions::ReviewActions"));
    assert!(source.contains("pkg::mosaic-pkg-review-history::ReviewHistoryPanel"));
    assert!(source.contains("pkg::mosaic-pkg-session-progress::SessionProgress"));
    assert!(!source.contains("layout CardBrowser"));
    assert!(!source.contains("layout CollectionActions"));
    assert!(!source.contains("layout DeckOptionsPanel"));
    assert!(!source.contains("layout DeckStatsPanel"));
    assert!(!source.contains("layout NoteEditor"));
    assert!(!source.contains("layout NoteTypeEditor"));
    assert!(!source.contains("layout ReviewCard"));
    assert!(!source.contains("layout ReviewActions"));
    assert!(!source.contains("layout ReviewHistoryPanel"));
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
    assert_eq!(props["props"]["host-status-visible"], false);
    assert_eq!(props["props"]["host-status-label"], "");
}

#[test]
fn app_manifest_resolves_card_browser_dependency() {
    let resolver = dependency_resolver();

    match resolver.resolve("CardBrowser") {
        Some(Resolution::Component {
            package,
            component,
            package_path,
        }) => {
            assert_eq!(package, "mosaic-pkg-card-browser");
            assert_eq!(component, "CardBrowser");
            assert!(package_path.ends_with("mosaic-pkg-card-browser"));
        }
        other => panic!("expected CardBrowser component resolution, got {other:?}"),
    }
}

#[test]
fn app_manifest_resolves_collection_actions_dependency() {
    let resolver = dependency_resolver();

    match resolver.resolve("CollectionActions") {
        Some(Resolution::Component {
            package,
            component,
            package_path,
        }) => {
            assert_eq!(package, "mosaic-pkg-collection-actions");
            assert_eq!(component, "CollectionActions");
            assert!(package_path.ends_with("mosaic-pkg-collection-actions"));
        }
        other => panic!("expected CollectionActions component resolution, got {other:?}"),
    }
}

#[test]
fn app_manifest_resolves_deck_options_dependency() {
    let resolver = dependency_resolver();

    match resolver.resolve("DeckOptionsPanel") {
        Some(Resolution::Component {
            package,
            component,
            package_path,
        }) => {
            assert_eq!(package, "mosaic-pkg-deck-options");
            assert_eq!(component, "DeckOptionsPanel");
            assert!(package_path.ends_with("mosaic-pkg-deck-options"));
        }
        other => panic!("expected DeckOptionsPanel component resolution, got {other:?}"),
    }
}

#[test]
fn app_manifest_resolves_review_history_dependency() {
    let resolver = dependency_resolver();

    match resolver.resolve("ReviewHistoryPanel") {
        Some(Resolution::Component {
            package,
            component,
            package_path,
        }) => {
            assert_eq!(package, "mosaic-pkg-review-history");
            assert_eq!(component, "ReviewHistoryPanel");
            assert!(package_path.ends_with("mosaic-pkg-review-history"));
        }
        other => panic!("expected ReviewHistoryPanel component resolution, got {other:?}"),
    }
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
fn app_manifest_resolves_note_editor_dependency() {
    let resolver = dependency_resolver();

    match resolver.resolve("NoteEditor") {
        Some(Resolution::Component {
            package,
            component,
            package_path,
        }) => {
            assert_eq!(package, "mosaic-pkg-note-editor");
            assert_eq!(component, "NoteEditor");
            assert!(package_path.ends_with("mosaic-pkg-note-editor"));
        }
        other => panic!("expected NoteEditor component resolution, got {other:?}"),
    }
}

#[test]
fn app_manifest_resolves_note_type_editor_dependency() {
    let resolver = dependency_resolver();

    match resolver.resolve("NoteTypeEditor") {
        Some(Resolution::Component {
            package,
            component,
            package_path,
        }) => {
            assert_eq!(package, "mosaic-pkg-note-type-editor");
            assert_eq!(component, "NoteTypeEditor");
            assert!(package_path.ends_with("mosaic-pkg-note-type-editor"));
        }
        other => panic!("expected NoteTypeEditor component resolution, got {other:?}"),
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
fn app_manifest_resolves_review_actions_dependency() {
    let resolver = dependency_resolver();

    match resolver.resolve("ReviewActions") {
        Some(Resolution::Component {
            package,
            component,
            package_path,
        }) => {
            assert_eq!(package, "mosaic-pkg-review-actions");
            assert_eq!(component, "ReviewActions");
            assert!(package_path.ends_with("mosaic-pkg-review-actions"));
        }
        other => panic!("expected ReviewActions component resolution, got {other:?}"),
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
        (
            Backend::Html,
            "html/EngramApp.html",
            "html/EngramApp.touch.html",
        ),
        (
            Backend::WebComponent,
            "webcomponent/EngramApp.js",
            "webcomponent/EngramApp.touch.js",
        ),
        (
            Backend::React,
            "react/EngramApp.tsx",
            "react/EngramApp.touch.tsx",
        ),
        (
            Backend::Electron,
            "electron/EngramApp.tsx",
            "electron/EngramApp.touch.tsx",
        ),
        (
            Backend::SwiftUI,
            "swiftui/EngramApp.swift",
            "swiftui/EngramApp.touch.swift",
        ),
        (Backend::Qt, "qt/EngramApp.qml", "qt/EngramApp.touch.qml"),
        (
            Backend::Xaml,
            "xaml/EngramApp.xaml",
            "xaml/EngramApp.touch.xaml",
        ),
        (
            Backend::Flutter,
            "flutter/EngramApp.dart",
            "flutter/EngramApp.touch.dart",
        ),
        (
            Backend::Compose,
            "compose/EngramApp.kt",
            "compose/EngramApp.touch.kt",
        ),
    ];

    for (backend, expected_artifact, expected_touch_artifact) in backends {
        let result = build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: false,
            theme: None,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should build EngramApp: {e}"));

        assert_eq!(result.components_built, vec!["EngramApp"]);
        assert!(
            tmp.path().join(expected_artifact).exists(),
            "{backend:?} did not write {expected_artifact}"
        );
        assert!(
            tmp.path().join(expected_touch_artifact).exists(),
            "{backend:?} did not write {expected_touch_artifact}"
        );
    }

    let html = read_artifact(tmp.path(), "html/EngramApp.html");
    assert_contains(&html, "data-on-click=\"onImportAnki\"");
    assert_contains(&html, "data-on-click=\"onPruneUnusedMedia\"");
    assert_contains(&html, "data-on-commit=\"onBrowserSearch\"");
    assert_contains(&html, "data-on-change=\"onBrowserTagEditChange\"");
    assert_contains(&html, "data-on-click=\"onBrowserAddTagSelected\"");
    assert_contains(&html, "data-on-click=\"onBrowserRemoveTagSelected\"");
    assert_contains(&html, "data-on-change=\"onBrowserCustomStudyLimitChange\"");
    assert_contains(
        &html,
        "data-on-toggle=\"onBrowserCustomStudyRescheduleChange\"",
    );
    assert_contains(&html, "data-on-click=\"onBrowserRebuildFilteredDeck\"");
    assert_contains(&html, "data-on-click=\"onBrowserEmptyFilteredDeck\"");
    assert_contains(&html, "data-on-click=\"onBrowserToggleFilter\"");
    assert_contains(&html, "data-on-click=\"onBrowserSetFilter\"");
    assert_contains(&html, "data-on-click=\"onBrowserToggleFlagPicker\"");
    assert_contains(&html, "data-on-click=\"onBrowserSetFlagSelected\"");
    assert_contains(&html, "data-on-click=\"onNoteEditorSelectNoteType\"");
    assert_contains(&html, "data-on-click=\"onNoteEditorSelectDeck\"");
    assert_contains(&html, "data-on-click=\"onNoteEditorSelectField\"");
    assert_contains(&html, "data-on-change=\"onNoteEditorFieldValueChange\"");
    assert_contains(&html, "data-on-change=\"onNoteEditorTagsChange\"");
    assert_contains(&html, "data-on-click=\"onNoteEditorSaveNote\"");
    assert_contains(&html, "data-on-click=\"onNoteEditorDeleteNote\"");
    assert_contains(&html, "data-on-click=\"onNoteEditorCancel\"");
    assert_contains(&html, "data-on-click=\"onNoteTypeEditorSelectNoteType\"");
    assert_contains(&html, "data-on-click=\"onNoteTypeEditorSelectField\"");
    assert_contains(&html, "data-on-click=\"onNoteTypeEditorSelectTemplate\"");
    assert_contains(&html, "data-on-change=\"onNoteTypeEditorNameChange\"");
    assert_contains(&html, "data-on-change=\"onNoteTypeEditorFieldNameChange\"");
    assert_contains(
        &html,
        "data-on-toggle=\"onNoteTypeEditorFieldRequiredChange\"",
    );
    assert_contains(
        &html,
        "data-on-change=\"onNoteTypeEditorTemplateNameChange\"",
    );
    assert_contains(
        &html,
        "data-on-change=\"onNoteTypeEditorFrontTemplateChange\"",
    );
    assert_contains(
        &html,
        "data-on-change=\"onNoteTypeEditorBackTemplateChange\"",
    );
    assert_contains(&html, "data-on-change=\"onNoteTypeEditorStylesheetChange\"");
    assert_contains(&html, "data-on-click=\"onNoteTypeEditorNewNoteType\"");
    assert_contains(&html, "data-on-click=\"onNoteTypeEditorSaveNoteType\"");
    assert_contains(&html, "data-on-click=\"onNoteTypeEditorDeleteNoteType\"");
    assert_contains(&html, "data-on-click=\"onNoteTypeEditorCancel\"");
    assert_dependency_styles_reach_all_backends(tmp.path());
}

#[test]
fn app_package_light_theme_selects_light_app_shell_styles() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let result = build_package(&BuildOptions {
        package_root: package_root(),
        output_root: tmp.path().to_path_buf(),
        backend: Backend::React,
        emit_project: false,
        theme: Some("light".to_string()),
    })
    .expect("React light-theme package build should compile EngramApp");

    assert_eq!(result.components_built, vec!["EngramApp"]);
    let react = read_artifact(tmp.path(), "react/EngramApp.tsx");
    assert_contains(&react, "#ffffff");
    assert_contains(&react, "#1e40af");
    assert!(
        !react.contains("#101827"),
        "light-theme build should not select the dark app-shell background"
    );
}

#[test]
fn app_package_emits_native_project_shells() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let shells = [
        (
            Backend::Html,
            "html",
            vec!["EngramApp.html", "index.html", "main.js", "README.md"],
        ),
        (
            Backend::WebComponent,
            "webcomponent",
            vec![
                "EngramApp.js",
                "index.js",
                "index.html",
                "main.js",
                "README.md",
            ],
        ),
        (
            Backend::React,
            "react",
            vec![
                "EngramApp.tsx",
                "index.ts",
                "package.json",
                "vite.config.ts",
                "index.html",
                "tsconfig.json",
                "src/main.tsx",
                "src/engram-host.ts",
                "README.md",
            ],
        ),
        (
            Backend::Electron,
            "electron",
            vec![
                "EngramApp.tsx",
                "index.ts",
                "package.json",
                "vite.config.ts",
                "index.html",
                "tsconfig.json",
                "tsconfig.electron.json",
                "src/main.tsx",
                "electron/main.ts",
                "electron/preload.ts",
                "README.md",
            ],
        ),
        (
            Backend::Flutter,
            "flutter",
            vec![
                "EngramApp.dart",
                "index.dart",
                "pubspec.yaml",
                "README.md",
                "lib/main.dart",
                "lib/mosaic_host.dart",
            ],
        ),
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
                "Sources/App/EngramApp.swift",
            ],
        ),
        (
            Backend::Compose,
            "compose",
            vec![
                "EngramApp.kt",
                "index.kt",
                "settings.gradle.kts",
                "build.gradle.kts",
                "src/main/kotlin/Main.kt",
                "src/main/kotlin/EngramApp.kt",
                "src/main/kotlin/MosaicHost.kt",
                "README.md",
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
            theme: None,
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
        Backend::Html,
        Backend::WebComponent,
        Backend::React,
        Backend::Electron,
        Backend::Flutter,
        Backend::Compose,
        Backend::Qt,
        Backend::SwiftUI,
        Backend::Xaml,
    ] {
        build_package(&BuildOptions {
            package_root: package_root(),
            output_root: tmp.path().to_path_buf(),
            backend,
            emit_project: true,
            theme: None,
        })
        .unwrap_or_else(|e| panic!("{backend:?} should emit EngramApp project shell: {e}"));
    }

    let html_index =
        fs::read_to_string(tmp.path().join("html").join("index.html")).expect("html/index.html");
    assert_contains(&html_index, "data-mosaic-html-root=\"EngramApp\"");
    assert_contains(&html_index, "src=\"./engram-host.mjs\"");
    assert_contains(&html_index, "src=\"./main.js\"");
    assert!(
        html_index
            .find("src=\"./engram-host.mjs\"")
            .expect("html host script")
            < html_index
                .find("src=\"./main.js\"")
                .expect("html main script"),
        "HTML host adapter should load before generated main.js"
    );
    let html_main =
        fs::read_to_string(tmp.path().join("html").join("main.js")).expect("html/main.js");
    assert_contains(
        &html_main,
        "window.mosaicHost?.getProps?.({ component: componentName })",
    );
    assert_contains(
        &html_main,
        "window.mosaicHost.handleEvent({ component: componentName, event })",
    );
    assert_contains(&html_main, "const componentName = \"EngramApp\";");
    assert_contains(&html_main, "\"deckNames\": []");
    assert_contains(&html_main, "\"onSelectDeck\"");
    assert_contains(&html_main, "\"browserResults\": []");
    assert_contains(&html_main, "\"answerVisible\": false");
    assert_contains(&html_main, "\"typeAnswerActive\": false");
    assert_contains(&html_main, "\"onTypeAnswerChange\"");
    assert_contains(&html_main, "\"onBrowserSelectResult\"");
    assert_contains(&html_main, "\"onBrowserTagEditChange\"");
    assert_contains(&html_main, "\"onBrowserAddTagSelected\"");
    assert_contains(&html_main, "\"onBrowserRemoveTagSelected\"");
    assert_contains(&html_main, "\"onBrowserCustomStudyLimitChange\"");
    assert_contains(&html_main, "\"onBrowserCustomStudyRescheduleChange\"");
    assert_contains(&html_main, "\"onBrowserRebuildFilteredDeck\"");
    assert_contains(&html_main, "\"onBrowserEmptyFilteredDeck\"");
    assert_contains(&html_main, "\"onBrowserToggleFilter\"");
    assert_contains(&html_main, "\"onBrowserSetFilter\"");
    assert_contains(&html_main, "\"onBrowserToggleFlagPicker\"");
    assert_contains(&html_main, "\"onBrowserSetFlagSelected\"");
    assert_contains(&html_main, "\"name\": \"index\"");
    assert_contains(&html_main, "\"onDeckOptionsBuryNewSiblingsChange\"");
    assert_contains(&html_main, "\"onDeckOptionsInitialEaseChange\"");
    assert_contains(&html_main, "\"onDeckOptionsDesiredRetentionChange\"");
    assert_contains(&html_main, "\"onDeckOptionsFsrsParametersChange\"");
    assert_contains(&html_main, "\"onDeckOptionsLeechActionChange\"");
    assert_contains(&html_main, "\"name\": \"checked\"");
    let html_host =
        fs::read_to_string(tmp.path().join("html").join("engram-host.mjs")).expect("html host");
    assert_contains(&html_host, "engram_engine.wasm");
    assert_contains(
        &html_host,
        "const HOST_READY_EVENT = \"mosaic-host-ready\";",
    );
    assert_contains(
        &html_host,
        "window.dispatchEvent(new CustomEvent(HOST_READY_EVENT));",
    );
    assert_contains(&html_host, "createEngramEngine");
    assert_contains(
        &html_host,
        "const SNAPSHOT_STORAGE_KEY = \"engram.snapshot.v1\";",
    );
    assert_contains(&html_host, "hydrateEngine(engine);");
    assert_contains(&html_host, "withSnapshotPersistence(host, engine)");
    assert_contains(&html_host, "persistSnapshot(engine);");
    assert_contains(&html_host, "handleHostIntent(engine, intent, result)");
    assert_contains(&html_host, "chooseAnkiImportFile(intent)");
    assert_contains(&html_host, "engine.mergeAnkiApkg(bytes)");
    assert_contains(&html_host, "engine.exportAnkiApkg()");
    assert_contains(&html_host, "downloadBytes(bytes, name)");

    let webcomponent_index = fs::read_to_string(tmp.path().join("webcomponent").join("index.html"))
        .expect("webcomponent/index.html");
    assert_contains(&webcomponent_index, "src=\"./engram-host.mjs\"");
    assert_contains(&webcomponent_index, "src=\"./EngramApp.js\"");
    assert_contains(&webcomponent_index, "src=\"./main.js\"");
    assert_contains(&webcomponent_index, "<mos-engram-app></mos-engram-app>");
    assert!(
        webcomponent_index
            .find("src=\"./engram-host.mjs\"")
            .expect("webcomponent host script")
            < webcomponent_index
                .find("src=\"./EngramApp.js\"")
                .expect("webcomponent component script"),
        "WebComponent host adapter should load before generated EngramApp.js"
    );
    assert!(
        webcomponent_index
            .find("src=\"./EngramApp.js\"")
            .expect("webcomponent component script")
            < webcomponent_index
                .find("src=\"./main.js\"")
                .expect("webcomponent main script"),
        "WebComponent runtime should load after generated EngramApp.js"
    );
    let webcomponent_main = fs::read_to_string(tmp.path().join("webcomponent").join("main.js"))
        .expect("webcomponent/main.js");
    assert_contains(&webcomponent_main, "const componentName = \"EngramApp\";");
    assert_contains(&webcomponent_main, "const customTag = \"mos-engram-app\";");
    assert_contains(
        &webcomponent_main,
        "window.mosaicHost?.getProps?.({ component: componentName })",
    );
    assert_contains(
        &webcomponent_main,
        "window.mosaicHost?.handleEvent?.({ component: componentName, event })",
    );
    assert_contains(
        &webcomponent_main,
        "root.setAttribute(slot.name, serializeSlotValue(value, slot.type));",
    );
    assert_contains(
        &webcomponent_main,
        "root.addEventListener(`mosaic:${eventName}`",
    );
    assert_contains(
        &webcomponent_main,
        "{ name: \"deck-names\", prop: \"deckNames\", type: \"list\", fallback: [] }",
    );
    assert_contains(&webcomponent_main, "\"selectDeck\"");
    assert_contains(
        &webcomponent_main,
        "{ name: \"browser-results\", prop: \"browserResults\", type: \"list\", fallback: [] }",
    );
    assert_contains(
        &webcomponent_main,
        "{ name: \"browser-custom-study-limit-value\", prop: \"browserCustomStudyLimitValue\", type: \"number\", fallback: 0 }",
    );
    assert_contains(
        &webcomponent_main,
        "{ name: \"browser-custom-study-reschedule-value\", prop: \"browserCustomStudyRescheduleValue\", type: \"bool\", fallback: false }",
    );
    assert_contains(
        &webcomponent_main,
        "{ name: \"answer-visible\", prop: \"answerVisible\", type: \"bool\", fallback: false }",
    );
    assert_contains(
        &webcomponent_main,
        "new CustomEvent(MOSAIC_HOST_INTENT_EVENT",
    );
    let webcomponent_host =
        fs::read_to_string(tmp.path().join("webcomponent").join("engram-host.mjs"))
            .expect("webcomponent host");
    assert_contains(&webcomponent_host, "engram_engine.wasm");
    assert_contains(
        &webcomponent_host,
        "const HOST_READY_EVENT = \"mosaic-host-ready\";",
    );
    assert_contains(
        &webcomponent_host,
        "window.dispatchEvent(new CustomEvent(HOST_READY_EVENT));",
    );
    assert_contains(&webcomponent_host, "createEngramEngine");
    assert_contains(
        &webcomponent_host,
        "const SNAPSHOT_STORAGE_KEY = \"engram.snapshot.v1\";",
    );
    assert_contains(&webcomponent_host, "hydrateEngine(engine);");
    assert_contains(&webcomponent_host, "withSnapshotPersistence(host, engine)");
    assert_contains(&webcomponent_host, "persistSnapshot(engine);");
    assert_contains(
        &webcomponent_host,
        "handleHostIntent(engine, intent, result)",
    );
    assert_contains(&webcomponent_host, "chooseAnkiImportFile(intent)");
    assert_contains(&webcomponent_host, "engine.mergeAnkiApkg(bytes)");
    assert_contains(&webcomponent_host, "engine.exportAnkiApkg()");
    assert_contains(&webcomponent_host, "downloadBytes(bytes, name)");

    let react_app = fs::read_to_string(tmp.path().join("react").join("src").join("main.tsx"))
        .expect("react/src/main.tsx");
    assert_contains(&react_app, "import \"./engram-host\";");
    assert_contains(&react_app, "const fallbackProps = {");
    assert_contains(&react_app, "appTitle: \"Sample AppTitle\",");
    assert_contains(&react_app, "deckListLabel: \"Sample DeckListLabel\",");
    assert_contains(&react_app, "deckNames: [],");
    assert_contains(
        &react_app,
        "deckOptionsSettingsLabel: \"Sample DeckOptionsSettingsLabel\",",
    );
    assert_contains(
        &react_app,
        "deckOptionsLearningStepsValue: \"Sample DeckOptionsLearningStepsValue\",",
    );
    assert_contains(&react_app, "deckOptionsNewCardsValue: 0,");
    assert_contains(&react_app, "deckOptionsIntervalModifierValue: 0,");
    assert_contains(&react_app, "deckOptionsInitialEaseValue: 0,");
    assert_contains(&react_app, "deckOptionsDesiredRetentionValue: 0,");
    assert_contains(
        &react_app,
        "deckOptionsFsrsParametersValue: \"Sample DeckOptionsFsrsParametersValue\",",
    );
    assert_contains(&react_app, "deckOptionsLeechThresholdValue: 0,");
    assert_contains(&react_app, "deckOptionsLeechActionSuspendValue: false,");
    assert_contains(&react_app, "deckOptionsBuryNewSiblingsValue: false,");
    assert_contains(&react_app, "deckOptionsBuryReviewSiblingsValue: false,");
    assert_contains(&react_app, "historyLabel: \"Sample HistoryLabel\",");
    assert_contains(
        &react_app,
        "historyTotalValue: \"Sample HistoryTotalValue\",",
    );
    assert_contains(
        &react_app,
        "historyAccuracyValue: \"Sample HistoryAccuracyValue\",",
    );
    assert_contains(&react_app, "collectionLabel: \"Sample CollectionLabel\",");
    assert_contains(
        &react_app,
        "collectionNoteCountValue: \"Sample CollectionNoteCountValue\",",
    );
    assert_contains(&react_app, "browserQuery: \"Sample BrowserQuery\",");
    assert_contains(
        &react_app,
        "browserFilterValue: \"Sample BrowserFilterValue\",",
    );
    assert_contains(&react_app, "browserFilterOptions: [],");
    assert_contains(&react_app, "browserFilterOpen: false,");
    assert_contains(&react_app, "browserFlagValue: \"Sample BrowserFlagValue\",");
    assert_contains(&react_app, "browserFlagOptions: [],");
    assert_contains(&react_app, "browserFlagOpen: false,");
    assert_contains(&react_app, "browserTagEdit: \"Sample BrowserTagEdit\",");
    assert_contains(
        &react_app,
        "browserAddTagLabel: \"Sample BrowserAddTagLabel\",",
    );
    assert_contains(&react_app, "browserCustomStudyLimitValue: 0,");
    assert_contains(&react_app, "browserCustomStudyRescheduleValue: false,");
    assert_contains(
        &react_app,
        "browserCustomStudyRebuildLabel: \"Sample BrowserCustomStudyRebuildLabel\",",
    );
    assert_contains(&react_app, "browserResults: [],");
    assert_contains(&react_app, "browserResultCardIds: [],");
    assert_contains(
        &react_app,
        "browserSelectedCardId: \"Sample BrowserSelectedCardId\",",
    );
    assert_contains(&react_app, "answerVisible: false,");
    assert_contains(&react_app, "typeAnswerActive: false,");
    assert_contains(&react_app, "typeAnswerValue: \"Sample TypeAnswerValue\",");
    assert_contains(&react_app, "actionUndoLabel: \"Sample ActionUndoLabel\",");
    assert_contains(&react_app, "actionMarkLabel: \"Sample ActionMarkLabel\",");
    assert_contains(
        &react_app,
        "window.mosaicHost?.getProps?.({ component: \"EngramApp\" })",
    );
    assert_contains(
        &react_app,
        "window.mosaicHost?.handleEvent?.({ component: \"EngramApp\", event })",
    );
    assert_contains(
        &react_app,
        "return <EngramApp {...props} dispatch={dispatch} />;",
    );
    assert!(
        !react_app.contains("dispatch={(ev) => console.log(\"event:\", ev)}"),
        "react shell should route events through window.mosaicHost"
    );
    let react_host =
        fs::read_to_string(tmp.path().join("react").join("src").join("engram-host.ts"))
            .expect("react host");
    assert_contains(&react_host, "createEngramEngine");
    assert_contains(&react_host, "engram_engine.wasm");
    assert_contains(
        &react_host,
        "const HOST_READY_EVENT = \"mosaic-host-ready\";",
    );
    assert_contains(
        &react_host,
        "window.dispatchEvent(new CustomEvent(HOST_READY_EVENT));",
    );
    assert_contains(
        &react_host,
        "const SNAPSHOT_STORAGE_KEY = \"engram.snapshot.v1\";",
    );
    assert_contains(&react_host, "hydrateEngine(engine);");
    assert_contains(&react_host, "withSnapshotPersistence(host, engine)");
    assert_contains(&react_host, "persistSnapshot(engine);");
    assert_contains(&react_host, "handleHostIntent(engine, intent, result)");
    assert_contains(&react_host, "chooseAnkiImportFile(intent)");
    assert_contains(&react_host, "engine.mergeAnkiApkg(bytes)");
    assert_contains(&react_host, "engine.exportAnkiApkg()");
    assert_contains(&react_host, "downloadBytes(bytes, name)");
    assert!(
        tmp.path()
            .join("react")
            .join("src")
            .join("engram-mosaic-host-wasm.d.ts")
            .exists(),
        "react host wasm declarations should be installed from manifest assets"
    );

    let electron_app = fs::read_to_string(tmp.path().join("electron").join("src").join("main.tsx"))
        .expect("electron/src/main.tsx");
    assert_contains(&electron_app, "const fallbackProps = {");
    assert_contains(&electron_app, "appTitle: \"Sample AppTitle\",");
    assert_contains(&electron_app, "deckListLabel: \"Sample DeckListLabel\",");
    assert_contains(&electron_app, "deckNames: [],");
    assert_contains(
        &electron_app,
        "deckOptionsSettingsLabel: \"Sample DeckOptionsSettingsLabel\",",
    );
    assert_contains(
        &electron_app,
        "deckOptionsRelearningStepsValue: \"Sample DeckOptionsRelearningStepsValue\",",
    );
    assert_contains(&electron_app, "deckOptionsMaximumIntervalValue: 0,");
    assert_contains(&electron_app, "deckOptionsInitialEaseValue: 0,");
    assert_contains(&electron_app, "deckOptionsEasyBonusValue: 0,");
    assert_contains(
        &electron_app,
        "deckOptionsBuryInterdayLearningSiblingsValue: false,",
    );
    assert_contains(
        &electron_app,
        "historyWindowLabel: \"Sample HistoryWindowLabel\",",
    );
    assert_contains(
        &electron_app,
        "historyAgainValue: \"Sample HistoryAgainValue\",",
    );
    assert_contains(
        &electron_app,
        "collectionImportLabel: \"Sample CollectionImportLabel\",",
    );
    assert_contains(
        &electron_app,
        "collectionDeleteNoteTypeLabel: \"Sample CollectionDeleteNoteTypeLabel\",",
    );
    assert_contains(&electron_app, "browserQuery: \"Sample BrowserQuery\",");
    assert_contains(
        &electron_app,
        "browserFlagValue: \"Sample BrowserFlagValue\",",
    );
    assert_contains(&electron_app, "browserFlagOptions: [],");
    assert_contains(&electron_app, "browserFlagOpen: false,");
    assert_contains(&electron_app, "browserTagEdit: \"Sample BrowserTagEdit\",");
    assert_contains(
        &electron_app,
        "browserRemoveTagLabel: \"Sample BrowserRemoveTagLabel\",",
    );
    assert_contains(&electron_app, "browserCustomStudyLimitValue: 0,");
    assert_contains(&electron_app, "browserCustomStudyRescheduleValue: false,");
    assert_contains(&electron_app, "browserResults: [],");
    assert_contains(&electron_app, "browserResultCardIds: [],");
    assert_contains(
        &electron_app,
        "browserSelectedCardId: \"Sample BrowserSelectedCardId\",",
    );
    assert_contains(&electron_app, "answerVisible: false,");
    assert_contains(&electron_app, "typeAnswerActive: false,");
    assert_contains(
        &electron_app,
        "typeAnswerValue: \"Sample TypeAnswerValue\",",
    );
    assert_contains(
        &electron_app,
        "actionUndoLabel: \"Sample ActionUndoLabel\",",
    );
    assert_contains(
        &electron_app,
        "actionMarkLabel: \"Sample ActionMarkLabel\",",
    );
    assert_contains(
        &electron_app,
        "window.mosaicHost?.getProps?.({ component: \"EngramApp\" })",
    );
    assert_contains(
        &electron_app,
        "window.mosaicHost?.handleEvent?.({ component: \"EngramApp\", event })",
    );
    assert_contains(
        &electron_app,
        "return <EngramApp {...props} dispatch={dispatch} />;",
    );
    assert!(
        !electron_app.contains("dispatch={(ev) => console.log(\"event:\", ev)}"),
        "electron renderer shell should route events through window.mosaicHost"
    );
    let electron_main =
        fs::read_to_string(tmp.path().join("electron").join("electron").join("main.ts"))
            .expect("electron/electron/main.ts");
    assert_contains(&electron_main, "new BrowserWindow");
    assert_contains(&electron_main, "MOSAIC_ELECTRON_DEV_SERVER_URL");
    assert_contains(&electron_main, "EngramApp");
    assert_contains(&electron_main, "import { app, BrowserWindow, ipcMain }");
    assert_contains(&electron_main, "import { existsSync } from \"node:fs\"");
    assert_contains(&electron_main, "pathToFileURL");
    assert_contains(&electron_main, "MOSAIC_ELECTRON_HOST_MODULE");
    assert_contains(&electron_main, "async function loadMosaicHost()");
    assert_contains(&electron_main, "let mosaicHost: MosaicHost = {}");
    assert_contains(&electron_main, "ipcMain.handle(");
    assert_contains(&electron_main, "mosaic:get-props");
    assert_contains(&electron_main, "mosaic:handle-event");
    assert_contains(&electron_main, "mosaicHost.getProps?.(request)");
    assert_contains(&electron_main, "mosaicHost.handleEvent?.(request)");
    assert!(
        !electron_main.contains("=> undefined"),
        "electron main shell should delegate IPC to a host module when one is installed"
    );
    let electron_preload = fs::read_to_string(
        tmp.path()
            .join("electron")
            .join("electron")
            .join("preload.ts"),
    )
    .expect("electron/electron/preload.ts");
    assert_contains(&electron_preload, "import { contextBridge, ipcRenderer }");
    assert_contains(
        &electron_preload,
        "contextBridge.exposeInMainWorld(\"mosaicHost\"",
    );
    assert_contains(&electron_preload, "getProps: (request: MosaicHostRequest)");
    assert_contains(
        &electron_preload,
        "handleEvent: (request: MosaicHostRequest)",
    );
    let electron_host =
        fs::read_to_string(tmp.path().join("electron").join("electron").join("host.js"))
            .expect("electron host.js");
    assert_contains(&electron_host, "createMosaicHost");
    assert_contains(&electron_host, "createEngramEngine");
    assert_contains(&electron_host, "ENGRAM_SNAPSHOT_PATH");
    assert_contains(&electron_host, "mosaic-snapshot.v1.json");
    assert_contains(&electron_host, "hydrateEngine(engine);");
    assert_contains(&electron_host, "withSnapshotPersistence(host, engine)");
    assert_contains(&electron_host, "persistSnapshot(engine);");
    assert_contains(&electron_host, "spawnSync");
    assert_contains(&electron_host, "dialog");
    assert_contains(&electron_host, "ENGRAM_HOST_CLI");
    assert_contains(&electron_host, "engram-host-cli.exe");
    assert_contains(&electron_host, "handleHostIntent");
    assert_contains(&electron_host, "importAnkiPackage");
    assert_contains(&electron_host, "exportAnkiPackage");
    assert_contains(&electron_host, "runSidecar");
    assert_contains(&electron_host, "merge-apkg");
    assert_contains(&electron_host, "export-apkg");
    assert_contains(&electron_host, "loadPersistedSnapshot");

    let flutter_app = fs::read_to_string(tmp.path().join("flutter").join("lib").join("main.dart"))
        .expect("flutter/lib/main.dart");
    assert_contains(&flutter_app, "EngramApp(");
    assert_contains(&flutter_app, "import 'mosaic_host.dart';");
    assert_contains(&flutter_app, "MosaicHost.load()");
    assert_contains(&flutter_app, "_queueMosaicResponse(_mosaicHost?.props())");
    assert_contains(
        &flutter_app,
        "appTitle: mosaicString(_hostProps, \"app-title\", \"Sample AppTitle\"),",
    );
    assert_contains(
        &flutter_app,
        "deckListLabel: mosaicString(_hostProps, \"deck-list-label\", \"Sample DeckListLabel\"),",
    );
    assert_contains(
        &flutter_app,
        "deckNames: mosaicStringList(_hostProps, \"deck-names\"),",
    );
    assert_contains(
        &flutter_app,
        "deckOptionsSettingsLabel: mosaicString(_hostProps, \"deck-options-settings-label\", \"Sample DeckOptionsSettingsLabel\"),",
    );
    assert_contains(
        &flutter_app,
        "deckOptionsLearningStepsValue: mosaicString(_hostProps, \"deck-options-learning-steps-value\", \"Sample DeckOptionsLearningStepsValue\"),",
    );
    assert_contains(
        &flutter_app,
        "deckOptionsNewCardsValue: mosaicDouble(_hostProps, \"deck-options-new-cards-value\", 0.0),",
    );
    assert_contains(
        &flutter_app,
        "deckOptionsHardMultiplierValue: mosaicDouble(_hostProps, \"deck-options-hard-multiplier-value\", 0.0),",
    );
    assert_contains(
        &flutter_app,
        "deckOptionsInitialEaseValue: mosaicDouble(_hostProps, \"deck-options-initial-ease-value\", 0.0),",
    );
    assert_contains(
        &flutter_app,
        "deckOptionsBuryNewSiblingsValue: mosaicBoolean(_hostProps, \"deck-options-bury-new-siblings-value\", false),",
    );
    assert_contains(
        &flutter_app,
        "historyLabel: mosaicString(_hostProps, \"history-label\", \"Sample HistoryLabel\"),",
    );
    assert_contains(
        &flutter_app,
        "historyCorrectValue: mosaicString(_hostProps, \"history-correct-value\", \"Sample HistoryCorrectValue\"),",
    );
    assert_contains(
        &flutter_app,
        "collectionLabel: mosaicString(_hostProps, \"collection-label\", \"Sample CollectionLabel\"),",
    );
    assert_contains(
        &flutter_app,
        "collectionExportLabel: mosaicString(_hostProps, \"collection-export-label\", \"Sample CollectionExportLabel\"),",
    );
    assert_contains(
        &flutter_app,
        "browserQuery: mosaicString(_hostProps, \"browser-query\", \"Sample BrowserQuery\"),",
    );
    assert_contains(
        &flutter_app,
        "browserFlagValue: mosaicString(_hostProps, \"browser-flag-value\", \"Sample BrowserFlagValue\"),",
    );
    assert_contains(
        &flutter_app,
        "browserFlagOptions: mosaicStringList(_hostProps, \"browser-flag-options\"),",
    );
    assert_contains(
        &flutter_app,
        "browserFlagOpen: mosaicBoolean(_hostProps, \"browser-flag-open\", false),",
    );
    assert_contains(
        &flutter_app,
        "browserTagEdit: mosaicString(_hostProps, \"browser-tag-edit\", \"Sample BrowserTagEdit\"),",
    );
    assert_contains(
        &flutter_app,
        "browserAddTagLabel: mosaicString(_hostProps, \"browser-add-tag-label\", \"Sample BrowserAddTagLabel\"),",
    );
    assert_contains(
        &flutter_app,
        "browserCustomStudyLimitValue: mosaicDouble(_hostProps, \"browser-custom-study-limit-value\", 0.0),",
    );
    assert_contains(
        &flutter_app,
        "browserCustomStudyRescheduleValue: mosaicBoolean(_hostProps, \"browser-custom-study-reschedule-value\", false),",
    );
    assert_contains(
        &flutter_app,
        "browserResults: mosaicStringList(_hostProps, \"browser-results\"),",
    );
    assert_contains(
        &flutter_app,
        "browserResultCardIds: mosaicStringList(_hostProps, \"browser-result-card-ids\"),",
    );
    assert_contains(
        &flutter_app,
        "browserSelectedCardId: mosaicString(_hostProps, \"browser-selected-card-id\", \"Sample BrowserSelectedCardId\"),",
    );
    assert_contains(
        &flutter_app,
        "answerVisible: mosaicBoolean(_hostProps, \"answer-visible\", false),",
    );
    assert_contains(
        &flutter_app,
        "typeAnswerActive: mosaicBoolean(_hostProps, \"type-answer-active\", false),",
    );
    assert_contains(
        &flutter_app,
        "typeAnswerValue: mosaicString(_hostProps, \"type-answer-value\", \"Sample TypeAnswerValue\"),",
    );
    assert_contains(
        &flutter_app,
        "actionUndoLabel: mosaicString(_hostProps, \"action-undo-label\", \"Sample ActionUndoLabel\"),",
    );
    assert_contains(
        &flutter_app,
        "actionMarkLabel: mosaicString(_hostProps, \"action-mark-label\", \"Sample ActionMarkLabel\"),",
    );
    assert_contains(
        &flutter_app,
        "final response = _mosaicHost?.handleEvent(event.mosaicEnvelope);",
    );
    assert_contains(
        &flutter_app,
        "debugPrint(\"event: ${event.mosaicEnvelope}\");",
    );
    assert_contains(&flutter_app, "_queueMosaicResponse(response);");

    let flutter_host = fs::read_to_string(
        tmp.path()
            .join("flutter")
            .join("lib")
            .join("mosaic_host.dart"),
    )
    .expect("flutter/lib/mosaic_host.dart");
    assert_contains(&flutter_host, "class MosaicHost");
    assert_contains(&flutter_host, "DynamicLibrary.open");
    assert_contains(&flutter_host, "package:file_selector/file_selector.dart");
    assert_contains(&flutter_host, "eg_engram_app_props");
    assert_contains(&flutter_host, "eg_handle_engram_app_event");
    assert_contains(&flutter_host, "eg_export_anki_apkg");
    assert_contains(&flutter_host, "eg_merge_anki_apkg");
    assert_contains(&flutter_host, "jsonEncode(event)");
    assert_contains(&flutter_host, "openFile");
    assert_contains(&flutter_host, "getSaveLocation");
    assert_contains(&flutter_host, "'props': _mosaicMap(decoded['props'])");

    let compose_app =
        fs::read_to_string(tmp.path().join("compose").join("EngramApp.kt")).expect("EngramApp.kt");
    assert_contains(&compose_app, "sealed class EngramAppEvent {");
    assert_contains(&compose_app, "abstract val mosaicName: String");
    assert_contains(&compose_app, "val mosaicEnvelope: Map<String, Any?>");
    assert_contains(&compose_app, "data object ImportAnki : EngramAppEvent()");
    assert_contains(
        &compose_app,
        "data object PruneUnusedMedia : EngramAppEvent()",
    );
    assert_contains(
        &compose_app,
        "override val mosaicName: String = \"onImportAnki\"",
    );
    assert_contains(&compose_app, "data class SelectDeck(val value: String)");
    assert_contains(
        &compose_app,
        "data class DeckOptionsNewCardsChange(val value: Double)",
    );
    assert_contains(
        &compose_app,
        "data class DeckOptionsInitialEaseChange(val value: Double)",
    );
    assert_contains(
        &compose_app,
        "data class DeckOptionsDesiredRetentionChange(val value: Double)",
    );
    assert_contains(
        &compose_app,
        "data class DeckOptionsFsrsParametersChange(val value: String)",
    );
    assert_contains(
        &compose_app,
        "override val mosaicPayload: Map<String, Any?> get() = mapOf(\"value\" to value)",
    );
    assert_contains(
        &compose_app,
        "data class DeckOptionsBuryNewSiblingsChange(val checked: Boolean)",
    );
    assert_contains(
        &compose_app,
        "override val mosaicPayload: Map<String, Any?> get() = mapOf(\"checked\" to checked)",
    );
    assert_contains(
        &compose_app,
        "data class BrowserTagEditChange(val value: String)",
    );
    assert_contains(&compose_app, "data object BrowserToggleFilter");
    assert_contains(
        &compose_app,
        "data class BrowserSetFilter(val value: String)",
    );
    assert_contains(&compose_app, "data object BrowserToggleFlagPicker");
    assert_contains(
        &compose_app,
        "data class BrowserSetFlagSelected(val value: String)",
    );
    assert_contains(
        &compose_app,
        "data class TypeAnswerChange(val value: String)",
    );
    assert_contains(&compose_app, "data object BrowserAddTagSelected");
    assert_contains(&compose_app, "data object BrowserRemoveTagSelected");
    assert_contains(
        &compose_app,
        "data class BrowserCustomStudyLimitChange(val value: Double)",
    );
    assert_contains(
        &compose_app,
        "data class BrowserCustomStudyRescheduleChange(val checked: Boolean)",
    );
    assert_contains(&compose_app, "data object BrowserRebuildFilteredDeck");
    assert_contains(&compose_app, "data object BrowserEmptyFilteredDeck");
    assert_contains(&compose_app, "@Composable");
    assert_contains(&compose_app, "fun EngramApp(");
    assert_contains(&compose_app, "appTitle: String,");
    assert_contains(&compose_app, "deckNames: List<String>,");
    assert_contains(&compose_app, "deckOptionsSettingsLabel: String,");
    assert_contains(&compose_app, "deckOptionsLearningStepsValue: String,");
    assert_contains(&compose_app, "deckOptionsNewCardsValue: Double,");
    assert_contains(&compose_app, "deckOptionsEasyBonusValue: Double,");
    assert_contains(&compose_app, "deckOptionsInitialEaseValue: Double,");
    assert_contains(&compose_app, "deckOptionsDesiredRetentionValue: Double,");
    assert_contains(&compose_app, "deckOptionsFsrsParametersValue: String,");
    assert_contains(&compose_app, "deckOptionsBuryNewSiblingsValue: Boolean,");
    assert_contains(&compose_app, "historyLabel: String,");
    assert_contains(&compose_app, "collectionLabel: String,");
    assert_contains(&compose_app, "browserQuery: String,");
    assert_contains(&compose_app, "browserFlagValue: String,");
    assert_contains(&compose_app, "browserFlagOptions: List<String>,");
    assert_contains(&compose_app, "browserFlagOpen: Boolean,");
    assert_contains(&compose_app, "browserTagEdit: String,");
    assert_contains(&compose_app, "browserAddTagLabel: String,");
    assert_contains(&compose_app, "browserCustomStudyLimitValue: Double,");
    assert_contains(&compose_app, "browserCustomStudyRescheduleValue: Boolean,");
    assert_contains(&compose_app, "browserResults: List<String>,");
    assert_contains(&compose_app, "browserResultCardIds: List<String>,");
    assert_contains(&compose_app, "browserSelectedCardId: String,");
    assert_contains(&compose_app, "answerVisible: Boolean,");
    assert_contains(&compose_app, "typeAnswerActive: Boolean,");
    assert_contains(&compose_app, "typeAnswerValue: String,");
    assert_contains(&compose_app, "actionUndoLabel: String,");
    assert_contains(&compose_app, "actionMarkLabel: String,");
    assert_contains(&compose_app, "dispatch: (EngramAppEvent) -> Unit,");
    let compose_main = fs::read_to_string(
        tmp.path()
            .join("compose")
            .join("src")
            .join("main")
            .join("kotlin")
            .join("Main.kt"),
    )
    .expect("compose/src/main/kotlin/Main.kt");
    assert_contains(&compose_main, "fun main() = application");
    assert_contains(
        &compose_main,
        "Window(onCloseRequest = ::exitApplication, title = \"EngramApp\")",
    );
    assert_contains(&compose_main, "MosaicComposeHostBridge.load()");
    assert_contains(&compose_main, "var hostProps by remember");
    assert_contains(&compose_main, "applyMosaicResponse(mosaicHost?.props())");
    assert_contains(&compose_main, "EngramApp(");
    assert_contains(
        &compose_main,
        "appTitle = mosaicString(hostProps, \"app-title\", \"Sample AppTitle\"),",
    );
    assert_contains(
        &compose_main,
        "deckNames = mosaicStringList(hostProps, \"deck-names\"),",
    );
    assert_contains(
        &compose_main,
        "deckOptionsSettingsLabel = mosaicString(hostProps, \"deck-options-settings-label\", \"Sample DeckOptionsSettingsLabel\"),",
    );
    assert_contains(
        &compose_main,
        "deckOptionsLearningStepsValue = mosaicString(hostProps, \"deck-options-learning-steps-value\", \"Sample DeckOptionsLearningStepsValue\"),",
    );
    assert_contains(
        &compose_main,
        "deckOptionsNewCardsValue = mosaicDouble(hostProps, \"deck-options-new-cards-value\", 0.0),",
    );
    assert_contains(
        &compose_main,
        "deckOptionsEasyBonusValue = mosaicDouble(hostProps, \"deck-options-easy-bonus-value\", 0.0),",
    );
    assert_contains(
        &compose_main,
        "deckOptionsInitialEaseValue = mosaicDouble(hostProps, \"deck-options-initial-ease-value\", 0.0),",
    );
    assert_contains(
        &compose_main,
        "deckOptionsDesiredRetentionValue = mosaicDouble(hostProps, \"deck-options-desired-retention-value\", 0.0),",
    );
    assert_contains(
        &compose_main,
        "deckOptionsFsrsParametersValue = mosaicString(hostProps, \"deck-options-fsrs-parameters-value\", \"Sample DeckOptionsFsrsParametersValue\"),",
    );
    assert_contains(
        &compose_main,
        "deckOptionsBuryNewSiblingsValue = mosaicBoolean(hostProps, \"deck-options-bury-new-siblings-value\", false),",
    );
    assert_contains(
        &compose_main,
        "historyLabel = mosaicString(hostProps, \"history-label\", \"Sample HistoryLabel\"),",
    );
    assert_contains(
        &compose_main,
        "collectionLabel = mosaicString(hostProps, \"collection-label\", \"Sample CollectionLabel\"),",
    );
    assert_contains(
        &compose_main,
        "browserQuery = mosaicString(hostProps, \"browser-query\", \"Sample BrowserQuery\"),",
    );
    assert_contains(
        &compose_main,
        "browserFlagValue = mosaicString(hostProps, \"browser-flag-value\", \"Sample BrowserFlagValue\"),",
    );
    assert_contains(
        &compose_main,
        "browserFlagOptions = mosaicStringList(hostProps, \"browser-flag-options\"),",
    );
    assert_contains(
        &compose_main,
        "browserFlagOpen = mosaicBoolean(hostProps, \"browser-flag-open\", false),",
    );
    assert_contains(
        &compose_main,
        "browserTagEdit = mosaicString(hostProps, \"browser-tag-edit\", \"Sample BrowserTagEdit\"),",
    );
    assert_contains(
        &compose_main,
        "browserRemoveTagLabel = mosaicString(hostProps, \"browser-remove-tag-label\", \"Sample BrowserRemoveTagLabel\"),",
    );
    assert_contains(
        &compose_main,
        "browserCustomStudyLimitValue = mosaicDouble(hostProps, \"browser-custom-study-limit-value\", 0.0),",
    );
    assert_contains(
        &compose_main,
        "browserCustomStudyRescheduleValue = mosaicBoolean(hostProps, \"browser-custom-study-reschedule-value\", false),",
    );
    assert_contains(
        &compose_main,
        "browserResults = mosaicStringList(hostProps, \"browser-results\"),",
    );
    assert_contains(
        &compose_main,
        "browserResultCardIds = mosaicStringList(hostProps, \"browser-result-card-ids\"),",
    );
    assert_contains(
        &compose_main,
        "browserSelectedCardId = mosaicString(hostProps, \"browser-selected-card-id\", \"Sample BrowserSelectedCardId\"),",
    );
    assert_contains(
        &compose_main,
        "answerVisible = mosaicBoolean(hostProps, \"answer-visible\", false),",
    );
    assert_contains(
        &compose_main,
        "typeAnswerActive = mosaicBoolean(hostProps, \"type-answer-active\", false),",
    );
    assert_contains(
        &compose_main,
        "typeAnswerValue = mosaicString(hostProps, \"type-answer-value\", \"Sample TypeAnswerValue\"),",
    );
    assert_contains(
        &compose_main,
        "actionUndoLabel = mosaicString(hostProps, \"action-undo-label\", \"Sample ActionUndoLabel\"),",
    );
    assert_contains(
        &compose_main,
        "actionMarkLabel = mosaicString(hostProps, \"action-mark-label\", \"Sample ActionMarkLabel\"),",
    );
    assert_contains(
        &compose_main,
        "mosaicHost?.handleEvent(event.mosaicEnvelope)",
    );
    assert_contains(&compose_main, "applyMosaicResponse(response)");
    assert_contains(&compose_main, "Class.forName(\"MosaicHost\")");
    let compose_gradle = fs::read_to_string(tmp.path().join("compose").join("build.gradle.kts"))
        .expect("compose/build.gradle.kts");
    assert_contains(
        &compose_gradle,
        "id(\"org.jetbrains.compose\") version \"1.11.1\"",
    );
    assert_contains(&compose_gradle, "mainClass = \"MainKt\"");
    let compose_host = fs::read_to_string(
        tmp.path()
            .join("compose")
            .join("src")
            .join("main")
            .join("kotlin")
            .join("MosaicHost.kt"),
    )
    .expect("compose host");
    assert_contains(&compose_host, "class MosaicHost");
    assert_contains(&compose_host, "eg_engram_app_props");
    assert_contains(&compose_host, "eg_handle_engram_app_event");

    let qml =
        fs::read_to_string(tmp.path().join("qt").join("EngramApp.qml")).expect("EngramApp.qml");
    assert_contains(&qml, "property string appTitle");
    assert_contains(&qml, "property var deckNames");
    assert_contains(&qml, "property string deckOptionsSettingsLabel");
    assert_contains(&qml, "property string deckOptionsLearningStepsValue");
    assert_contains(&qml, "property string deckOptionsRelearningStepsValue");
    assert_contains(&qml, "property real deckOptionsNewCardsValue");
    assert_contains(&qml, "property real deckOptionsEasyBonusValue");
    assert_contains(&qml, "property real deckOptionsInitialEaseValue");
    assert_contains(&qml, "property bool deckOptionsBuryNewSiblingsValue");
    assert_contains(&qml, "property bool deckOptionsBuryReviewSiblingsValue");
    assert_contains(
        &qml,
        "property bool deckOptionsBuryInterdayLearningSiblingsValue",
    );
    assert_contains(&qml, "property string historyLabel");
    assert_contains(&qml, "property string historyTotalValue");
    assert_contains(&qml, "property string historyAccuracyValue");
    assert_contains(&qml, "property string collectionLabel");
    assert_contains(&qml, "property string collectionNoteCountValue");
    assert_contains(&qml, "property string browserQuery");
    assert_contains(&qml, "property string browserFlagValue");
    assert_contains(&qml, "property var browserFlagOptions");
    assert_contains(&qml, "property bool browserFlagOpen");
    assert_contains(&qml, "property string browserTagEdit");
    assert_contains(&qml, "property string browserAddTagLabel");
    assert_contains(&qml, "property var browserResults");
    assert_contains(&qml, "property var browserResultCardIds");
    assert_contains(&qml, "property string browserSelectedCardId");
    assert_contains(&qml, "property bool answerVisible");
    assert_contains(&qml, "property bool typeAnswerActive");
    assert_contains(&qml, "property string typeAnswerValue");
    assert_contains(&qml, "signal reveal()");
    assert_contains(&qml, "signal selectDeck(string value)");
    assert_contains(&qml, "signal typeAnswerChange(string value)");
    assert_contains(&qml, "signal again()");
    assert_contains(&qml, "signal hard()");
    assert_contains(&qml, "signal good()");
    assert_contains(&qml, "signal easy()");
    assert_contains(&qml, "signal undo()");
    assert_contains(&qml, "signal buryCard()");
    assert_contains(&qml, "signal burySiblings()");
    assert_contains(&qml, "signal suspendCard()");
    assert_contains(&qml, "signal toggleMark()");
    assert_contains(&qml, "signal importAnki()");
    assert_contains(&qml, "signal pruneUnusedMedia()");
    assert_contains(&qml, "signal mosaicEvent(var event)");
    assert_contains(&qml, "id: mosaicRoot");
    assert_contains(&qml, "property var mosaicHost: null");
    assert_contains(&qml, "property var lastHostIntent: null");
    assert_contains(&qml, "function applyMosaicProps(props)");
    assert_contains(&qml, "function applyMosaicResponse(response)");
    assert_contains(&qml, "lastHostIntent = response.hostIntent;");
    assert_contains(&qml, "mosaicRoot[key] = props[key];");
    assert_contains(
        &qml,
        "onMosaicEvent: applyMosaicResponse(mosaicHost ? mosaicHost.handleEvent(event) : null)",
    );
    assert_contains(
        &qml,
        "onImportAnki: mosaicEvent({ \"event\": \"onImportAnki\" })",
    );
    assert_contains(
        &qml,
        "onPruneUnusedMedia: mosaicEvent({ \"event\": \"onPruneUnusedMedia\" })",
    );
    assert_contains(
        &qml,
        "onSelectDeck: mosaicEvent({ \"event\": \"onSelectDeck\", \"value\": value })",
    );
    assert_contains(&qml, "signal exportAnki()");
    assert_contains(&qml, "signal addNote()");
    assert_contains(&qml, "signal addNoteType()");
    assert_contains(&qml, "signal deleteNote()");
    assert_contains(&qml, "signal deleteNoteType()");
    assert_contains(&qml, "signal noteEditorSelectNoteType(real index)");
    assert_contains(&qml, "signal noteEditorSelectDeck(real index)");
    assert_contains(&qml, "property int noteTypeIndex: index");
    assert_contains(&qml, "property int deckIndex: index");
    assert_contains(&qml, "property int fieldIndex: index");
    assert_contains(&qml, "onClicked: noteEditorSelectNoteType(noteTypeIndex)");
    assert_contains(&qml, "onClicked: noteEditorSelectDeck(deckIndex)");
    assert_contains(&qml, "onClicked: noteEditorSelectField(fieldIndex)");
    assert_contains(
        &qml,
        "onClicked: noteTypeEditorSelectNoteType(noteTypeIndex)",
    );
    assert!(
        !qml.contains("invoking parameterless"),
        "Qt shell should preserve For-loop row payloads for generated selection events"
    );
    assert_contains(&qml, "signal deckOptionsLearningStepsChange(string value)");
    assert_contains(
        &qml,
        "signal deckOptionsRelearningStepsChange(string value)",
    );
    assert_contains(&qml, "signal deckOptionsNewCardsChange(real value)");
    assert_contains(&qml, "signal deckOptionsMaximumIntervalChange(real value)");
    assert_contains(&qml, "signal deckOptionsInitialEaseChange(real value)");
    assert_contains(&qml, "signal deckOptionsEasyBonusChange(real value)");
    assert_contains(&qml, "text: String(deckOptionsInitialEaseValue)");
    assert_contains(&qml, "placeholderText: \"2.5\"");
    assert_contains(&qml, "validator: DoubleValidator {");
    assert_contains(
        &qml,
        "if (text.length > 0 && !isNaN(nextValue)) { deckOptionsInitialEaseChange(nextValue) }",
    );
    assert_contains(
        &qml,
        "signal deckOptionsBuryNewSiblingsChange(bool checked)",
    );
    assert_contains(
        &qml,
        "signal deckOptionsBuryInterdayLearningSiblingsChange(bool checked)",
    );
    assert_contains(&qml, "signal browserSearch()");
    assert_contains(&qml, "signal browserSelectResult(real index)");
    assert_contains(&qml, "signal browserToggleFilter()");
    assert_contains(&qml, "signal browserSetFilter(string value)");
    assert_contains(&qml, "signal browserToggleFlagPicker()");
    assert_contains(&qml, "signal browserSetFlagSelected(string value)");
    assert_contains(&qml, "signal browserTagEditChange(string value)");
    assert_contains(&qml, "signal browserAddTagSelected()");
    assert_contains(&qml, "signal browserRemoveTagSelected()");
    let qt_main = fs::read_to_string(tmp.path().join("qt").join("main.cpp")).expect("qt/main.cpp");
    assert_contains(&qt_main, "#include <QApplication>");
    assert_contains(&qt_main, "QApplication app(argc, argv);");
    assert_contains(&qt_main, "#if __has_include(\"MosaicHost.h\")");
    assert_contains(&qt_main, "MosaicHost mosaicHost;");
    assert_contains(&qt_main, "root->setProperty(\"mosaicHost\"");
    assert_contains(
        &qt_main,
        "QMetaObject::invokeMethod(root, \"applyMosaicResponse\"",
    );
    let qt_cmake = fs::read_to_string(tmp.path().join("qt").join("CMakeLists.txt"))
        .expect("qt/CMakeLists.txt");
    assert_contains(
        &qt_cmake,
        "target_sources(EngramApp PRIVATE MosaicHost.cpp MosaicHost.h)",
    );
    assert_contains(
        &qt_cmake,
        "find_package(Qt6 6.7 REQUIRED COMPONENTS Quick QmlImportScanner Widgets)",
    );
    assert_contains(
        &qt_cmake,
        "target_link_libraries(EngramApp PRIVATE Qt6::Quick Qt6::Widgets)",
    );
    assert_contains(
        &qt_cmake,
        "foreach(_mosaic_native_library IN ITEMS engram_capi.dll libengram_capi.dylib libengram_capi.so)",
    );
    assert!(
        tmp.path().join("qt").join("MosaicHost.h").exists()
            && tmp.path().join("qt").join("MosaicHost.cpp").exists(),
        "qt host adapter files should be installed from manifest assets"
    );

    let swift = fs::read_to_string(tmp.path().join("swiftui").join("EngramApp.swift"))
        .expect("EngramApp.swift");
    assert_contains(&swift, "enum EngramAppEvent {");
    assert_contains(&swift, "case selectDeck");
    assert_contains(&swift, "case reveal");
    assert_contains(&swift, "case again");
    assert_contains(&swift, "case hard");
    assert_contains(&swift, "case good");
    assert_contains(&swift, "case easy");
    assert_contains(&swift, "case undo");
    assert_contains(&swift, "case buryCard");
    assert_contains(&swift, "case burySiblings");
    assert_contains(&swift, "case suspendCard");
    assert_contains(&swift, "case toggleMark");
    assert_contains(&swift, "case typeAnswerChange");
    assert_contains(&swift, "case deckOptionsLearningStepsChange");
    assert_contains(&swift, "case deckOptionsRelearningStepsChange");
    assert_contains(&swift, "case deckOptionsNewCardsChange");
    assert_contains(&swift, "case deckOptionsMaximumIntervalChange");
    assert_contains(&swift, "case deckOptionsInitialEaseChange");
    assert_contains(&swift, "case deckOptionsEasyBonusChange");
    assert_contains(&swift, "case deckOptionsBuryNewSiblingsChange");
    assert_contains(&swift, "case deckOptionsBuryReviewSiblingsChange");
    assert_contains(&swift, "case deckOptionsBuryInterdayLearningSiblingsChange");
    assert_contains(&swift, "case importAnki");
    assert_contains(&swift, "case exportAnki");
    assert_contains(&swift, "case addNote");
    assert_contains(&swift, "case addNoteType");
    assert_contains(&swift, "case deleteNote");
    assert_contains(&swift, "case deleteNoteType");
    assert_contains(&swift, "case browserSearch");
    assert_contains(&swift, "case browserSelectResult");
    assert_contains(&swift, "case browserToggleFilter");
    assert_contains(&swift, "case browserSetFilter");
    assert_contains(&swift, "case browserToggleFlagPicker");
    assert_contains(&swift, "case browserSetFlagSelected");
    assert_contains(&swift, "case browserTagEditChange");
    assert_contains(&swift, "case browserAddTagSelected");
    assert_contains(&swift, "case browserRemoveTagSelected");
    assert_contains(&swift, "case noteEditorSelectNoteType");
    assert_contains(&swift, "case noteEditorSelectDeck");
    assert_contains(&swift, "var mosaicName: String");
    assert_contains(&swift, "var mosaicEnvelope: [String: Any]");
    assert_contains(&swift, "struct EngramAppView: View");
    assert_contains(&swift, "let appTitle: String");
    assert_contains(&swift, "let deckNames: [String]");
    assert_contains(&swift, "let deckOptionsSettingsLabel: String");
    assert_contains(&swift, "let deckOptionsLearningStepsValue: String");
    assert_contains(&swift, "let deckOptionsRelearningStepsValue: String");
    assert_contains(&swift, "let deckOptionsNewCardsValue: Double");
    assert_contains(&swift, "let deckOptionsEasyBonusValue: Double");
    assert_contains(&swift, "let deckOptionsInitialEaseValue: Double");
    assert_contains(
        &swift,
        "TextField(\"20\", value: Binding(get: { deckOptionsNewCardsValue }, set: { dispatch(.deckOptionsNewCardsChange(value: $0)) }), format: .number)",
    );
    assert_contains(&swift, "let deckOptionsBuryNewSiblingsValue: Bool");
    assert_contains(&swift, "let deckOptionsBuryReviewSiblingsValue: Bool");
    assert_contains(
        &swift,
        "let deckOptionsBuryInterdayLearningSiblingsValue: Bool",
    );
    assert_contains(&swift, "let historyLabel: String");
    assert_contains(&swift, "let historyTotalValue: String");
    assert_contains(&swift, "let historyAccuracyValue: String");
    assert_contains(&swift, "let collectionLabel: String");
    assert_contains(&swift, "let collectionNoteCountValue: String");
    assert_contains(&swift, "let browserResults: [String]");
    assert_contains(&swift, "let browserFilterValue: String");
    assert_contains(&swift, "let browserFilterOptions: [String]");
    assert_contains(&swift, "let browserFlagValue: String");
    assert_contains(&swift, "let browserFlagOptions: [String]");
    assert_contains(&swift, "let browserFlagOpen: Bool");
    assert_contains(&swift, "let browserTagEdit: String");
    assert_contains(&swift, "let browserAddTagLabel: String");
    assert_contains(&swift, "let browserResultCardIds: [String]");
    assert_contains(&swift, "let browserSelectedCardId: String");
    assert_contains(&swift, "let noteEditorNoteTypeNames: [String]");
    assert_contains(&swift, "let noteEditorDeckNames: [String]");
    assert_contains(&swift, "let noteEditorSelectedNoteTypeIndex: Double");
    assert_contains(&swift, "let noteEditorSelectedDeckIndex: Double");
    assert_contains(&swift, "let answerVisible: Bool");
    assert_contains(&swift, "let typeAnswerActive: Bool");
    assert_contains(&swift, "let typeAnswerValue: String");
    assert_contains(&swift, "let actionUndoLabel: String");
    assert_contains(&swift, "let actionMarkLabel: String");
    let swift_app = fs::read_to_string(
        tmp.path()
            .join("swiftui")
            .join("Sources")
            .join("App")
            .join("App.swift"),
    )
    .expect("Sources/App/App.swift");
    assert_contains(&swift_app, "EngramAppView(");
    assert_contains(
        &swift_app,
        "appTitle: MosaicHostValue.string(host.props, \"app-title\", fallback: \"Sample AppTitle\"),",
    );
    assert_contains(
        &swift_app,
        "deckNames: MosaicHostValue.stringList(host.props, \"deck-names\", fallback: []),",
    );
    assert_contains(
        &swift_app,
        "deckOptionsSettingsLabel: MosaicHostValue.string(host.props, \"deck-options-settings-label\", fallback: \"Sample DeckOptionsSettingsLabel\"),",
    );
    assert_contains(
        &swift_app,
        "deckOptionsLearningStepsValue: MosaicHostValue.string(host.props, \"deck-options-learning-steps-value\", fallback: \"Sample DeckOptionsLearningStepsValue\"),",
    );
    assert_contains(
        &swift_app,
        "deckOptionsNewCardsValue: MosaicHostValue.double(host.props, \"deck-options-new-cards-value\", fallback: 0),",
    );
    assert_contains(
        &swift_app,
        "deckOptionsEasyBonusValue: MosaicHostValue.double(host.props, \"deck-options-easy-bonus-value\", fallback: 0),",
    );
    assert_contains(
        &swift_app,
        "deckOptionsInitialEaseValue: MosaicHostValue.double(host.props, \"deck-options-initial-ease-value\", fallback: 0),",
    );
    assert_contains(
        &swift_app,
        "deckOptionsLeechThresholdValue: MosaicHostValue.double(host.props, \"deck-options-leech-threshold-value\", fallback: 0),",
    );
    assert_contains(
        &swift_app,
        "deckOptionsLeechActionSuspendValue: MosaicHostValue.bool(host.props, \"deck-options-leech-action-suspend-value\", fallback: false),",
    );
    assert_contains(
        &swift_app,
        "deckOptionsBuryNewSiblingsValue: MosaicHostValue.bool(host.props, \"deck-options-bury-new-siblings-value\", fallback: false),",
    );
    assert_contains(
        &swift_app,
        "historyLabel: MosaicHostValue.string(host.props, \"history-label\", fallback: \"Sample HistoryLabel\"),",
    );
    assert_contains(
        &swift_app,
        "historyLastValue: MosaicHostValue.string(host.props, \"history-last-value\", fallback: \"Sample HistoryLastValue\"),",
    );
    assert_contains(
        &swift_app,
        "collectionLabel: MosaicHostValue.string(host.props, \"collection-label\", fallback: \"Sample CollectionLabel\"),",
    );
    assert_contains(
        &swift_app,
        "collectionImportLabel: MosaicHostValue.string(host.props, \"collection-import-label\", fallback: \"Sample CollectionImportLabel\"),",
    );
    assert_contains(
        &swift_app,
        "collectionMissingMediaFilenames: MosaicHostValue.stringList(host.props, \"collection-missing-media-filenames\", fallback: []),",
    );
    assert_contains(
        &swift_app,
        "collectionUnusedMediaAssetIds: MosaicHostValue.stringList(host.props, \"collection-unused-media-asset-ids\", fallback: []),",
    );
    assert_contains(
        &swift_app,
        "collectionPruneUnusedMediaLabel: MosaicHostValue.string(host.props, \"collection-prune-unused-media-label\", fallback: \"Sample CollectionPruneUnusedMediaLabel\"),",
    );
    assert_contains(
        &swift_app,
        "browserQuery: MosaicHostValue.string(host.props, \"browser-query\", fallback: \"Sample BrowserQuery\"),",
    );
    assert_contains(
        &swift_app,
        "browserFlagValue: MosaicHostValue.string(host.props, \"browser-flag-value\", fallback: \"Sample BrowserFlagValue\"),",
    );
    assert_contains(
        &swift_app,
        "browserFlagOptions: MosaicHostValue.stringList(host.props, \"browser-flag-options\", fallback: []),",
    );
    assert_contains(
        &swift_app,
        "browserFlagOpen: MosaicHostValue.bool(host.props, \"browser-flag-open\", fallback: false),",
    );
    assert_contains(
        &swift_app,
        "browserTagEdit: MosaicHostValue.string(host.props, \"browser-tag-edit\", fallback: \"Sample BrowserTagEdit\"),",
    );
    assert_contains(
        &swift_app,
        "browserAddTagLabel: MosaicHostValue.string(host.props, \"browser-add-tag-label\", fallback: \"Sample BrowserAddTagLabel\"),",
    );
    assert_contains(
        &swift_app,
        "browserCustomStudyLimitValue: MosaicHostValue.double(host.props, \"browser-custom-study-limit-value\", fallback: 0),",
    );
    assert_contains(
        &swift_app,
        "browserCustomStudyRescheduleValue: MosaicHostValue.bool(host.props, \"browser-custom-study-reschedule-value\", fallback: false),",
    );
    assert_contains(
        &swift_app,
        "browserResults: MosaicHostValue.stringList(host.props, \"browser-results\", fallback: []),",
    );
    assert_contains(
        &swift_app,
        "browserResultCardIds: MosaicHostValue.stringList(host.props, \"browser-result-card-ids\", fallback: []),",
    );
    assert_contains(
        &swift_app,
        "browserSelectedCardId: MosaicHostValue.string(host.props, \"browser-selected-card-id\", fallback: \"Sample BrowserSelectedCardId\"),",
    );
    assert_contains(
        &swift_app,
        "answerVisible: MosaicHostValue.bool(host.props, \"answer-visible\", fallback: false),",
    );
    assert_contains(
        &swift_app,
        "typeAnswerActive: MosaicHostValue.bool(host.props, \"type-answer-active\", fallback: false),",
    );
    assert_contains(
        &swift_app,
        "typeAnswerValue: MosaicHostValue.string(host.props, \"type-answer-value\", fallback: \"Sample TypeAnswerValue\"),",
    );
    assert_contains(
        &swift_app,
        "actionUndoLabel: MosaicHostValue.string(host.props, \"action-undo-label\", fallback: \"Sample ActionUndoLabel\"),",
    );
    assert_contains(
        &swift_app,
        "actionMarkLabel: MosaicHostValue.string(host.props, \"action-mark-label\", fallback: \"Sample ActionMarkLabel\"),",
    );
    assert_contains(&swift_app, "dispatch: { event in");
    assert_contains(&swift_app, "host.dispatch(event)");
    assert_contains(&swift_app, "final class MosaicHostState: ObservableObject");
    assert_contains(
        &swift_app,
        "@Published var lastHostIntent: [String: Any]? = nil",
    );
    assert_contains(
        &swift_app,
        "private func applyHostResponse(_ response: [String: Any]?)",
    );
    assert_contains(&swift_app, "self.lastHostIntent = intent");
    assert_contains(&swift_app, "MosaicHostBridge.load()");
    assert_contains(&swift_app, "@objc protocol MosaicHostBridgeObject");
    assert_contains(&swift_app, "[\"App.MosaicHost\", \"MosaicHost\"]");
    assert_contains(&swift_app, "NSClassFromString(className)");
    assert!(
        tmp.path()
            .join("swiftui")
            .join("Sources")
            .join("App")
            .join("MosaicHost.swift")
            .exists(),
        "swiftui host adapter should be installed from manifest assets"
    );
    let swift_package = fs::read_to_string(tmp.path().join("swiftui").join("Package.swift"))
        .expect("Package.swift");
    assert_contains(&swift_package, "platforms: [.macOS(.v13), .iOS(.v16)]");
    let swift_readme =
        fs::read_to_string(tmp.path().join("swiftui").join("README.md")).expect("swift README");
    assert_contains(&swift_readme, "SwiftUI macOS and iOS-ready shell");
    assert_contains(&swift_readme, "## Run on macOS");
    assert_contains(&swift_readme, "## Use from iOS");
    assert_contains(&swift_readme, "Sources/App/EngramApp.swift");
    assert!(
        !swift_readme.contains("mv EngramApp.swift"),
        "SwiftUI README should not ask package users to move the component"
    );
    let swift_nested_component = fs::read_to_string(
        tmp.path()
            .join("swiftui")
            .join("Sources")
            .join("App")
            .join("EngramApp.swift"),
    )
    .expect("Sources/App/EngramApp.swift");
    assert_contains(&swift_nested_component, "struct EngramAppView: View");

    let xaml_markup =
        fs::read_to_string(tmp.path().join("xaml").join("EngramApp.xaml")).expect("EngramApp.xaml");
    assert_contains(&xaml_markup, "Spacing=\"18\"");
    assert_contains(&xaml_markup, "Spacing=\"16\"");
    assert_contains(&xaml_markup, "MaxWidth=\"980\"");
    assert_contains(
        &xaml_markup,
        "<ContentControl Visibility=\"{x:Bind AnswerVisible, Converter={StaticResource BoolToVisibilityConverter}}\">\n                                    <StackPanel Orientation=\"Vertical\">",
    );
    assert_contains(
        &xaml_markup,
        "<DataTemplate x:DataType=\"local:EngramApp_ItemVm\">\n                                                    <StackPanel Orientation=\"Vertical\">",
    );
    for invalid in [
        "Property=\"Gap\"",
        "Property=\"AlignItems\"",
        "Property=\"FlexWrap\"",
        "Property=\"JustifyContent\"",
        "Value=\"980px\"",
        "Value=\"760px\"",
        "Value=\"960px\"",
    ] {
        assert!(
            !xaml_markup.contains(invalid),
            "Engram XAML must not contain invalid WinUI style fragment `{invalid}`:\n{xaml_markup}"
        );
    }

    let xaml_code_behind = fs::read_to_string(tmp.path().join("xaml").join("EngramApp.xaml.cs"))
        .expect("EngramApp.xaml.cs");
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty AppTitleProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckNamesProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckOptionsSettingsLabelProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckOptionsLearningStepsValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckOptionsRelearningStepsValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckOptionsNewCardsValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckOptionsEasyBonusValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckOptionsInitialEaseValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckOptionsDesiredRetentionValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckOptionsFsrsParametersValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckOptionsLeechThresholdValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckOptionsLeechActionSuspendValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckOptionsBuryNewSiblingsValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckOptionsBuryReviewSiblingsValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty DeckOptionsBuryInterdayLearningSiblingsValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty HistoryLabelProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty HistoryTotalValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty HistoryAccuracyValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty CollectionLabelProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty CollectionNoteCountValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty BrowserResultsProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty BrowserFilterValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty BrowserFilterOptionsProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty BrowserFlagValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty BrowserFlagOptionsProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty BrowserFlagOpenProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty BrowserTagEditProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty BrowserAddTagLabelProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty BrowserCustomStudyLimitValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty BrowserCustomStudyRescheduleValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty BrowserResultCardIdsProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty BrowserSelectedCardIdProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty NoteEditorNoteTypeNamesProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty NoteEditorDeckNamesProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty NoteEditorSelectedNoteTypeIndexProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty NoteEditorSelectedDeckIndexProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty NoteTypeEditorTemplateLabelsProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty NoteTypeEditorSelectedTemplateIndexProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty NoteTypeEditorFrontTemplateValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty NoteTypeEditorBackTemplateValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty AnswerVisibleProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty TypeAnswerActiveProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty TypeAnswerValueProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty ActionUndoLabelProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty ActionMarkLabelProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public event EventHandler<EngramAppEvent>? Dispatch;",
    );

    let xaml_events = fs::read_to_string(tmp.path().join("xaml").join("EngramApp.Event.cs"))
        .expect("EngramApp.Event.cs");
    assert_contains(&xaml_events, "public abstract string MosaicName { get; }");
    assert_contains(
        &xaml_events,
        "public System.Collections.Generic.IReadOnlyDictionary<string, object?> MosaicEnvelope",
    );
    assert_contains(
        &xaml_events,
        "public sealed record Reveal() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record SelectDeck(string Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record Again() : EngramAppEvent",
    );
    assert_contains(&xaml_events, "public sealed record Hard() : EngramAppEvent");
    assert_contains(&xaml_events, "public sealed record Good() : EngramAppEvent");
    assert_contains(&xaml_events, "public sealed record Easy() : EngramAppEvent");
    assert_contains(&xaml_events, "public sealed record Undo() : EngramAppEvent");
    assert_contains(
        &xaml_events,
        "public sealed record BuryCard() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BurySiblings() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record SuspendCard() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record ToggleMark() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record TypeAnswerChange(string Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeckOptionsLearningStepsChange(string Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeckOptionsRelearningStepsChange(string Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeckOptionsNewCardsChange(double Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeckOptionsMaximumIntervalChange(double Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeckOptionsInitialEaseChange(double Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeckOptionsDesiredRetentionChange(double Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeckOptionsFsrsParametersChange(string Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeckOptionsEasyBonusChange(double Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeckOptionsLeechThresholdChange(double Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeckOptionsLeechActionChange(string Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeckOptionsBuryNewSiblingsChange(bool Checked) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeckOptionsBuryReviewSiblingsChange(bool Checked) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeckOptionsBuryInterdayLearningSiblingsChange(bool Checked) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record ImportAnki() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record ExportAnki() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record PruneUnusedMedia() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record AddNote() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record AddNoteType() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeleteNote() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record DeleteNoteType() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BrowserSearch() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BrowserSelectResult(double Index) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BrowserToggleFilter() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BrowserSetFilter(string Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BrowserToggleFlagPicker() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BrowserSetFlagSelected(string Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BrowserTagEditChange(string Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BrowserAddTagSelected() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BrowserRemoveTagSelected() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BrowserCustomStudyLimitChange(double Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BrowserCustomStudyRescheduleChange(bool Checked) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BrowserRebuildFilteredDeck() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record BrowserEmptyFilteredDeck() : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record NoteEditorSelectNoteType(double Index) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record NoteEditorSelectDeck(double Index) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record NoteTypeEditorSelectTemplate(double Index) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record NoteTypeEditorTemplateNameChange(string Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record NoteTypeEditorFrontTemplateChange(string Value) : EngramAppEvent",
    );
    assert_contains(
        &xaml_events,
        "public sealed record NoteTypeEditorBackTemplateChange(string Value) : EngramAppEvent",
    );
    let xaml_main_window = fs::read_to_string(tmp.path().join("xaml").join("MainWindow.xaml.cs"))
        .expect("MainWindow.xaml.cs");
    assert_contains(&xaml_main_window, "TryApplyMosaicHostProps");
    assert_contains(&xaml_main_window, "CoerceMosaicHostResult");
    assert_contains(&xaml_main_window, "private async void OnComponentDispatch");
    assert_contains(&xaml_main_window, "await TryHandleMosaicHostEvent");
    assert_contains(&xaml_main_window, "TryHandleMosaicHostIntent");
    assert_contains(&xaml_main_window, "UnwrapMosaicHostResultAsync");
    assert_contains(&xaml_main_window, "HandleHostIntent");
    assert_contains(
        &xaml_main_window,
        "FindMosaicHostMethod(\"ApplyProps\", typeof(EngramApp))",
    );
    assert_contains(
        &xaml_main_window,
        "FindMosaicHostMethod(\"HandleEvent\", typeof(EngramApp), typeof(EngramAppEvent))",
    );
    assert_contains(
        &xaml_main_window,
        "System.Type.GetType(\"Mosaic.Generated.MosaicHost\")",
    );
    assert!(
        tmp.path().join("xaml").join("MosaicHost.cs").exists(),
        "xaml host adapter should be installed from manifest assets"
    );
    let xaml_csproj = fs::read_to_string(tmp.path().join("xaml").join("EngramApp.csproj"))
        .expect("xaml/EngramApp.csproj");
    assert_contains(&xaml_csproj, "CopyMosaicNativeHostLibraries");
    assert_contains(&xaml_csproj, "$(MSBuildProjectDirectory)\\*.dll");
    assert_contains(&xaml_main_window, "Status: sample props loaded");

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
    assert_contains(&capi_header, "eg_engram_browser_props");
}

fn assert_contains(haystack: &str, needle: &str) {
    assert!(
        haystack.contains(needle),
        "expected generated artifact to contain `{needle}`"
    );
}

fn assert_dependency_styles_reach_all_backends(output_root: &Path) {
    let hex_sentinels = [
        ("DeckStatsPanel", "#2563eb"),
        ("ReviewHistoryPanel", "#ca8a04"),
        ("ReviewCard", "#e94560"),
        ("SessionProgress", "#0f766e"),
        ("ReviewActions", "#7c3aed"),
        ("RatingControls", "#f87171"),
        ("NoteTypeEditor", "#1d4ed8"),
    ];

    for (backend, artifact) in [
        ("HTML", "html/EngramApp.html"),
        ("WebComponent", "webcomponent/EngramApp.js"),
        ("React", "react/EngramApp.tsx"),
        ("Electron", "electron/EngramApp.tsx"),
        ("Qt", "qt/EngramApp.qml"),
        ("XAML", "xaml/EngramApp.xaml"),
    ] {
        let source = read_artifact(output_root, artifact);
        for (component, needle) in hex_sentinels {
            assert!(
                source.contains(needle),
                "{component} dependency style {needle} should reach {backend} artifact {artifact}"
            );
        }
    }

    let flutter = read_artifact(output_root, "flutter/EngramApp.dart");
    for (component, hex) in hex_sentinels {
        let needle = format!("0xFF{}", hex.trim_start_matches('#').to_ascii_uppercase());
        assert!(
            flutter.contains(&needle),
            "{component} dependency style {needle} should reach Flutter artifact"
        );
    }

    let compose = read_artifact(output_root, "compose/EngramApp.kt");
    for (component, hex) in hex_sentinels {
        let needle = format!(
            "Color(0xFF{})",
            hex.trim_start_matches('#').to_ascii_uppercase()
        );
        assert!(
            compose.contains(&needle),
            "{component} dependency style {needle} should reach Compose artifact"
        );
    }
    assert!(
        read_artifact(output_root, "html/EngramApp.html").contains("#0891b2"),
        "CollectionActions dependency style should reach HTML artifact"
    );
    assert!(
        read_artifact(output_root, "html/EngramApp.html").contains("#f59e0b"),
        "DeckOptionsPanel dependency style should reach HTML artifact"
    );
    assert!(
        read_artifact(output_root, "webcomponent/EngramApp.js").contains("#0891b2"),
        "CollectionActions dependency style should reach WebComponent artifact"
    );
    assert!(
        read_artifact(output_root, "webcomponent/EngramApp.js").contains("#f59e0b"),
        "DeckOptionsPanel dependency style should reach WebComponent artifact"
    );
    assert!(
        read_artifact(output_root, "react/EngramApp.tsx").contains("#0891b2"),
        "CollectionActions dependency style should reach React artifact"
    );
    assert!(
        read_artifact(output_root, "react/EngramApp.tsx").contains("#f59e0b"),
        "DeckOptionsPanel dependency style should reach React artifact"
    );
    assert!(
        read_artifact(output_root, "electron/EngramApp.tsx").contains("#0891b2"),
        "CollectionActions dependency style should reach Electron artifact"
    );
    assert!(
        read_artifact(output_root, "electron/EngramApp.tsx").contains("#f59e0b"),
        "DeckOptionsPanel dependency style should reach Electron artifact"
    );
    assert!(
        read_artifact(output_root, "xaml/EngramApp.xaml").contains("#0891b2"),
        "CollectionActions dependency style should reach XAML artifact"
    );
    assert!(
        read_artifact(output_root, "xaml/EngramApp.xaml").contains("#f59e0b"),
        "DeckOptionsPanel dependency style should reach XAML artifact"
    );
    assert!(
        compose.contains("Color(0xFF0891B2)"),
        "CollectionActions dependency style should reach Compose artifact"
    );
    assert!(
        compose.contains("Color(0xFFF59E0B)"),
        "DeckOptionsPanel dependency style should reach Compose artifact"
    );

    let swift = read_artifact(output_root, "swiftui/EngramApp.swift");
    for (component, needle) in [
        ("DeckStatsPanel", "0.145, green: 0.388, blue: 0.922"),
        ("ReviewHistoryPanel", "0.792, green: 0.541, blue: 0.016"),
        ("DeckOptionsPanel", "0.961, green: 0.62, blue: 0.043"),
        ("CollectionActions", "0.031, green: 0.569, blue: 0.698"),
        ("ReviewCard", "0.914, green: 0.271, blue: 0.376"),
        ("SessionProgress", "0.059, green: 0.463, blue: 0.431"),
        ("ReviewActions", "0.486, green: 0.227, blue: 0.929"),
        ("RatingControls", "0.973, green: 0.443, blue: 0.443"),
    ] {
        assert!(
            swift.contains(needle),
            "{component} dependency style {needle} should reach SwiftUI artifact"
        );
    }
}

fn read_artifact(output_root: &Path, relative: &str) -> String {
    let path = output_root.join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("expected generated artifact {}: {e}", path.display()))
}

#[test]
fn source_tree_has_expected_shape() {
    let expected = [
        "EngramApp.mil",
        "EngramApp.mll",
        "EngramApp.touch.mll",
        "EngramApp.dark.msl",
        "EngramApp.light.msl",
    ];
    for name in expected {
        let path = src_path(name);
        assert!(
            path.exists(),
            "expected source file missing: {}",
            path.display()
        );
    }

    for relative in [
        "host/web/engram-host.ts",
        "host/web/engram-host.mjs",
        "host/web/engram-mosaic-host-wasm.d.ts",
        "host/electron/host.js",
        "host/qt/MosaicHost.h",
        "host/qt/MosaicHost.cpp",
        "host/swiftui/MosaicHost.swift",
        "host/compose/MosaicHost.kt",
        "host/flutter/mosaic_host.dart",
        "host/xaml/MosaicHost.cs",
    ] {
        let path = package_root().join(relative);
        assert!(
            path.exists(),
            "expected host template missing: {}",
            path.display()
        );
    }

    let web_ts_host = fs::read_to_string(package_root().join("host/web/engram-host.ts"))
        .expect("web ts host template");
    assert_contains(&web_ts_host, "handleHostIntent(engine, intent, result)");
    assert_contains(&web_ts_host, "importAnkiPackage(engine, intent)");
    assert_contains(&web_ts_host, "exportAnkiPackage(engine, intent)");
    assert_contains(&web_ts_host, "chooseAnkiImportFile(intent)");
    assert_contains(&web_ts_host, "engine.mergeAnkiApkg(bytes)");
    assert_contains(&web_ts_host, "engine.exportAnkiApkg()");
    assert_contains(&web_ts_host, "downloadBytes(bytes, name)");
    assert_contains(&web_ts_host, "hostResultStatus(result) === \"imported\"");
    assert_contains(&web_ts_host, "hostStatusProps(hostResult)");
    assert_contains(&web_ts_host, "hostStatusVisible: true");

    let web_mjs_host = fs::read_to_string(package_root().join("host/web/engram-host.mjs"))
        .expect("web mjs host template");
    assert_contains(&web_mjs_host, "handleHostIntent(engine, intent, result)");
    assert_contains(&web_mjs_host, "importAnkiPackage(engine, intent)");
    assert_contains(&web_mjs_host, "exportAnkiPackage(engine, intent)");
    assert_contains(&web_mjs_host, "chooseAnkiImportFile(intent)");
    assert_contains(&web_mjs_host, "engine.mergeAnkiApkg(bytes)");
    assert_contains(&web_mjs_host, "engine.exportAnkiApkg()");
    assert_contains(&web_mjs_host, "downloadBytes(bytes, name)");
    assert_contains(&web_mjs_host, "hostResultStatus(result) === \"imported\"");
    assert_contains(&web_mjs_host, "hostStatusProps(hostResult)");
    assert_contains(&web_mjs_host, "hostStatusVisible: true");

    let web_wasm_defs =
        fs::read_to_string(package_root().join("host/web/engram-mosaic-host-wasm.d.ts"))
            .expect("web wasm declarations");
    assert_contains(&web_wasm_defs, "exportAnkiApkg");
    assert_contains(&web_wasm_defs, "mergeAnkiApkg");
    assert_contains(&web_wasm_defs, "Promise<unknown>");

    let xaml_host = fs::read_to_string(package_root().join("host/xaml/MosaicHost.cs"))
        .expect("xaml host template");
    assert_contains(&xaml_host, "eg_engram_app_props");
    assert_contains(&xaml_host, "eg_handle_engram_app_event");
    assert_contains(&xaml_host, "SlotNameToPropertyName");
    assert_contains(&xaml_host, "public sealed record MosaicHostIntent");
    assert_contains(&xaml_host, "public sealed record MosaicHostResult");
    assert_contains(&xaml_host, "HostIntentReceived");
    assert_contains(&xaml_host, "LastHostIntent");
    assert_contains(&xaml_host, "HandleHostIntent");
    assert_contains(&xaml_host, "FileOpenPicker");
    assert_contains(&xaml_host, "FileSavePicker");
    assert_contains(&xaml_host, "ImportAnkiPackage");
    assert_contains(&xaml_host, "ExportAnkiPackage");
    assert_contains(&xaml_host, "eg_merge_anki_apkg");
    assert_contains(&xaml_host, "eg_export_anki_apkg");
    assert_contains(&xaml_host, "private static IntPtr Session = IntPtr.Zero");
    assert_contains(&xaml_host, "TryGetSession");
    assert_contains(&xaml_host, "Engram native host unavailable");
    assert_contains(&xaml_host, "ENGRAM_SNAPSHOT_PATH");
    assert_contains(&xaml_host, "mosaic-snapshot.v1.json");
    assert_contains(&xaml_host, "HydrateSession");
    assert_contains(&xaml_host, "PersistSnapshot");
    assert_contains(&xaml_host, "eg_snapshot");
    assert_contains(&xaml_host, "eg_load_snapshot");
    assert_contains(&xaml_host, "ApplyHostStatus(component, \"imported\"");
    assert_contains(&xaml_host, "component.HostStatusVisible = true");
    assert_contains(
        &xaml_host,
        "component.HostStatusMessage = HostStatusMessage",
    );
    assert!(
        !xaml_host.contains("private static IntPtr Session = Native.eg_session_new()"),
        "xaml host template must not load the native library from a static field initializer"
    );

    let electron_host = fs::read_to_string(package_root().join("host/electron/host.js"))
        .expect("electron host template");
    assert_contains(&electron_host, "dialog");
    assert_contains(&electron_host, "spawnSync");
    assert_contains(&electron_host, "ENGRAM_HOST_CLI");
    assert_contains(&electron_host, "engram-host-cli.exe");
    assert_contains(&electron_host, "handleHostIntent");
    assert_contains(&electron_host, "importAnkiPackage");
    assert_contains(&electron_host, "exportAnkiPackage");
    assert_contains(&electron_host, "runSidecar");
    assert_contains(&electron_host, "merge-apkg");
    assert_contains(&electron_host, "export-apkg");
    assert_contains(&electron_host, "loadPersistedSnapshot");
    assert_contains(&electron_host, "persistSnapshot(engine)");
    assert_contains(&electron_host, "hostStatusProps(result.hostResult)");
    assert_contains(&electron_host, "hostStatusVisible: true");

    let swiftui_host = fs::read_to_string(package_root().join("host/swiftui/MosaicHost.swift"))
        .expect("swiftui host template");
    assert_contains(&swiftui_host, "MosaicHostBridgeObject");
    assert_contains(&swiftui_host, "eg_engram_app_props");
    assert_contains(&swiftui_host, "eg_handle_engram_app_event");
    assert_contains(&swiftui_host, "hostResponseDictionary");
    assert_contains(&swiftui_host, "import AppKit");
    assert_contains(&swiftui_host, "handleHostIntent");
    assert_contains(&swiftui_host, "importAnkiPackage");
    assert_contains(&swiftui_host, "exportAnkiPackage");
    assert_contains(&swiftui_host, "NSOpenPanel");
    assert_contains(&swiftui_host, "NSSavePanel");
    assert_contains(&swiftui_host, "eg_merge_anki_apkg");
    assert_contains(&swiftui_host, "eg_export_anki_apkg");
    assert_contains(&swiftui_host, "\"hostIntent\"");
    assert_contains(&swiftui_host, "\"hostResult\"");
    assert_contains(&swiftui_host, "\"props\"");
    assert_contains(&swiftui_host, "ENGRAM_SNAPSHOT_PATH");
    assert_contains(&swiftui_host, "mosaic-snapshot.v1.json");
    assert_contains(&swiftui_host, "hydrateSession");
    assert_contains(&swiftui_host, "persistSnapshot");
    assert_contains(&swiftui_host, "eg_snapshot");
    assert_contains(&swiftui_host, "eg_load_snapshot");
    assert_contains(&swiftui_host, "withHostStatusProps");
    assert_contains(&swiftui_host, "\"host-status-visible\": true");
    assert_contains(&swiftui_host, "hostResult[\"error\"] = error");
    assert_contains(&swiftui_host, "Could not import \\(subject): \\(error)");
    assert_contains(&swiftui_host, "Could not export Anki package: \\(error)");

    let qt_host = fs::read_to_string(package_root().join("host/qt/MosaicHost.cpp"))
        .expect("qt host template");
    assert_contains(&qt_host, "eg_engram_app_props");
    assert_contains(&qt_host, "eg_handle_engram_app_event");
    assert_contains(&qt_host, "QLibrary");
    assert_contains(&qt_host, "mosaicPropName");
    assert_contains(&qt_host, "hostResponseFromJson");
    assert_contains(&qt_host, "hostIntent");
    assert_contains(&qt_host, "QFileDialog");
    assert_contains(&qt_host, "handleHostIntent");
    assert_contains(&qt_host, "importAnkiPackage");
    assert_contains(&qt_host, "exportAnkiPackage");
    assert_contains(&qt_host, "eg_merge_anki_apkg");
    assert_contains(&qt_host, "eg_export_anki_apkg");
    assert_contains(&qt_host, "props");
    assert_contains(&qt_host, "ENGRAM_SNAPSHOT_PATH");
    assert_contains(&qt_host, "mosaic-snapshot.v1.json");
    assert_contains(&qt_host, "hydrateSession");
    assert_contains(&qt_host, "persistSnapshot");
    assert_contains(&qt_host, "eg_snapshot");
    assert_contains(&qt_host, "eg_load_snapshot");
    assert_contains(&qt_host, "withHostStatusProps");
    assert_contains(&qt_host, "QStringLiteral(\"hostStatusVisible\")");
    assert_contains(
        &qt_host,
        "hostResult.insert(QStringLiteral(\"error\"), error)",
    );
    assert_contains(&qt_host, "Could not import %1: %2");
    assert_contains(&qt_host, "Could not export Anki package: %1");

    let compose_host = fs::read_to_string(package_root().join("host/compose/MosaicHost.kt"))
        .expect("compose host template");
    assert_contains(&compose_host, "interface EngramCapi");
    assert_contains(&compose_host, "eg_engram_app_props");
    assert_contains(&compose_host, "eg_handle_engram_app_event");
    assert_contains(&compose_host, "Native.load");
    assert_contains(&compose_host, "JSONObject(event)");
    assert_contains(&compose_host, "\"hostIntent\"");
    assert_contains(&compose_host, "\"hostResult\"");
    assert_contains(&compose_host, "\"props\"");
    assert_contains(&compose_host, "JFileChooser");
    assert_contains(&compose_host, "FileNameExtensionFilter");
    assert_contains(&compose_host, "importAnkiPackage");
    assert_contains(&compose_host, "exportAnkiPackage");
    assert_contains(&compose_host, "eg_merge_anki_apkg");
    assert_contains(&compose_host, "eg_export_anki_apkg");
    assert_contains(&compose_host, "ENGRAM_SNAPSHOT_PATH");
    assert_contains(&compose_host, "mosaic-snapshot.v1.json");
    assert_contains(&compose_host, "hydrateSession");
    assert_contains(&compose_host, "persistSnapshot");
    assert_contains(&compose_host, "eg_snapshot");
    assert_contains(&compose_host, "eg_load_snapshot");
    assert_contains(&compose_host, "withHostStatusProps");
    assert_contains(&compose_host, "\"host-status-visible\" to true");
    assert_contains(&compose_host, "hostResult[\"error\"] = error.toString()");
    assert_contains(&compose_host, "Could not import $file: $error");
    assert_contains(&compose_host, "Could not export Anki package: $error");

    let flutter_host = fs::read_to_string(package_root().join("host/flutter/mosaic_host.dart"))
        .expect("flutter host template");
    assert_contains(&flutter_host, "dart:ffi");
    assert_contains(&flutter_host, "package:ffi/ffi.dart");
    assert_contains(&flutter_host, "DynamicLibrary.open");
    assert_contains(&flutter_host, "eg_engram_app_props");
    assert_contains(&flutter_host, "eg_handle_engram_app_event");
    assert_contains(&flutter_host, "eg_export_anki_apkg");
    assert_contains(&flutter_host, "eg_merge_anki_apkg");
    assert_contains(&flutter_host, "jsonEncode(event)");
    assert_contains(&flutter_host, "openFile");
    assert_contains(&flutter_host, "getSaveLocation");
    assert_contains(&flutter_host, "'hostIntent'");
    assert_contains(&flutter_host, "'hostResult'");
    assert_contains(&flutter_host, "'props'");
    assert_contains(&flutter_host, "ENGRAM_SNAPSHOT_PATH");
    assert_contains(&flutter_host, "mosaic-snapshot.v1.json");
    assert_contains(&flutter_host, "_hydrateSession");
    assert_contains(&flutter_host, "_persistSnapshot");
    assert_contains(&flutter_host, "eg_snapshot");
    assert_contains(&flutter_host, "eg_load_snapshot");
    assert_contains(&flutter_host, "_withHostStatusProps");
    assert_contains(&flutter_host, "'host-status-visible': true");

    let build_script =
        fs::read_to_string(package_root().join("scripts/build-all.ps1")).expect("build-all.ps1");
    assert_contains(&build_script, "Install-EngramHtmlHost");
    assert_contains(&build_script, "Install-EngramWebComponentHost");
    assert_contains(&build_script, "Install-EngramXamlHost");
    assert_contains(&build_script, "Install-EngramQtHost");
    assert_contains(&build_script, "Install-EngramSwiftUIHost");
    assert_contains(&build_script, "Install-EngramFlutterHost");
    assert_contains(&build_script, "Install-EngramComposeHost");
    assert_contains(&build_script, "ffi: ^2.1.3");
    assert_contains(&build_script, "file_selector: ^1.0.3");
    assert_contains(&build_script, "net.java.dev.jna:jna:5.19.1");
    assert_contains(&build_script, "org.json:json:20260522");
    assert_contains(&build_script, "module CEngram");
    assert_contains(&build_script, "libengram_capi.a");
    assert_contains(&build_script, "engram_capi.dll");
    assert_contains(&build_script, "hostCliName");
    assert_contains(&build_script, "engram-host-cli.exe");
    assert_contains(&build_script, "engramHostCli");
    assert!(
        !build_script.contains("Add-EngramXamlNativeContent"),
        "XAML native library copying should be owned by the generated project"
    );
}
