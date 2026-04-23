use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

pub const CI_WORKFLOW_PATH: &str = ".github/workflows/ci.yml";

#[derive(Debug, Default)]
pub struct CIWorkflowChange {
    pub toolchains: HashSet<String>,
    pub requires_full_rebuild: bool,
}

const TOOLCHAIN_MARKERS: &[(&str, &[&str])] = &[
    (
        "python",
        &[
            "needs_python",
            "setup-python",
            "python-version",
            "setup-uv",
            "python --version",
            "uv --version",
            "pytest",
            "set up python",
            "install uv",
        ],
    ),
    (
        "ruby",
        &[
            "needs_ruby",
            "setup-ruby",
            "ruby-version",
            "bundler",
            "gem install bundler",
            "ruby --version",
            "bundle --version",
            "set up ruby",
            "install bundler",
        ],
    ),
    (
        "go",
        &[
            "needs_go",
            "setup-go",
            "go-version",
            "go version",
            "set up go",
        ],
    ),
    (
        "typescript",
        &[
            "needs_typescript",
            "setup-node",
            "node-version",
            "npm install -g jest",
            "node --version",
            "npm --version",
            "set up node",
        ],
    ),
    (
        "rust",
        &[
            "needs_rust",
            "rust-toolchain",
            "cargo",
            "rustc",
            "tarpaulin",
            "wasm32-unknown-unknown",
            "set up rust",
            "install cargo-tarpaulin",
        ],
    ),
    (
        "elixir",
        &[
            "needs_elixir",
            "setup-beam",
            "elixir-version",
            "otp-version",
            "elixir --version",
            "mix --version",
            "set up elixir",
        ],
    ),
    (
        "lua",
        &[
            "needs_lua",
            "gh-actions-lua",
            "gh-actions-luarocks",
            "luarocks",
            "lua -v",
            "msvc",
            "set up lua",
            "set up luarocks",
        ],
    ),
    (
        "perl",
        &["needs_perl", "cpanm", "perl --version", "install cpanm"],
    ),
    (
        "haskell",
        &[
            "needs_haskell",
            "haskell-actions/setup",
            "ghc-version",
            "cabal-version",
            "ghc --version",
            "cabal --version",
            "set up haskell",
        ],
    ),
    (
        "java",
        &[
            "needs_java",
            "setup-java",
            "java-version",
            "java --version",
            "temurin",
            "set up jdk",
            "set up gradle",
            "setup-gradle",
            "disable long-lived gradle services",
            "gradle_opts",
            "org.gradle.daemon",
            "org.gradle.vfs.watch",
        ],
    ),
    (
        "kotlin",
        &[
            "needs_kotlin",
            "setup-java",
            "java-version",
            "temurin",
            "set up jdk",
            "set up gradle",
            "setup-gradle",
            "disable long-lived gradle services",
            "gradle_opts",
            "org.gradle.daemon",
            "org.gradle.vfs.watch",
        ],
    ),
    (
        "dotnet",
        &[
            "needs_dotnet",
            "setup-dotnet",
            "dotnet-version",
            "dotnet --version",
            "set up .net",
        ],
    ),
];

const UNSAFE_MARKERS: &[&str] = &[
    "./build-tool",
    "build-tool.exe",
    "-detect-languages",
    "-emit-plan",
    "-force",
    "-plan-file",
    "-validate-build-files",
    "actions/checkout",
    "build-plan",
    "cancel-in-progress:",
    "concurrency:",
    "diff-base",
    "download-artifact",
    "event_name",
    "fetch-depth",
    "git fetch origin main",
    "git_ref",
    "is_main",
    "matrix:",
    "permissions:",
    "pr_base_ref",
    "pull_request:",
    "push:",
    "runs-on:",
    "strategy:",
    "upload-artifact",
];

pub fn analyze_ci_workflow_changes(repo_root: &Path, diff_base: &str) -> CIWorkflowChange {
    analyze_ci_workflow_patch(&get_file_diff(repo_root, diff_base, CI_WORKFLOW_PATH))
}

pub fn analyze_ci_workflow_patch(patch: &str) -> CIWorkflowChange {
    let mut toolchains = HashSet::new();
    let mut hunk = Vec::new();

    for line in patch.lines() {
        if line.starts_with("@@") {
            let (hunk_toolchains, unsafe_change) = classify_hunk(&hunk);
            if unsafe_change {
                return CIWorkflowChange {
                    toolchains: HashSet::new(),
                    requires_full_rebuild: true,
                };
            }
            toolchains.extend(hunk_toolchains);
            hunk.clear();
            continue;
        }

        if line.starts_with("diff --git ")
            || line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
        {
            continue;
        }

        hunk.push(line.to_string());
    }

    let (hunk_toolchains, unsafe_change) = classify_hunk(&hunk);
    if unsafe_change {
        return CIWorkflowChange {
            toolchains: HashSet::new(),
            requires_full_rebuild: true,
        };
    }
    toolchains.extend(hunk_toolchains);

    CIWorkflowChange {
        toolchains,
        requires_full_rebuild: false,
    }
}

pub fn sorted_toolchains(toolchains: &HashSet<String>) -> Vec<String> {
    let mut sorted: Vec<String> = toolchains.iter().cloned().collect();
    sorted.sort();
    sorted
}

fn classify_hunk(lines: &[String]) -> (HashSet<String>, bool) {
    let mut hunk_toolchains = HashSet::new();
    let mut changed_toolchains = HashSet::new();
    let mut changed_lines = Vec::new();

    for line in lines {
        if line.is_empty() || !is_diff_line(line) {
            continue;
        }

        let content = line[1..].trim();
        hunk_toolchains.extend(detect_toolchains(content));

        if !is_changed_line(line) {
            continue;
        }
        if content.is_empty() || content.starts_with('#') {
            continue;
        }

        changed_lines.push(content.to_string());
        changed_toolchains.extend(detect_toolchains(content));
    }

    if changed_lines.is_empty() {
        return (HashSet::new(), false);
    }

    let resolved_toolchains = if changed_toolchains.is_empty() {
        if hunk_toolchains.len() != 1 {
            return (HashSet::new(), true);
        }
        hunk_toolchains
    } else {
        changed_toolchains
    };

    for content in &changed_lines {
        if touches_shared_ci_behavior(content) {
            return (HashSet::new(), true);
        }
        if !detect_toolchains(content).is_empty() {
            continue;
        }
        if is_toolchain_scoped_structural_line(content) {
            continue;
        }
        return (HashSet::new(), true);
    }

    (resolved_toolchains, false)
}

fn detect_toolchains(content: &str) -> HashSet<String> {
    let normalized = content.to_lowercase();
    let mut found = HashSet::new();

    for (toolchain, markers) in TOOLCHAIN_MARKERS {
        if markers.iter().any(|marker| normalized.contains(marker)) {
            found.insert((*toolchain).to_string());
        }
    }

    found
}

fn touches_shared_ci_behavior(content: &str) -> bool {
    let normalized = content.to_lowercase();
    UNSAFE_MARKERS
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn is_toolchain_scoped_structural_line(content: &str) -> bool {
    [
        "if:",
        "run:",
        "shell:",
        "with:",
        "env:",
        "{",
        "}",
        "else",
        "fi",
        "then",
        "printf ",
        "echo ",
        "curl ",
        "powershell ",
        "call ",
        "cd ",
    ]
    .iter()
    .any(|prefix| content.starts_with(prefix))
}

fn is_diff_line(line: &str) -> bool {
    line.starts_with(' ') || is_changed_line(line)
}

fn is_changed_line(line: &str) -> bool {
    line.starts_with('+') || line.starts_with('-')
}

fn get_file_diff(repo_root: &Path, diff_base: &str, relative_path: &str) -> String {
    for args in [
        vec![
            "diff".to_string(),
            "--unified=0".to_string(),
            format!("{diff_base}...HEAD"),
            "--".to_string(),
            relative_path.to_string(),
        ],
        vec![
            "diff".to_string(),
            "--unified=0".to_string(),
            diff_base.to_string(),
            "HEAD".to_string(),
            "--".to_string(),
            relative_path.to_string(),
        ],
    ] {
        match Command::new("git")
            .args(&args)
            .current_dir(repo_root)
            .output()
        {
            Ok(output) if output.status.success() => {
                return String::from_utf8_lossy(&output.stdout).into_owned();
            }
            _ => {}
        }
    }

    String::new()
}

#[cfg(test)]
mod tests {
    use super::{analyze_ci_workflow_patch, sorted_toolchains};

    #[test]
    fn test_analyze_ci_workflow_patch_allows_toolchain_scoped_dotnet_changes() {
        let change = analyze_ci_workflow_patch(
            r#"
@@ -312,0 +313,6 @@
+      - name: Set up .NET
+        if: needs.detect.outputs.needs_dotnet == 'true'
+        uses: actions/setup-dotnet@v4
+        with:
+          dotnet-version: '9.0.x'
"#,
        );

        assert!(!change.requires_full_rebuild);
        assert_eq!(sorted_toolchains(&change.toolchains), vec!["dotnet"]);
    }

    #[test]
    fn test_analyze_ci_workflow_patch_allows_shared_jvm_toolchain_changes() {
        let change = analyze_ci_workflow_patch(
            r#"
@@ -314,0 +315,17 @@
+      - name: Set up JDK 21
+        if: needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true'
+        uses: actions/setup-java@v4
+        with:
+          distribution: 'temurin'
+          java-version: '21'
+      - name: Set up Gradle
+        if: needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true'
+        uses: gradle/actions/setup-gradle@v4
+      - name: Disable long-lived Gradle services on Windows CI
+        if: (needs.detect.outputs.needs_java == 'true' || needs.detect.outputs.needs_kotlin == 'true') && runner.os == 'Windows'
+        shell: bash
+        run: |
+          {
+            echo 'GRADLE_OPTS=-Dorg.gradle.daemon=false -Dorg.gradle.vfs.watch=false'
+          } >> "$GITHUB_ENV"
"#,
        );

        assert!(!change.requires_full_rebuild);
        assert_eq!(sorted_toolchains(&change.toolchains), vec!["java", "kotlin"]);
    }

    #[test]
    fn test_analyze_ci_workflow_patch_ignores_comment_only_changes() {
        let change = analyze_ci_workflow_patch(
            r#"
@@ -316,2 +316,2 @@
-          # .NET 8 is the current LTS release.
+          # .NET 9 is the current LTS release.
"#,
        );

        assert!(!change.requires_full_rebuild);
        assert!(change.toolchains.is_empty());
    }

    #[test]
    fn test_analyze_ci_workflow_patch_requires_full_rebuild_for_build_command_changes() {
        let change = analyze_ci_workflow_patch(
            r#"
@@ -404,1 +404,1 @@
-          $BT -root . -validate-build-files -language all
+          $BT -root . -force -validate-build-files -language all
"#,
        );

        assert!(change.requires_full_rebuild);
    }

    #[test]
    fn test_analyze_ci_workflow_patch_requires_full_rebuild_for_unknown_changes() {
        let change = analyze_ci_workflow_patch(
            r#"
@@ -170,0 +171,1 @@
+      timeout-minutes: 45
"#,
        );

        assert!(change.requires_full_rebuild);
    }
}
