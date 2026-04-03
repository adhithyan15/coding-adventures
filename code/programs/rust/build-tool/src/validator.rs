use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::discovery::Package;

const CI_MANAGED_TOOLCHAIN_LANGUAGES: &[&str] = &[
    "python",
    "ruby",
    "typescript",
    "rust",
    "elixir",
    "lua",
    "perl",
];

pub fn validate_ci_full_build_toolchains(repo_root: &Path, packages: &[Package]) -> Option<String> {
    let ci_path = repo_root.join(".github").join("workflows").join("ci.yml");
    let workflow = fs::read_to_string(&ci_path).ok()?;

    if !workflow.contains("Full build on main merge") {
        return None;
    }

    let compact_workflow: String = workflow.chars().filter(|c| !c.is_whitespace()).collect();
    let mut missing_output_binding = Vec::new();
    let mut missing_main_force = Vec::new();

    for lang in languages_needing_ci_toolchains(packages) {
        let output_binding = format!("needs_{lang}:${{{{steps.toolchains.outputs.needs_{lang}}}}}");
        if !compact_workflow.contains(&output_binding) {
            missing_output_binding.push(lang.clone());
        }

        if !compact_workflow.contains(&format!("needs_{lang}=true")) {
            missing_main_force.push(lang);
        }
    }

    if missing_output_binding.is_empty() && missing_main_force.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    if !missing_output_binding.is_empty() {
        parts.push(format!(
            "detect outputs for forced main full builds are not normalized through steps.toolchains for: {}",
            missing_output_binding.join(", ")
        ));
    }
    if !missing_main_force.is_empty() {
        parts.push(format!(
            "forced main full-build path does not explicitly enable toolchains for: {}",
            missing_main_force.join(", ")
        ));
    }

    Some(format!(
        "{}: {}",
        ci_path.to_string_lossy().replace('\\', "/"),
        parts.join("; ")
    ))
}

fn languages_needing_ci_toolchains(packages: &[Package]) -> Vec<String> {
    let mut langs = BTreeSet::new();
    for pkg in packages {
        if CI_MANAGED_TOOLCHAIN_LANGUAGES.contains(&pkg.language.as_str()) {
            langs.insert(pkg.language.clone());
        }
    }
    langs.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::validate_ci_full_build_toolchains;
    use crate::discovery::Package;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_package(root: &std::path::Path, rel_path: &str, language: &str) -> Package {
        let pkg_path = root.join(rel_path);
        fs::create_dir_all(&pkg_path).unwrap();
        Package {
            name: format!(
                "{language}/{}",
                pkg_path.file_name().unwrap().to_string_lossy()
            ),
            path: pkg_path,
            build_commands: vec!["echo hi".to_string()],
            language: language.to_string(),
        }
    }

    fn make_temp_root(label: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "build_tool_validator_{}_{}_{}",
            label,
            std::process::id(),
            stamp
        ))
    }

    #[test]
    fn fails_without_normalized_outputs() {
        let root = make_temp_root("missing");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".github/workflows")).unwrap();

        let packages = vec![
            make_package(&root, "code/packages/elixir/actor", "elixir"),
            make_package(&root, "code/packages/python/actor", "python"),
        ];

        fs::write(
            root.join(".github/workflows/ci.yml"),
            r#"
jobs:
  detect:
    outputs:
      needs_python: ${{ steps.detect.outputs.needs_python }}
      needs_elixir: ${{ steps.detect.outputs.needs_elixir }}
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
"#,
        )
        .unwrap();

        let error = validate_ci_full_build_toolchains(&root, &packages).unwrap();
        assert!(error.contains(".github/workflows/ci.yml"));
        assert!(error.contains("python"));
        assert!(error.contains("elixir"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn allows_normalized_outputs() {
        let root = make_temp_root("normalized");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".github/workflows")).unwrap();

        let packages = vec![
            make_package(&root, "code/packages/elixir/actor", "elixir"),
            make_package(&root, "code/packages/python/actor", "python"),
        ];

        fs::write(
            root.join(".github/workflows/ci.yml"),
            r#"
jobs:
  detect:
    outputs:
      needs_python: ${{ steps.toolchains.outputs.needs_python }}
      needs_elixir: ${{ steps.toolchains.outputs.needs_elixir }}
    steps:
      - name: Normalize toolchain requirements
        id: toolchains
        run: |
          printf '%s\n' \
            'needs_python=true' \
            'needs_elixir=true' >> "$GITHUB_OUTPUT"
  build:
    steps:
      - name: Full build on main merge
        run: ./build-tool -root . -force -validate-build-files -language all
"#,
        )
        .unwrap();

        assert!(validate_ci_full_build_toolchains(&root, &packages).is_none());

        let _ = fs::remove_dir_all(&root);
    }
}
