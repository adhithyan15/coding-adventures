//! # mosaic-ios-project
//!
//! Writes the Xcode project that turns a generated Mosaic SwiftUI package into
//! an iOS and iPadOS **app** (UI89 §2.2).
//!
//! SwiftPM can compile a SwiftUI program for iOS, but it cannot produce an
//! `.app` bundle — only an Xcode project target can. So for every Mosaic
//! package built with a static runtime (`.xcframework`), the artifact builder
//! asks this crate for a `project.pbxproj` and writes it beside the Swift
//! package:
//!
//! ```text
//!   swiftui/
//!     Package.swift                         ← still builds, as before
//!     iOS/App.xcodeproj/project.pbxproj     ← this crate (source root "..")
//!     Sources/App/*.swift                   ← compiled by both
//!     Sources/CMosaicRuntime/CMosaicRuntime.c
//!     Sources/CMosaicRuntime/include/{CMosaicRuntime.h, module.modulemap}
//!     Runtime/MosaicAppRuntime.xcframework  ← the Rust engine, linked
//! ```
//!
//! ## What a `project.pbxproj` is
//!
//! An old-style ("ASCII") property list: one dictionary of *objects*, each a
//! dictionary with an `isa` (its class) and a 24-hex-digit identifier, which
//! other objects refer to. An app needs about a dozen kinds of object:
//!
//! ```text
//!   PBXProject ── mainGroup ──▶ PBXGroup ── children ──▶ PBXFileReference …
//!       │
//!       └── targets ──▶ PBXNativeTarget "App"
//!                         ├── buildPhases: Sources, Frameworks, Resources
//!                         │                  └── files ──▶ PBXBuildFile ──▶ PBXFileReference
//!                         └── buildConfigurationList ──▶ Debug, Release
//! ```
//!
//! Nothing here needs Xcode; the tests read the text back. Identifiers are a
//! hash of each object's role and path, so the same input always produces the
//! same file (regenerating a package shows no diff).
//!
//! ## Trust
//!
//! Paths and names come from a Mosaic package, which is repository content,
//! but the text becomes a build description Xcode executes. Every string is
//! quoted and escaped, and [`project_pbxproj`] refuses control characters,
//! absolute paths and `..` — a path cannot reach outside the project, and a
//! name cannot close a string and inject a build setting.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Everything the project needs to know about one app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IosApp {
    /// The target and product name; the bundle is `<product_name>.app`.
    pub product_name: String,
    /// The name under the icon on the home screen.
    pub display_name: String,
    /// Reverse-DNS identity, for example `dev.codingadventures.trestle`.
    pub bundle_identifier: String,
    /// `CFBundleShortVersionString`, from the package manifest.
    pub marketing_version: String,
    /// The minimum iOS / iPadOS version, for example `16.0`.
    pub deployment_target: String,
    /// Swift sources, relative to the project's directory.
    pub swift_sources: Vec<String>,
    /// C sources, relative to the project's directory.
    pub c_sources: Vec<String>,
    /// Headers to show in the project (not compiled on their own).
    pub headers: Vec<String>,
    /// `HEADER_SEARCH_PATHS` for the C sources.
    pub header_search_paths: Vec<String>,
    /// `SWIFT_INCLUDE_PATHS`: directories holding `module.modulemap` files.
    pub swift_include_paths: Vec<String>,
    /// `GCC_PREPROCESSOR_DEFINITIONS`, for example `MOSAIC_RUNTIME_STATIC=1`.
    pub preprocessor_definitions: Vec<String>,
    /// `.xcframework`s to link.
    pub xcframeworks: Vec<String>,
    /// Where the paths above are relative to, from the directory holding
    /// `App.xcodeproj`: `""` for that directory, `".."` for its parent. The
    /// Mosaic builder puts the project in `iOS/` beside the Swift package, so
    /// that `xcodebuild` run in the package directory still builds the
    /// package, and passes `".."`.
    pub source_root: String,
    /// XCUITest sources (UI89 §4.3), relative to the project's directory.
    /// Empty — the default for every app — means no test target and no
    /// scheme: the project is one app target, as before. Non-empty adds a
    /// UI test bundle that tests the app, and [`shared_scheme`] then
    /// describes the scheme `xcodebuild test` needs.
    pub ui_test_sources: Vec<String>,
    /// Swift compilation conditions defined for the app target's Debug
    /// configuration only, after `$(inherited)` (which brings the
    /// project's `DEBUG`). Release never gets them. Empty — the default —
    /// leaves the app target's settings as they were. The Mosaic builder
    /// passes `MOSAIC_UI_TEST_PICKER` for a build with XCUITests (UI89 §4.4),
    /// so the UI tests' fake document picker is compiled in Debug only.
    pub debug_compilation_conditions: Vec<String>,
}

/// The UI test target's name for an app product: `<product>UITests`.
pub fn ui_test_target_name(app: &IosApp) -> String {
    format!("{}UITests", app.product_name)
}

/// Why a description cannot become a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    /// A name or setting is empty or contains a control character.
    InvalidText { field: &'static str, value: String },
    /// A path is absolute, climbs out with `..`, or is otherwise unusable.
    InvalidPath { field: &'static str, value: String },
    /// The same path is listed twice.
    DuplicatePath(String),
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectError::InvalidText { field, value } => {
                write!(
                    formatter,
                    "{field} is not usable in an Xcode project: {value:?}"
                )
            }
            ProjectError::InvalidPath { field, value } => write!(
                formatter,
                "{field} must be a relative path inside the project: {value:?}"
            ),
            ProjectError::DuplicatePath(path) => write!(formatter, "{path:?} is listed twice"),
        }
    }
}

impl std::error::Error for ProjectError {}

/// The default bundle identifier for a Mosaic package:
/// `dev.codingadventures.` followed by the package name's letters and digits,
/// lowercased (`task-app` → `dev.codingadventures.taskapp`). Bundle
/// identifiers allow only letters, digits, `-` and `.`; dropping everything
/// else keeps any package name valid.
pub fn default_bundle_identifier(package_name: &str) -> String {
    let mut suffix: String = package_name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect();
    if suffix.is_empty() {
        suffix.push_str("app");
    }
    format!("dev.codingadventures.{suffix}")
}

/// A module map that makes a C header importable from Swift as `module`.
/// SwiftPM uses a `module.modulemap` it finds in a target's public headers
/// directory, so the Swift package and the Xcode project see one module.
pub fn module_map(module: &str, header: &str) -> String {
    format!("module {module} {{\n  header \"{header}\"\n  export *\n}}\n")
}

/// The text of `App.xcodeproj/project.pbxproj` for `app`.
pub fn project_pbxproj(app: &IosApp) -> Result<String, ProjectError> {
    validate(app)?;
    Ok(Builder::new(app).build())
}

// --------------------------------------------------------------------------
// Validation
// --------------------------------------------------------------------------

fn validate(app: &IosApp) -> Result<(), ProjectError> {
    for (field, value) in [
        ("product_name", &app.product_name),
        ("display_name", &app.display_name),
        ("bundle_identifier", &app.bundle_identifier),
        ("marketing_version", &app.marketing_version),
        ("deployment_target", &app.deployment_target),
    ] {
        check_text(field, value)?;
    }
    if !app
        .bundle_identifier
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '.'))
    {
        return Err(ProjectError::InvalidText {
            field: "bundle_identifier",
            value: app.bundle_identifier.clone(),
        });
    }
    if !app
        .product_name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(ProjectError::InvalidText {
            field: "product_name",
            value: app.product_name.clone(),
        });
    }
    if !matches!(app.source_root.as_str(), "" | "..") {
        return Err(ProjectError::InvalidPath {
            field: "source_root",
            value: app.source_root.clone(),
        });
    }
    for definition in &app.preprocessor_definitions {
        check_text("preprocessor_definitions", definition)?;
    }
    // A Swift compilation condition is an identifier; anything else would
    // be a different setting, or none, by the time Xcode splits the list.
    for condition in &app.debug_compilation_conditions {
        let mut characters = condition.chars();
        let identifier = characters
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
            && characters.all(|rest| rest.is_ascii_alphanumeric() || rest == '_');
        if !identifier {
            return Err(ProjectError::InvalidText {
                field: "debug_compilation_conditions",
                value: condition.clone(),
            });
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    for (field, paths) in [
        ("swift_sources", &app.swift_sources),
        ("c_sources", &app.c_sources),
        ("headers", &app.headers),
        ("xcframeworks", &app.xcframeworks),
        ("ui_test_sources", &app.ui_test_sources),
    ] {
        for path in paths {
            check_path(field, path)?;
            if !seen.insert(path.as_str()) {
                return Err(ProjectError::DuplicatePath(path.clone()));
            }
        }
    }
    for (field, paths) in [
        ("header_search_paths", &app.header_search_paths),
        ("swift_include_paths", &app.swift_include_paths),
    ] {
        for path in paths {
            check_path(field, path)?;
        }
    }
    Ok(())
}

fn check_text(field: &'static str, value: &str) -> Result<(), ProjectError> {
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(ProjectError::InvalidText {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn check_path(field: &'static str, path: &str) -> Result<(), ProjectError> {
    let invalid = || ProjectError::InvalidPath {
        field,
        value: path.to_string(),
    };
    check_text(field, path).map_err(|_| invalid())?;
    // Xcode expands `$(…)` in paths; a package path has no business doing so.
    if path.starts_with('/')
        || path.starts_with('~')
        || path.contains('\\')
        || path.contains("$(")
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(invalid());
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Generation
// --------------------------------------------------------------------------

/// A 24-hex-digit object identifier derived from `role`: two FNV-1a hashes
/// with different offsets, 48 bits each. Stable across runs and machines.
fn object_id(role: &str) -> String {
    fn fnv1a(bytes: &[u8], offset: u64) -> u64 {
        bytes.iter().fold(offset, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }
    let high = fnv1a(role.as_bytes(), 0xcbf2_9ce4_8422_2325) & 0xffff_ffff_ffff;
    let low = fnv1a(role.as_bytes(), 0x84222325_cbf29ce4) & 0xffff_ffff_ffff;
    format!("{high:012X}{low:012X}")
}

/// An ASCII property-list string: always quoted, `\` and `"` escaped.
/// (Control characters were refused by validation.)
fn quote(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for character in value.chars() {
        if matches!(character, '"' | '\\') {
            quoted.push('\\');
        }
        quoted.push(character);
    }
    quoted.push('"');
    quoted
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// One value in a build-settings dictionary.
enum Setting {
    Text(String),
    List(Vec<String>),
}

impl Setting {
    fn render(&self) -> String {
        match self {
            Setting::Text(value) => quote(value),
            Setting::List(values) => {
                let items: Vec<String> = values.iter().map(|value| quote(value)).collect();
                format!("({})", items.join(", "))
            }
        }
    }
}

fn text(value: impl Into<String>) -> Setting {
    Setting::Text(value.into())
}

struct Builder<'a> {
    app: &'a IosApp,
    /// Rendered objects by section (`isa`), each a list of `(id, body)`.
    sections: BTreeMap<&'static str, Vec<(String, String)>>,
}

impl<'a> Builder<'a> {
    fn new(app: &'a IosApp) -> Self {
        Self {
            app,
            sections: BTreeMap::new(),
        }
    }

    fn add(&mut self, isa: &'static str, id: &str, fields: &[(&str, String)]) {
        let mut body = format!("{{isa = {isa}; ");
        for (key, value) in fields {
            let _ = write!(body, "{key} = {value}; ");
        }
        body.push('}');
        self.sections
            .entry(isa)
            .or_default()
            .push((id.to_string(), body));
    }

    fn file_reference(&mut self, path: &str, file_type: &str) -> String {
        let id = object_id(&format!("file:{path}"));
        self.add(
            "PBXFileReference",
            &id,
            &[
                ("lastKnownFileType", file_type.to_string()),
                ("name", quote(file_name(path))),
                ("path", quote(path)),
                ("sourceTree", "SOURCE_ROOT".to_string()),
            ],
        );
        id
    }

    fn build_file(&mut self, phase: &str, path: &str, file_ref: &str) -> String {
        let id = object_id(&format!("build:{phase}:{path}"));
        self.add("PBXBuildFile", &id, &[("fileRef", file_ref.to_string())]);
        id
    }

    fn configuration(&mut self, role: &str, name: &str, settings: &[(&str, Setting)]) -> String {
        let id = object_id(&format!("configuration:{role}:{name}"));
        let mut rendered = String::from("{");
        for (key, value) in settings {
            let _ = write!(rendered, "{key} = {}; ", value.render());
        }
        rendered.push('}');
        self.add(
            "XCBuildConfiguration",
            &id,
            &[("buildSettings", rendered), ("name", quote(name))],
        );
        id
    }

    fn configuration_list(&mut self, role: &str, debug: &str, release: &str) -> String {
        let id = object_id(&format!("configuration-list:{role}"));
        self.add(
            "XCConfigurationList",
            &id,
            &[
                ("buildConfigurations", format!("({debug}, {release})")),
                ("defaultConfigurationIsVisible", "0".to_string()),
                ("defaultConfigurationName", "Release".to_string()),
            ],
        );
        id
    }

    /// The XCUITest bundle (UI89 §4.3): its own phases and configurations,
    /// `TEST_TARGET_NAME` naming the app it drives, and a dependency on the
    /// app so `xcodebuild test` builds the app first. Returns its id.
    fn ui_test_target(
        &mut self,
        project: &str,
        app_target: &str,
        product: &str,
        sources: Vec<String>,
    ) -> String {
        let app = self.app;
        let name = ui_test_target_name(app);
        let sources_phase = object_id("phase:ui-test-sources");
        self.add(
            "PBXSourcesBuildPhase",
            &sources_phase,
            &[
                ("buildActionMask", "2147483647".to_string()),
                ("files", format!("({})", sources.join(", "))),
                ("runOnlyForDeploymentPostprocessing", "0".to_string()),
            ],
        );
        let frameworks_phase = object_id("phase:ui-test-frameworks");
        self.add(
            "PBXFrameworksBuildPhase",
            &frameworks_phase,
            &[
                ("buildActionMask", "2147483647".to_string()),
                ("files", "()".to_string()),
                ("runOnlyForDeploymentPostprocessing", "0".to_string()),
            ],
        );
        let resources_phase = object_id("phase:ui-test-resources");
        self.add(
            "PBXResourcesBuildPhase",
            &resources_phase,
            &[
                ("buildActionMask", "2147483647".to_string()),
                ("files", "()".to_string()),
                ("runOnlyForDeploymentPostprocessing", "0".to_string()),
            ],
        );
        let proxy = object_id("proxy:ui-tests-app");
        self.add(
            "PBXContainerItemProxy",
            &proxy,
            &[
                ("containerPortal", project.to_string()),
                ("proxyType", "1".to_string()),
                ("remoteGlobalIDString", app_target.to_string()),
                ("remoteInfo", quote(&app.product_name)),
            ],
        );
        let dependency = object_id("dependency:ui-tests-app");
        self.add(
            "PBXTargetDependency",
            &dependency,
            &[("target", app_target.to_string()), ("targetProxy", proxy)],
        );
        let debug = self.configuration("ui-test-target", "Debug", &ui_test_settings(app));
        let release = self.configuration("ui-test-target", "Release", &ui_test_settings(app));
        let list = self.configuration_list("ui-test-target", &debug, &release);
        let target = object_id("target:ui-tests");
        self.add(
            "PBXNativeTarget",
            &target,
            &[
                ("buildConfigurationList", list),
                (
                    "buildPhases",
                    format!("({sources_phase}, {frameworks_phase}, {resources_phase})"),
                ),
                ("buildRules", "()".to_string()),
                ("dependencies", format!("({dependency})")),
                ("name", quote(&name)),
                ("productName", quote(&name)),
                ("productReference", product.to_string()),
                (
                    "productType",
                    quote("com.apple.product-type.bundle.ui-testing"),
                ),
            ],
        );
        target
    }

    fn build(mut self) -> String {
        let app = self.app;

        // Files, and the build files that put them in a phase.
        let mut group_children = Vec::new();
        let mut sources = Vec::new();
        for path in &app.swift_sources {
            let file = self.file_reference(path, "sourcecode.swift");
            group_children.push(file.clone());
            sources.push(self.build_file("sources", path, &file));
        }
        for path in &app.c_sources {
            let file = self.file_reference(path, "sourcecode.c.c");
            group_children.push(file.clone());
            sources.push(self.build_file("sources", path, &file));
        }
        for path in &app.headers {
            group_children.push(self.file_reference(path, "sourcecode.c.h"));
        }
        let mut frameworks = Vec::new();
        for path in &app.xcframeworks {
            let file = self.file_reference(path, "wrapper.xcframework");
            group_children.push(file.clone());
            frameworks.push(self.build_file("frameworks", path, &file));
        }
        // The UI tests' sources go in their own target's phase, never the
        // app's: test code must not ship in the app.
        let mut ui_test_sources = Vec::new();
        for path in &app.ui_test_sources {
            let file = self.file_reference(path, "sourcecode.swift");
            group_children.push(file.clone());
            ui_test_sources.push(self.build_file("ui-test-sources", path, &file));
        }
        let has_ui_tests = !ui_test_sources.is_empty();
        let ui_test_name = ui_test_target_name(app);

        // The product, and the groups the project navigator shows.
        let product = object_id("product");
        self.add(
            "PBXFileReference",
            &product,
            &[
                ("explicitFileType", "wrapper.application".to_string()),
                ("includeInIndex", "0".to_string()),
                ("path", quote(&format!("{}.app", app.product_name))),
                ("sourceTree", "BUILT_PRODUCTS_DIR".to_string()),
            ],
        );
        let ui_test_product = object_id("product:ui-tests");
        let mut products = vec![product.clone()];
        if has_ui_tests {
            self.add(
                "PBXFileReference",
                &ui_test_product,
                &[
                    ("explicitFileType", "wrapper.cfbundle".to_string()),
                    ("includeInIndex", "0".to_string()),
                    ("path", quote(&format!("{ui_test_name}.xctest"))),
                    ("sourceTree", "BUILT_PRODUCTS_DIR".to_string()),
                ],
            );
            products.push(ui_test_product.clone());
        }
        let products_group = object_id("group:products");
        self.add(
            "PBXGroup",
            &products_group,
            &[
                ("children", format!("({})", products.join(", "))),
                ("name", quote("Products")),
                ("sourceTree", quote("<group>")),
            ],
        );
        group_children.push(products_group.clone());
        let main_group = object_id("group:main");
        self.add(
            "PBXGroup",
            &main_group,
            &[
                ("children", format!("({})", group_children.join(", "))),
                ("sourceTree", quote("<group>")),
            ],
        );

        // Build phases.
        let sources_phase = object_id("phase:sources");
        self.add(
            "PBXSourcesBuildPhase",
            &sources_phase,
            &[
                ("buildActionMask", "2147483647".to_string()),
                ("files", format!("({})", sources.join(", "))),
                ("runOnlyForDeploymentPostprocessing", "0".to_string()),
            ],
        );
        let frameworks_phase = object_id("phase:frameworks");
        self.add(
            "PBXFrameworksBuildPhase",
            &frameworks_phase,
            &[
                ("buildActionMask", "2147483647".to_string()),
                ("files", format!("({})", frameworks.join(", "))),
                ("runOnlyForDeploymentPostprocessing", "0".to_string()),
            ],
        );
        let resources_phase = object_id("phase:resources");
        self.add(
            "PBXResourcesBuildPhase",
            &resources_phase,
            &[
                ("buildActionMask", "2147483647".to_string()),
                ("files", "()".to_string()),
                ("runOnlyForDeploymentPostprocessing", "0".to_string()),
            ],
        );

        // Configurations: the project's (shared) and the target's (the app).
        let project_debug = self.configuration("project", "Debug", &project_settings(app, true));
        let project_release =
            self.configuration("project", "Release", &project_settings(app, false));
        let project_list = self.configuration_list("project", &project_debug, &project_release);
        let target_debug = self.configuration("target", "Debug", &target_settings(app, true));
        let target_release = self.configuration("target", "Release", &target_settings(app, false));
        let target_list = self.configuration_list("target", &target_debug, &target_release);

        let target = object_id("target:app");
        self.add(
            "PBXNativeTarget",
            &target,
            &[
                ("buildConfigurationList", target_list),
                (
                    "buildPhases",
                    format!("({sources_phase}, {frameworks_phase}, {resources_phase})"),
                ),
                ("buildRules", "()".to_string()),
                ("dependencies", "()".to_string()),
                ("name", quote(&app.product_name)),
                ("productName", quote(&app.product_name)),
                ("productReference", product.clone()),
                ("productType", quote("com.apple.product-type.application")),
            ],
        );

        let project = object_id("project");
        let mut targets = vec![target.clone()];
        let mut target_attributes = String::new();
        if has_ui_tests {
            let ui_test_target =
                self.ui_test_target(&project, &target, &ui_test_product, ui_test_sources);
            target_attributes = format!(
                " TargetAttributes = {{{ui_test_target} = {{TestTargetID = {target}; }}; }};"
            );
            targets.push(ui_test_target);
        }
        self.add(
            "PBXProject",
            &project,
            &[
                (
                    "attributes",
                    format!(
                        "{{BuildIndependentTargetsInParallel = 1; LastUpgradeCheck = 1500;{target_attributes} }}"
                    ),
                ),
                ("buildConfigurationList", project_list),
                ("compatibilityVersion", quote("Xcode 14.0")),
                ("developmentRegion", "en".to_string()),
                ("hasScannedForEncodings", "0".to_string()),
                ("knownRegions", "(en, Base)".to_string()),
                ("mainGroup", main_group),
                ("productRefGroup", products_group),
                ("projectDirPath", quote(&app.source_root)),
                ("projectRoot", quote("")),
                ("targets", format!("({})", targets.join(", "))),
            ],
        );

        let mut out = String::from(
            "// !$*UTF8*$!\n// Generated by mosaic-ios-project (UI89 §2.2). Do not edit; regenerate.\n{\n\tarchiveVersion = 1;\n\tclasses = {\n\t};\n\tobjectVersion = 56;\n\tobjects = {\n",
        );
        for (isa, objects) in &self.sections {
            let _ = write!(out, "\n/* Begin {isa} section */\n");
            let mut objects = objects.clone();
            objects.sort();
            for (id, body) in objects {
                let _ = writeln!(out, "\t\t{id} = {body};");
            }
            let _ = writeln!(out, "/* End {isa} section */");
        }
        let _ = write!(out, "\t}};\n\trootObject = {project};\n}}\n");
        out
    }
}

fn project_settings(app: &IosApp, debug: bool) -> Vec<(&'static str, Setting)> {
    let mut settings = vec![
        ("ALWAYS_SEARCH_USER_PATHS", text("NO")),
        ("CLANG_ENABLE_MODULES", text("YES")),
        ("CLANG_ENABLE_OBJC_ARC", text("YES")),
        (
            "IPHONEOS_DEPLOYMENT_TARGET",
            text(app.deployment_target.clone()),
        ),
        ("SDKROOT", text("iphoneos")),
        ("SWIFT_VERSION", text("5.0")),
    ];
    if debug {
        settings.extend([
            ("DEBUG_INFORMATION_FORMAT", text("dwarf")),
            ("ENABLE_TESTABILITY", text("YES")),
            ("GCC_OPTIMIZATION_LEVEL", text("0")),
            ("ONLY_ACTIVE_ARCH", text("YES")),
            ("SWIFT_ACTIVE_COMPILATION_CONDITIONS", text("DEBUG")),
            ("SWIFT_OPTIMIZATION_LEVEL", text("-Onone")),
        ]);
    } else {
        settings.extend([
            ("DEBUG_INFORMATION_FORMAT", text("dwarf-with-dsym")),
            ("SWIFT_COMPILATION_MODE", text("wholemodule")),
            ("VALIDATE_PRODUCT", text("YES")),
        ]);
    }
    settings
}

fn target_settings(app: &IosApp, debug: bool) -> Vec<(&'static str, Setting)> {
    let every_orientation = "UIInterfaceOrientationPortrait UIInterfaceOrientationPortraitUpsideDown UIInterfaceOrientationLandscapeLeft UIInterfaceOrientationLandscapeRight";
    let mut definitions = vec!["$(inherited)".to_string()];
    definitions.extend(app.preprocessor_definitions.iter().cloned());
    let with_inherited = |paths: &[String]| {
        let mut all = vec!["$(inherited)".to_string()];
        all.extend(paths.iter().map(|path| format!("$(SRCROOT)/{path}")));
        Setting::List(all)
    };
    let mut settings = vec![
        ("CODE_SIGN_STYLE", text("Automatic")),
        ("CURRENT_PROJECT_VERSION", text("1")),
        ("GCC_PREPROCESSOR_DEFINITIONS", Setting::List(definitions)),
        ("GENERATE_INFOPLIST_FILE", text("YES")),
        (
            "HEADER_SEARCH_PATHS",
            with_inherited(&app.header_search_paths),
        ),
        (
            "INFOPLIST_KEY_CFBundleDisplayName",
            text(app.display_name.clone()),
        ),
        (
            "INFOPLIST_KEY_UIApplicationSceneManifest_Generation",
            text("YES"),
        ),
        (
            "INFOPLIST_KEY_UIApplicationSupportsIndirectInputEvents",
            text("YES"),
        ),
        ("INFOPLIST_KEY_UILaunchScreen_Generation", text("YES")),
        (
            "INFOPLIST_KEY_UISupportedInterfaceOrientations_iPad",
            text(every_orientation),
        ),
        (
            "INFOPLIST_KEY_UISupportedInterfaceOrientations_iPhone",
            text(every_orientation),
        ),
        (
            "LD_RUNPATH_SEARCH_PATHS",
            Setting::List(vec![
                "$(inherited)".to_string(),
                "@executable_path/Frameworks".to_string(),
            ]),
        ),
        ("MARKETING_VERSION", text(app.marketing_version.clone())),
        (
            "PRODUCT_BUNDLE_IDENTIFIER",
            text(app.bundle_identifier.clone()),
        ),
        ("PRODUCT_NAME", text(app.product_name.clone())),
        // Also set on the project; repeated on each target so a scheme's
        // destinations resolve from the target itself (UI89 §4.3).
        ("SDKROOT", text("iphoneos")),
        ("SUPPORTED_PLATFORMS", text("iphoneos iphonesimulator")),
        ("SUPPORTS_MACCATALYST", text("NO")),
        ("SWIFT_EMIT_LOC_STRINGS", text("NO")),
        (
            "SWIFT_INCLUDE_PATHS",
            with_inherited(&app.swift_include_paths),
        ),
        ("TARGETED_DEVICE_FAMILY", text("1,2")),
    ];
    // Debug only, and only when asked for: the setting is absent otherwise,
    // so the app target inherits the project's `DEBUG` exactly as before.
    if debug && !app.debug_compilation_conditions.is_empty() {
        let mut conditions = vec!["$(inherited)".to_string()];
        conditions.extend(app.debug_compilation_conditions.iter().cloned());
        let at = settings
            .iter()
            .position(|(key, _)| *key > "SWIFT_ACTIVE_COMPILATION_CONDITIONS")
            .unwrap_or(settings.len());
        settings.insert(
            at,
            (
                "SWIFT_ACTIVE_COMPILATION_CONDITIONS",
                Setting::List(conditions),
            ),
        );
    }
    settings
}

/// The UI test bundle's settings. It has its own bundle identifier (the
/// app's plus `.uitests`) and names the app it launches in
/// `TEST_TARGET_NAME`. It links nothing of the app's: an XCUITest drives the
/// installed app from outside, through accessibility.
fn ui_test_settings(app: &IosApp) -> Vec<(&'static str, Setting)> {
    vec![
        ("CODE_SIGN_STYLE", text("Automatic")),
        ("CURRENT_PROJECT_VERSION", text("1")),
        ("GENERATE_INFOPLIST_FILE", text("YES")),
        ("MARKETING_VERSION", text(app.marketing_version.clone())),
        (
            "PRODUCT_BUNDLE_IDENTIFIER",
            text(format!("{}.uitests", app.bundle_identifier)),
        ),
        ("PRODUCT_NAME", text(ui_test_target_name(app))),
        ("SDKROOT", text("iphoneos")),
        ("SUPPORTED_PLATFORMS", text("iphoneos iphonesimulator")),
        ("SWIFT_EMIT_LOC_STRINGS", text("NO")),
        ("TARGETED_DEVICE_FAMILY", text("1,2")),
        ("TEST_TARGET_NAME", text(app.product_name.clone())),
    ]
}

// --------------------------------------------------------------------------
// The shared scheme
// --------------------------------------------------------------------------

/// The scheme for an app with UI tests (UI89 §4.3), or `None` without them.
/// `xcodebuild test` needs a scheme, and Xcode creates one only when the
/// project is opened in its UI, so the builder writes it. The scheme builds
/// the app, runs it, and tests it with the UI test bundle. It is named
/// [`ui_test_target_name`] (`AppUITests`), so it can never be confused with a
/// scheme Xcode creates for the app target itself.
///
/// It belongs in a WORKSPACE beside the project
/// (`App.xcworkspace/xcshareddata/xcschemes/AppUITests.xcscheme`, with
/// [`workspace_contents`]), not inside the `.xcodeproj`. A project's own
/// schemes resolve `container:` against the project's directory, and this
/// project's directory is `..` (`source_root`), where no `App.xcodeproj`
/// exists. Two CI runs saw every scheme in the project, Xcode's own
/// included, resolve to no buildables ("Supported platforms for the
/// buildables in the current scheme is empty"). A workspace resolves
/// `container:` against its own folder, which is the project's folder.
///
/// The file is exactly what Xcode itself writes: the XML declaration, then
/// the `<Scheme>` root. It carries no provenance comment, unlike the
/// `.pbxproj`: the first CI run with one (and its `§`) before the root saw
/// `xcodebuild test` find no buildables in the scheme at all.
///
/// Every attribute value is XML-escaped, although validation already limits
/// the product name to letters, digits, `-` and `_`. `project_file` is the
/// `.xcodeproj` directory's name, which the scheme refers to as its container.
pub fn shared_scheme(app: &IosApp, project_file: &str) -> Result<Option<String>, ProjectError> {
    // (The workspace that holds this scheme is `workspace_contents`.)
    validate(app)?;
    check_text("project_file", project_file)?;
    if app.ui_test_sources.is_empty() {
        return Ok(None);
    }
    let reference = |blueprint: &str, buildable: &str, name: &str| {
        format!(
            "<BuildableReference BuildableIdentifier = \"primary\" BlueprintIdentifier = \"{}\" BuildableName = \"{}\" BlueprintName = \"{}\" ReferencedContainer = \"container:{}\">\n            </BuildableReference>",
            xml_escape(blueprint),
            xml_escape(buildable),
            xml_escape(name),
            xml_escape(project_file),
        )
    };
    let app_reference = reference(
        &object_id("target:app"),
        &format!("{}.app", app.product_name),
        &app.product_name,
    );
    let tests = ui_test_target_name(app);
    let test_reference = reference(
        &object_id("target:ui-tests"),
        &format!("{tests}.xctest"),
        &tests,
    );
    Ok(Some(format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<Scheme LastUpgradeVersion = \"1500\" version = \"1.7\">\n",
            "   <BuildAction parallelizeBuildables = \"YES\" buildImplicitDependencies = \"YES\">\n",
            "      <BuildActionEntries>\n",
            "         <BuildActionEntry buildForTesting = \"YES\" buildForRunning = \"YES\" buildForProfiling = \"YES\" buildForArchiving = \"YES\" buildForAnalyzing = \"YES\">\n",
            "            {app}\n",
            "         </BuildActionEntry>\n",
            "      </BuildActionEntries>\n",
            "   </BuildAction>\n",
            "   <TestAction buildConfiguration = \"Debug\" selectedDebuggerIdentifier = \"Xcode.DebuggerFoundation.Debugger.LLDB\" selectedLauncherIdentifier = \"Xcode.DebuggerFoundation.Launcher.LLDB\" shouldUseLaunchSchemeArgsEnv = \"YES\">\n",
            "      <Testables>\n",
            "         <TestableReference skipped = \"NO\">\n",
            "            {tests}\n",
            "         </TestableReference>\n",
            "      </Testables>\n",
            "   </TestAction>\n",
            "   <LaunchAction buildConfiguration = \"Debug\" selectedDebuggerIdentifier = \"Xcode.DebuggerFoundation.Debugger.LLDB\" selectedLauncherIdentifier = \"Xcode.DebuggerFoundation.Launcher.LLDB\" launchStyle = \"0\" useCustomWorkingDirectory = \"NO\" ignoresPersistentStateOnLaunch = \"NO\" debugDocumentVersioning = \"YES\" debugServiceExtension = \"internal\" allowLocationSimulation = \"YES\">\n",
            "      <BuildableProductRunnable runnableDebuggingMode = \"0\">\n",
            "            {app}\n",
            "      </BuildableProductRunnable>\n",
            "   </LaunchAction>\n",
            "</Scheme>\n",
        ),
        app = app_reference,
        tests = test_reference,
    )))
}

/// `contents.xcworkspacedata` for a workspace that holds only `project_file`
/// (the `.xcodeproj` beside it), so a scheme in the workspace can refer to the
/// project by a path relative to the workspace's folder. See [`shared_scheme`].
pub fn workspace_contents(project_file: &str) -> Result<String, ProjectError> {
    check_text("project_file", project_file)?;
    Ok(format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<Workspace\n",
            "   version = \"1.0\">\n",
            "   <FileRef\n",
            "      location = \"group:{}\">\n",
            "   </FileRef>\n",
            "</Workspace>\n",
        ),
        xml_escape(project_file)
    ))
}

/// An XML attribute value with `&`, `<`, `>`, `"` and `'` escaped.
fn xml_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trestle() -> IosApp {
        IosApp {
            product_name: "App".into(),
            display_name: "Trestle".into(),
            bundle_identifier: default_bundle_identifier("task-app"),
            marketing_version: "0.1.0".into(),
            deployment_target: "16.0".into(),
            swift_sources: vec![
                "Sources/App/App.swift".into(),
                "Sources/App/MosaicRuntimeHost.swift".into(),
            ],
            c_sources: vec!["Sources/CMosaicRuntime/CMosaicRuntime.c".into()],
            headers: vec!["Sources/CMosaicRuntime/include/CMosaicRuntime.h".into()],
            header_search_paths: vec!["Sources/CMosaicRuntime/include".into()],
            swift_include_paths: vec!["Sources/CMosaicRuntime/include".into()],
            preprocessor_definitions: vec!["MOSAIC_RUNTIME_STATIC=1".into()],
            xcframeworks: vec!["Runtime/MosaicAppRuntime.xcframework".into()],
            source_root: String::new(),
            ui_test_sources: Vec::new(),
            debug_compilation_conditions: Vec::new(),
        }
    }

    fn with_ui_tests() -> IosApp {
        IosApp {
            ui_test_sources: vec!["UITests/JournalUiTests.swift".into()],
            ..trestle()
        }
    }

    #[test]
    fn without_ui_tests_there_is_one_target_and_no_scheme() {
        let app = trestle();
        let project = project_pbxproj(&app).unwrap();
        assert_eq!(project.matches("isa = PBXNativeTarget;").count(), 1);
        assert!(!project.contains("ui-testing"), "{project}");
        assert!(!project.contains("TargetAttributes"), "{project}");
        assert!(!project.contains("PBXTargetDependency"), "{project}");
        assert_eq!(shared_scheme(&app, "App.xcodeproj").unwrap(), None);
    }

    #[test]
    fn ui_tests_add_a_test_bundle_that_depends_on_the_app() {
        let app = with_ui_tests();
        let project = project_pbxproj(&app).unwrap();
        let objects = objects(&project);
        let app_target = object_id("target:app");
        let test_target = object_id("target:ui-tests");
        assert_eq!(project.matches("isa = PBXNativeTarget;").count(), 2);

        let tests = &objects[&test_target];
        assert!(
            tests.contains("productType = \"com.apple.product-type.bundle.ui-testing\""),
            "{tests}"
        );
        assert!(tests.contains("name = \"AppUITests\""), "{tests}");
        let dependency = object_id("dependency:ui-tests-app");
        assert!(
            tests.contains(&format!("dependencies = ({dependency})")),
            "{tests}"
        );
        assert!(objects[&dependency].contains(&format!("target = {app_target}")));
        let proxy = &objects[&object_id("proxy:ui-tests-app")];
        assert!(
            proxy.contains(&format!("remoteGlobalIDString = {app_target}")),
            "{proxy}"
        );
        assert!(
            proxy.contains(&format!("containerPortal = {}", object_id("project"))),
            "{proxy}"
        );

        // The test source is compiled by the test target only.
        let test_build_file = object_id("build:ui-test-sources:UITests/JournalUiTests.swift");
        assert!(objects[&object_id("phase:ui-test-sources")].contains(&test_build_file));
        assert!(!objects[&object_id("phase:sources")].contains(&test_build_file));
        assert!(!project.contains("build:sources:UITests"), "{project}");

        // The test bundle names the app it drives, and has its own identity.
        assert!(project.contains("TEST_TARGET_NAME = \"App\";"), "{project}");
        assert!(
            project
                .contains("PRODUCT_BUNDLE_IDENTIFIER = \"dev.codingadventures.taskapp.uitests\";"),
            "{project}"
        );
        assert!(
            project.contains("path = \"AppUITests.xctest\""),
            "{project}"
        );
        let root = &objects[&object_id("project")];
        assert!(
            root.contains(&format!("targets = ({app_target}, {test_target})")),
            "{root}"
        );
        assert!(
            root.contains(&format!(
                "TargetAttributes = {{{test_target} = {{TestTargetID = {app_target}; }}; }};"
            )),
            "{root}"
        );
    }

    #[test]
    fn the_shared_scheme_builds_runs_and_tests_the_app() {
        let scheme = shared_scheme(&with_ui_tests(), "App.xcodeproj")
            .unwrap()
            .unwrap();
        // Exactly Xcode's shape: the declaration, then the root, ASCII only.
        assert!(scheme.starts_with(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Scheme LastUpgradeVersion = \"1500\" version = \"1.7\">\n"
        ));
        assert!(scheme.is_ascii() && !scheme.contains("<!--"), "{scheme}");
        let app = format!("BlueprintIdentifier = \"{}\"", object_id("target:app"));
        let tests = format!("BlueprintIdentifier = \"{}\"", object_id("target:ui-tests"));
        // The app is built and launched; the bundle is the one testable.
        assert_eq!(scheme.matches(&app).count(), 2, "{scheme}");
        assert_eq!(scheme.matches(&tests).count(), 1, "{scheme}");
        let testables =
            &scheme[scheme.find("<Testables>").unwrap()..scheme.find("</Testables>").unwrap()];
        assert!(
            testables.contains(&tests)
                && testables.contains("BuildableName = \"AppUITests.xctest\"")
        );
        assert!(scheme.contains("BuildableName = \"App.app\""));
        assert!(scheme.contains("ReferencedContainer = \"container:App.xcodeproj\""));
        // Balanced, and nothing unescaped in attribute values.
        assert_eq!(
            scheme.matches("<BuildableReference ").count(),
            scheme.matches("</BuildableReference>").count()
        );
    }

    #[test]
    fn the_workspace_holds_only_the_project_beside_it() {
        let workspace = workspace_contents("App.xcodeproj").unwrap();
        assert_eq!(
            workspace,
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Workspace\n   version = \"1.0\">\n   <FileRef\n      location = \"group:App.xcodeproj\">\n   </FileRef>\n</Workspace>\n"
        );
        assert!(workspace_contents("A&B.xcodeproj")
            .unwrap()
            .contains("group:A&amp;B.xcodeproj"));
        assert!(workspace_contents("").is_err());
    }

    /// The XCBuildConfiguration lines of the app target (the ones naming its
    /// bundle identifier), as (name, line).
    fn app_configurations(project: &str) -> Vec<(&str, &str)> {
        project
            .lines()
            .filter(|line| {
                line.contains("isa = XCBuildConfiguration")
                    && line.contains("PRODUCT_BUNDLE_IDENTIFIER = \"dev.codingadventures.taskapp\"")
            })
            .map(|line| {
                let name = if line.contains("name = \"Debug\"") {
                    "Debug"
                } else {
                    "Release"
                };
                (name, line)
            })
            .collect()
    }

    #[test]
    fn debug_compilation_conditions_reach_the_app_target_in_debug_only() {
        // None by default: the app target has no setting of its own and
        // inherits the project's DEBUG, exactly as before.
        let plain = project_pbxproj(&trestle()).unwrap();
        for (_, line) in app_configurations(&plain) {
            assert!(
                !line.contains("SWIFT_ACTIVE_COMPILATION_CONDITIONS"),
                "{line}"
            );
        }

        let mut app = trestle();
        app.debug_compilation_conditions = vec!["MOSAIC_UI_TEST_PICKER".to_string()];
        let project = project_pbxproj(&app).unwrap();
        let configurations = app_configurations(&project);
        assert_eq!(configurations.len(), 2, "{project}");
        for (name, line) in configurations {
            if name == "Debug" {
                assert!(
                    line.contains(
                        "SWIFT_ACTIVE_COMPILATION_CONDITIONS = (\"$(inherited)\", \"MOSAIC_UI_TEST_PICKER\");"
                    ),
                    "{line}"
                );
            } else {
                assert!(!line.contains("MOSAIC_UI_TEST_PICKER"), "{line}");
            }
        }

        for bad in ["", "1ST", "A B", "A;B", "A=1", "$(X)"] {
            app.debug_compilation_conditions = vec![bad.to_string()];
            assert!(project_pbxproj(&app).is_err(), "{bad:?} was accepted");
        }
    }

    #[test]
    fn scheme_attribute_values_are_escaped() {
        assert_eq!(xml_escape("a&b<c>\"d'"), "a&amp;b&lt;c&gt;&quot;d&apos;");
        let scheme = shared_scheme(&with_ui_tests(), "A&\"B.xcodeproj")
            .unwrap()
            .unwrap();
        assert!(
            scheme.contains("container:A&amp;&quot;B.xcodeproj"),
            "{scheme}"
        );
        assert!(!scheme.contains("A&\"B"), "{scheme}");
    }

    #[test]
    fn ui_test_sources_are_checked_like_every_other_path() {
        for bad in [
            "/abs/T.swift",
            "../T.swift",
            "UITests/../T.swift",
            "$(SRCROOT)/T.swift",
        ] {
            let app = IosApp {
                ui_test_sources: vec![bad.into()],
                ..trestle()
            };
            assert!(
                matches!(
                    project_pbxproj(&app),
                    Err(ProjectError::InvalidPath {
                        field: "ui_test_sources",
                        ..
                    })
                ),
                "{bad}"
            );
            assert!(shared_scheme(&app, "App.xcodeproj").is_err(), "{bad}");
        }
        let duplicate = IosApp {
            ui_test_sources: vec!["Sources/App/App.swift".into()],
            ..trestle()
        };
        assert!(matches!(
            project_pbxproj(&duplicate),
            Err(ProjectError::DuplicatePath(_))
        ));
    }

    /// The objects of a project, as `id -> body`, parsed back from the text.
    fn objects(project: &str) -> BTreeMap<String, String> {
        project
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let (id, body) = line.split_once(" = {isa = ")?;
                Some((id.to_string(), body.to_string()))
            })
            .collect()
    }

    #[test]
    fn default_bundle_identifiers_keep_letters_and_digits() {
        assert_eq!(
            default_bundle_identifier("task-app"),
            "dev.codingadventures.taskapp"
        );
        assert_eq!(
            default_bundle_identifier("Engram 2"),
            "dev.codingadventures.engram2"
        );
        assert_eq!(default_bundle_identifier("---"), "dev.codingadventures.app");
    }

    #[test]
    fn module_map_names_the_header() {
        assert_eq!(
            module_map("CMosaicRuntime", "CMosaicRuntime.h"),
            "module CMosaicRuntime {\n  header \"CMosaicRuntime.h\"\n  export *\n}\n"
        );
    }

    #[test]
    fn project_has_one_app_target_for_iphone_and_ipad() {
        let project = project_pbxproj(&trestle()).unwrap();
        assert!(project.starts_with("// !$*UTF8*$!\n"));
        assert_eq!(project.matches("isa = PBXNativeTarget;").count(), 1);
        assert!(project.contains("productType = \"com.apple.product-type.application\";"));
        assert!(project.contains("TARGETED_DEVICE_FAMILY = \"1,2\";"));
        assert!(project.contains("PRODUCT_BUNDLE_IDENTIFIER = \"dev.codingadventures.taskapp\";"));
        assert!(project.contains("INFOPLIST_KEY_CFBundleDisplayName = \"Trestle\";"));
        assert!(project.contains("GENERATE_INFOPLIST_FILE = \"YES\";"));
        assert!(project.contains(
            "GCC_PREPROCESSOR_DEFINITIONS = (\"$(inherited)\", \"MOSAIC_RUNTIME_STATIC=1\");"
        ));
        assert!(project.contains(
            "SWIFT_INCLUDE_PATHS = (\"$(inherited)\", \"$(SRCROOT)/Sources/CMosaicRuntime/include\");"
        ));
    }

    #[test]
    fn every_reference_names_an_object_that_exists() {
        let project = project_pbxproj(&trestle()).unwrap();
        let objects = objects(&project);
        // Every 24-hex token in the file is either an object id or the root.
        for token in project
            .split(|character: char| !character.is_ascii_hexdigit())
            .filter(|token| token.len() == 24)
        {
            assert!(objects.contains_key(token), "dangling reference {token}");
        }
        let root = project
            .lines()
            .find_map(|line| line.trim().strip_prefix("rootObject = "))
            .unwrap()
            .trim_end_matches(';');
        assert!(objects[root].starts_with("PBXProject;"));
    }

    #[test]
    fn sources_are_compiled_and_the_xcframework_is_linked() {
        let project = project_pbxproj(&trestle()).unwrap();
        let objects = objects(&project);
        let phase = |isa: &str| {
            objects
                .values()
                .find(|body| body.starts_with(&format!("{isa};")))
                .unwrap()
                .clone()
        };
        let build_files_in = |body: &str| {
            body.matches(char::is_alphanumeric).count() > 0 && body.contains("files = (")
        };
        let sources = phase("PBXSourcesBuildPhase");
        let frameworks = phase("PBXFrameworksBuildPhase");
        assert!(build_files_in(&sources) && build_files_in(&frameworks));
        // Three compiled sources (two Swift, one C), one linked framework,
        // and the header is referenced but not compiled.
        let count = |body: &str| {
            body.split("files = (")
                .nth(1)
                .unwrap()
                .split(')')
                .next()
                .unwrap()
                .split(',')
                .filter(|id| !id.trim().is_empty())
                .count()
        };
        assert_eq!(count(&sources), 3);
        assert_eq!(count(&frameworks), 1);
        assert!(project.contains("lastKnownFileType = wrapper.xcframework;"));
        assert!(project.contains("path = \"Runtime/MosaicAppRuntime.xcframework\";"));
        assert!(project.contains("lastKnownFileType = sourcecode.c.h;"));
    }

    #[test]
    fn the_source_root_becomes_the_project_dir_path() {
        let mut app = trestle();
        app.source_root = "..".into();
        assert!(project_pbxproj(&app)
            .unwrap()
            .contains("projectDirPath = \"..\";"));
    }

    #[test]
    fn output_is_deterministic() {
        assert_eq!(
            project_pbxproj(&trestle()).unwrap(),
            project_pbxproj(&trestle()).unwrap()
        );
        assert_ne!(object_id("file:a"), object_id("file:b"));
        assert_eq!(object_id("x").len(), 24);
    }

    #[test]
    fn strings_are_quoted_and_escaped() {
        let mut app = trestle();
        app.display_name = "Say \"hi\" \\ bye; OTHER_LDFLAGS = -evil".into();
        let project = project_pbxproj(&app).unwrap();
        assert!(project.contains(
            "INFOPLIST_KEY_CFBundleDisplayName = \"Say \\\"hi\\\" \\\\ bye; OTHER_LDFLAGS = -evil\";"
        ));
        assert!(!project.contains("OTHER_LDFLAGS = -evil;"));
    }

    #[test]
    fn unusable_names_and_paths_are_refused() {
        let refused = |edit: fn(&mut IosApp)| {
            let mut app = trestle();
            edit(&mut app);
            project_pbxproj(&app).is_err()
        };
        assert!(refused(|app| app.display_name = "two\nlines".into()));
        assert!(refused(|app| app.display_name = String::new()));
        assert!(refused(
            |app| app.bundle_identifier = "dev.example/app".into()
        ));
        assert!(refused(|app| app.product_name = "App Name".into()));
        assert!(refused(|app| app.swift_sources.push("/etc/passwd".into())));
        assert!(refused(|app| app
            .swift_sources
            .push("../outside.swift".into())));
        assert!(refused(|app| app
            .swift_sources
            .push("Sources//App.swift".into())));
        assert!(refused(|app| app.c_sources.push("$(HOME)/evil.c".into())));
        assert!(refused(|app| app
            .header_search_paths
            .push("~/include".into())));
        assert!(refused(|app| app
            .swift_sources
            .push("Sources/App/App.swift".into())));
        assert!(refused(|app| app.source_root = "../..".into()));
        assert!(refused(|app| app.source_root = "/tmp".into()));
        assert!(!refused(|app| app.source_root = "..".into()));
        assert!(!refused(|_| {}));
    }
}
