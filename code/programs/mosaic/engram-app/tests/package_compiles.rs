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
    assert!(source.contains("pkg::mosaic-pkg-card-browser::CardBrowser"));
    assert!(source.contains("pkg::mosaic-pkg-collection-actions::CollectionActions"));
    assert!(source.contains("pkg::mosaic-pkg-deck-options::DeckOptionsPanel"));
    assert!(source.contains("pkg::mosaic-pkg-deck-stats::DeckStatsPanel"));
    assert!(source.contains("pkg::mosaic-pkg-review-card::ReviewCard"));
    assert!(source.contains("pkg::mosaic-pkg-review-actions::ReviewActions"));
    assert!(source.contains("pkg::mosaic-pkg-review-history::ReviewHistoryPanel"));
    assert!(source.contains("pkg::mosaic-pkg-session-progress::SessionProgress"));
    assert!(!source.contains("layout CardBrowser"));
    assert!(!source.contains("layout CollectionActions"));
    assert!(!source.contains("layout DeckOptionsPanel"));
    assert!(!source.contains("layout DeckStatsPanel"));
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
        (Backend::Html, "html/EngramApp.html"),
        (Backend::React, "react/EngramApp.tsx"),
        (Backend::Electron, "electron/EngramApp.tsx"),
        (Backend::SwiftUI, "swiftui/EngramApp.swift"),
        (Backend::Qt, "qt/EngramApp.qml"),
        (Backend::Xaml, "xaml/EngramApp.xaml"),
        (Backend::Flutter, "flutter/EngramApp.dart"),
        (Backend::Compose, "compose/EngramApp.kt"),
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

    let html = read_artifact(tmp.path(), "html/EngramApp.html");
    assert_contains(&html, "data-on-click=\"onImportAnki\"");
    assert_contains(&html, "data-on-commit=\"onBrowserSearch\"");
    assert_dependency_styles_reach_all_backends(tmp.path());
}

#[test]
fn app_package_emits_native_project_shells() {
    let tmp = tempfile::tempdir().expect("temp dist root");
    let shells = [
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
        })
        .unwrap_or_else(|e| panic!("{backend:?} should emit EngramApp project shell: {e}"));
    }

    let react_app = fs::read_to_string(tmp.path().join("react").join("src").join("main.tsx"))
        .expect("react/src/main.tsx");
    assert_contains(&react_app, "const fallbackProps = {");
    assert_contains(&react_app, "appTitle: \"Sample AppTitle\",");
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
    assert_contains(&react_app, "browserResults: [],");
    assert_contains(&react_app, "browserResultCardIds: [],");
    assert_contains(
        &react_app,
        "browserSelectedCardId: \"Sample BrowserSelectedCardId\",",
    );
    assert_contains(&react_app, "answerVisible: false,");
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

    let electron_app = fs::read_to_string(tmp.path().join("electron").join("src").join("main.tsx"))
        .expect("electron/src/main.tsx");
    assert_contains(&electron_app, "const fallbackProps = {");
    assert_contains(&electron_app, "appTitle: \"Sample AppTitle\",");
    assert_contains(
        &electron_app,
        "deckOptionsSettingsLabel: \"Sample DeckOptionsSettingsLabel\",",
    );
    assert_contains(
        &electron_app,
        "deckOptionsRelearningStepsValue: \"Sample DeckOptionsRelearningStepsValue\",",
    );
    assert_contains(&electron_app, "deckOptionsMaximumIntervalValue: 0,");
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
    assert_contains(&electron_app, "browserResults: [],");
    assert_contains(&electron_app, "browserResultCardIds: [],");
    assert_contains(
        &electron_app,
        "browserSelectedCardId: \"Sample BrowserSelectedCardId\",",
    );
    assert_contains(&electron_app, "answerVisible: false,");
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

    let flutter_app = fs::read_to_string(tmp.path().join("flutter").join("lib").join("main.dart"))
        .expect("flutter/lib/main.dart");
    assert_contains(&flutter_app, "EngramApp(");
    assert_contains(&flutter_app, "appTitle: \"Sample AppTitle\",");
    assert_contains(
        &flutter_app,
        "deckOptionsSettingsLabel: \"Sample DeckOptionsSettingsLabel\",",
    );
    assert_contains(
        &flutter_app,
        "deckOptionsLearningStepsValue: \"Sample DeckOptionsLearningStepsValue\",",
    );
    assert_contains(&flutter_app, "deckOptionsNewCardsValue: 0.0,");
    assert_contains(&flutter_app, "deckOptionsHardMultiplierValue: 0.0,");
    assert_contains(&flutter_app, "deckOptionsBuryNewSiblingsValue: false,");
    assert_contains(&flutter_app, "historyLabel: \"Sample HistoryLabel\",");
    assert_contains(
        &flutter_app,
        "historyCorrectValue: \"Sample HistoryCorrectValue\",",
    );
    assert_contains(&flutter_app, "collectionLabel: \"Sample CollectionLabel\",");
    assert_contains(
        &flutter_app,
        "collectionExportLabel: \"Sample CollectionExportLabel\",",
    );
    assert_contains(&flutter_app, "browserQuery: \"Sample BrowserQuery\",");
    assert_contains(&flutter_app, "browserResults: const [],");
    assert_contains(&flutter_app, "browserResultCardIds: const [],");
    assert_contains(
        &flutter_app,
        "browserSelectedCardId: \"Sample BrowserSelectedCardId\",",
    );
    assert_contains(&flutter_app, "answerVisible: false,");
    assert_contains(&flutter_app, "actionUndoLabel: \"Sample ActionUndoLabel\",");
    assert_contains(&flutter_app, "actionMarkLabel: \"Sample ActionMarkLabel\",");
    assert_contains(
        &flutter_app,
        "dispatch: (event) => debugPrint(\"event: ${event.mosaicEnvelope}\")",
    );

    let compose_app =
        fs::read_to_string(tmp.path().join("compose").join("EngramApp.kt")).expect("EngramApp.kt");
    assert_contains(&compose_app, "sealed class EngramAppEvent {");
    assert_contains(&compose_app, "abstract val mosaicName: String");
    assert_contains(&compose_app, "val mosaicEnvelope: Map<String, Any?>");
    assert_contains(&compose_app, "data object ImportAnki : EngramAppEvent()");
    assert_contains(
        &compose_app,
        "override val mosaicName: String = \"onImportAnki\"",
    );
    assert_contains(
        &compose_app,
        "data class DeckOptionsNewCardsChange(val value: Double)",
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
    assert_contains(&compose_app, "@Composable");
    assert_contains(&compose_app, "fun EngramApp(");
    assert_contains(&compose_app, "appTitle: String,");
    assert_contains(&compose_app, "deckOptionsSettingsLabel: String,");
    assert_contains(&compose_app, "deckOptionsLearningStepsValue: String,");
    assert_contains(&compose_app, "deckOptionsNewCardsValue: Double,");
    assert_contains(&compose_app, "deckOptionsEasyBonusValue: Double,");
    assert_contains(&compose_app, "deckOptionsBuryNewSiblingsValue: Boolean,");
    assert_contains(&compose_app, "historyLabel: String,");
    assert_contains(&compose_app, "collectionLabel: String,");
    assert_contains(&compose_app, "browserQuery: String,");
    assert_contains(&compose_app, "browserResults: List<String>,");
    assert_contains(&compose_app, "browserResultCardIds: List<String>,");
    assert_contains(&compose_app, "browserSelectedCardId: String,");
    assert_contains(&compose_app, "answerVisible: Boolean,");
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
    assert_contains(&compose_main, "EngramApp(");
    assert_contains(&compose_main, "appTitle = \"Sample AppTitle\",");
    assert_contains(
        &compose_main,
        "deckOptionsSettingsLabel = \"Sample DeckOptionsSettingsLabel\",",
    );
    assert_contains(
        &compose_main,
        "deckOptionsLearningStepsValue = \"Sample DeckOptionsLearningStepsValue\",",
    );
    assert_contains(&compose_main, "deckOptionsNewCardsValue = 0.0,");
    assert_contains(&compose_main, "deckOptionsEasyBonusValue = 0.0,");
    assert_contains(&compose_main, "deckOptionsBuryNewSiblingsValue = false,");
    assert_contains(&compose_main, "historyLabel = \"Sample HistoryLabel\",");
    assert_contains(
        &compose_main,
        "collectionLabel = \"Sample CollectionLabel\",",
    );
    assert_contains(&compose_main, "browserQuery = \"Sample BrowserQuery\",");
    assert_contains(&compose_main, "browserResults = emptyList(),");
    assert_contains(&compose_main, "browserResultCardIds = emptyList(),");
    assert_contains(
        &compose_main,
        "browserSelectedCardId = \"Sample BrowserSelectedCardId\",",
    );
    assert_contains(&compose_main, "answerVisible = false,");
    assert_contains(
        &compose_main,
        "actionUndoLabel = \"Sample ActionUndoLabel\",",
    );
    assert_contains(
        &compose_main,
        "actionMarkLabel = \"Sample ActionMarkLabel\",",
    );
    assert_contains(
        &compose_main,
        "dispatch = { event -> println(\"event: ${event.mosaicEnvelope}\") }",
    );
    let compose_gradle = fs::read_to_string(tmp.path().join("compose").join("build.gradle.kts"))
        .expect("compose/build.gradle.kts");
    assert_contains(
        &compose_gradle,
        "id(\"org.jetbrains.compose\") version \"1.11.1\"",
    );
    assert_contains(&compose_gradle, "mainClass = \"MainKt\"");

    let qml =
        fs::read_to_string(tmp.path().join("qt").join("EngramApp.qml")).expect("EngramApp.qml");
    assert_contains(&qml, "property string appTitle");
    assert_contains(&qml, "property string deckOptionsSettingsLabel");
    assert_contains(&qml, "property string deckOptionsLearningStepsValue");
    assert_contains(&qml, "property string deckOptionsRelearningStepsValue");
    assert_contains(&qml, "property real deckOptionsNewCardsValue");
    assert_contains(&qml, "property real deckOptionsEasyBonusValue");
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
    assert_contains(&qml, "property var browserResults");
    assert_contains(&qml, "property var browserResultCardIds");
    assert_contains(&qml, "property string browserSelectedCardId");
    assert_contains(&qml, "property bool answerVisible");
    assert_contains(&qml, "signal reveal()");
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
    assert_contains(&qml, "signal mosaicEvent(var event)");
    assert_contains(
        &qml,
        "onImportAnki: mosaicEvent({ \"event\": \"onImportAnki\" })",
    );
    assert_contains(&qml, "signal exportAnki()");
    assert_contains(&qml, "signal addNote()");
    assert_contains(&qml, "signal addNoteType()");
    assert_contains(&qml, "signal deleteNote()");
    assert_contains(&qml, "signal deleteNoteType()");
    assert_contains(&qml, "signal deckOptionsLearningStepsChange(string value)");
    assert_contains(
        &qml,
        "signal deckOptionsRelearningStepsChange(string value)",
    );
    assert_contains(&qml, "signal deckOptionsNewCardsChange(real value)");
    assert_contains(&qml, "signal deckOptionsMaximumIntervalChange(real value)");
    assert_contains(&qml, "signal deckOptionsEasyBonusChange(real value)");
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

    let swift = fs::read_to_string(tmp.path().join("swiftui").join("EngramApp.swift"))
        .expect("EngramApp.swift");
    assert_contains(&swift, "enum EngramAppEvent {");
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
    assert_contains(&swift, "case deckOptionsLearningStepsChange");
    assert_contains(&swift, "case deckOptionsRelearningStepsChange");
    assert_contains(&swift, "case deckOptionsNewCardsChange");
    assert_contains(&swift, "case deckOptionsMaximumIntervalChange");
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
    assert_contains(&swift, "var mosaicName: String");
    assert_contains(&swift, "var mosaicEnvelope: [String: Any]");
    assert_contains(&swift, "struct EngramAppView: View");
    assert_contains(&swift, "let appTitle: String");
    assert_contains(&swift, "let deckOptionsSettingsLabel: String");
    assert_contains(&swift, "let deckOptionsLearningStepsValue: String");
    assert_contains(&swift, "let deckOptionsRelearningStepsValue: String");
    assert_contains(&swift, "let deckOptionsNewCardsValue: Double");
    assert_contains(&swift, "let deckOptionsEasyBonusValue: Double");
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
    assert_contains(&swift, "let browserResultCardIds: [String]");
    assert_contains(&swift, "let browserSelectedCardId: String");
    assert_contains(&swift, "let answerVisible: Bool");
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
        "browserQuery: MosaicHostValue.string(host.props, \"browser-query\", fallback: \"Sample BrowserQuery\"),",
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
        "actionUndoLabel: MosaicHostValue.string(host.props, \"action-undo-label\", fallback: \"Sample ActionUndoLabel\"),",
    );
    assert_contains(
        &swift_app,
        "actionMarkLabel: MosaicHostValue.string(host.props, \"action-mark-label\", fallback: \"Sample ActionMarkLabel\"),",
    );
    assert_contains(&swift_app, "dispatch: { event in");
    assert_contains(&swift_app, "host.dispatch(event)");
    assert_contains(&swift_app, "final class MosaicHostState: ObservableObject");
    assert_contains(&swift_app, "MosaicHostBridge.load()");
    assert_contains(&swift_app, "@objc protocol MosaicHostBridgeObject");
    assert_contains(&swift_app, "[\"App.MosaicHost\", \"MosaicHost\"]");
    assert_contains(&swift_app, "NSClassFromString(className)");
    let swift_package = fs::read_to_string(tmp.path().join("swiftui").join("Package.swift"))
        .expect("Package.swift");
    assert_contains(&swift_package, "platforms: [.macOS(.v13), .iOS(.v16)]");
    let swift_nested_component = fs::read_to_string(
        tmp.path()
            .join("swiftui")
            .join("Sources")
            .join("App")
            .join("EngramApp.swift"),
    )
    .expect("Sources/App/EngramApp.swift");
    assert_contains(&swift_nested_component, "struct EngramAppView: View");

    let xaml_code_behind = fs::read_to_string(tmp.path().join("xaml").join("EngramApp.xaml.cs"))
        .expect("EngramApp.xaml.cs");
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty AppTitleProperty",
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
        "public static readonly DependencyProperty BrowserResultCardIdsProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty BrowserSelectedCardIdProperty",
    );
    assert_contains(
        &xaml_code_behind,
        "public static readonly DependencyProperty AnswerVisibleProperty",
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
        "public sealed record DeckOptionsEasyBonusChange(double Value) : EngramAppEvent",
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
    let xaml_main_window = fs::read_to_string(tmp.path().join("xaml").join("MainWindow.xaml.cs"))
        .expect("MainWindow.xaml.cs");
    assert_contains(&xaml_main_window, "TryApplyMosaicHostProps");
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
    ];

    for (backend, artifact) in [
        ("HTML", "html/EngramApp.html"),
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
    assert!(
        read_artifact(output_root, "html/EngramApp.html").contains("#0891b2"),
        "CollectionActions dependency style should reach HTML artifact"
    );
    assert!(
        read_artifact(output_root, "html/EngramApp.html").contains("#f59e0b"),
        "DeckOptionsPanel dependency style should reach HTML artifact"
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
    let expected = ["EngramApp.mil", "EngramApp.mll", "EngramApp.dark.msl"];
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
        "host/web/engram-mosaic-host-wasm.d.ts",
        "host/electron/host.js",
        "host/swiftui/MosaicHost.swift",
        "host/xaml/MosaicHost.cs",
    ] {
        let path = package_root().join(relative);
        assert!(
            path.exists(),
            "expected host template missing: {}",
            path.display()
        );
    }

    let xaml_host = fs::read_to_string(package_root().join("host/xaml/MosaicHost.cs"))
        .expect("xaml host template");
    assert_contains(&xaml_host, "eg_engram_app_props");
    assert_contains(&xaml_host, "eg_handle_engram_app_event");
    assert_contains(&xaml_host, "SlotNameToPropertyName");

    let swiftui_host = fs::read_to_string(package_root().join("host/swiftui/MosaicHost.swift"))
        .expect("swiftui host template");
    assert_contains(&swiftui_host, "MosaicHostBridgeObject");
    assert_contains(&swiftui_host, "eg_engram_app_props");
    assert_contains(&swiftui_host, "eg_handle_engram_app_event");

    let build_script =
        fs::read_to_string(package_root().join("scripts/build-all.ps1")).expect("build-all.ps1");
    assert_contains(&build_script, "Install-EngramXamlHost");
    assert_contains(&build_script, "Install-EngramSwiftUIHost");
    assert_contains(&build_script, "module CEngram");
    assert_contains(&build_script, "libengram_capi.a");
    assert_contains(&build_script, "engram_capi.dll");
}
