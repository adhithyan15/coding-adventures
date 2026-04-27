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
    "java",
    "kotlin",
    "haskell",
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

pub fn validate_build_contracts(repo_root: &Path, packages: &[Package]) -> Option<String> {
    let mut errors = Vec::new();

    if let Some(error) = validate_ci_full_build_toolchains(repo_root, packages) {
        errors.push(error);
    }
    errors.extend(validate_lua_isolated_build_files(packages));
    errors.extend(validate_perl_build_files(packages));

    if errors.is_empty() {
        None
    } else {
        Some(errors.join("\n  - "))
    }
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

fn validate_lua_isolated_build_files(packages: &[Package]) -> Vec<String> {
    let mut errors = Vec::new();

    for pkg in packages {
        if pkg.language != "lua" {
            continue;
        }

        let self_rock = format!(
            "coding-adventures-{}",
            pkg.path
                .file_name()
                .map(|name| name.to_string_lossy().replace('_', "-"))
                .unwrap_or_default()
        );
        let mut build_lines = std::collections::BTreeMap::new();

        for build_path in lua_build_files(&pkg.path) {
            let lines = read_build_lines(&build_path);
            if let Some(name) = build_path.file_name().and_then(|value| value.to_str()) {
                build_lines.insert(name.to_string(), lines.clone());
            }
            if lines.is_empty() {
                continue;
            }

            if let Some(foreign_remove) = first_foreign_lua_remove(&lines, &self_rock) {
                errors.push(format!(
                    "{}: Lua BUILD removes unrelated rock {}; isolated package builds should only remove the package they are rebuilding",
                    build_path.to_string_lossy().replace('\\', "/"),
                    foreign_remove
                ));
            }

            let state_machine_index =
                first_line_containing(&lines, &["../state_machine", "..\\state_machine"]);
            let directed_graph_index =
                first_line_containing(&lines, &["../directed_graph", "..\\directed_graph"]);
            if let (Some(state_machine_index), Some(directed_graph_index)) =
                (state_machine_index, directed_graph_index)
            {
                if state_machine_index < directed_graph_index {
                    errors.push(format!(
                        "{}: Lua BUILD installs state_machine before directed_graph; isolated LuaRocks builds require directed_graph first",
                        build_path.to_string_lossy().replace('\\', "/")
                    ));
                }
            }

            if (has_guarded_local_lua_install(&lines)
                || (build_path.file_name().and_then(|value| value.to_str()) == Some("BUILD_windows")
                    && has_local_lua_sibling_install(&lines)))
                && !self_install_disables_deps(&lines, &self_rock)
            {
                errors.push(format!(
                    "{}: Lua BUILD bootstraps sibling rocks but the final self-install does not pass --deps-mode=none or --no-manifest",
                    build_path.to_string_lossy().replace('\\', "/")
                ));
            }
        }

        let missing_windows_deps = missing_lua_sibling_installs(
            build_lines.get("BUILD").map(Vec::as_slice).unwrap_or(&[]),
            build_lines
                .get("BUILD_windows")
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        );
        if !missing_windows_deps.is_empty() {
            errors.push(format!(
                "{}: Lua BUILD_windows is missing sibling installs present in BUILD: {}",
                pkg.path.join("BUILD_windows").to_string_lossy().replace('\\', "/"),
                missing_windows_deps.join(", ")
            ));
        }
    }

    errors
}

fn validate_perl_build_files(packages: &[Package]) -> Vec<String> {
    let mut errors = Vec::new();

    for pkg in packages {
        if pkg.language != "perl" {
            continue;
        }

        for build_path in lua_build_files(&pkg.path) {
            let lines = read_build_lines(&build_path);
            if lines.iter().any(|line| {
                line.contains("cpanm")
                    && line.contains("Test2::V0")
                    && !line.contains("--notest")
            }) {
                errors.push(format!(
                    "{}: Perl BUILD bootstraps Test2::V0 without --notest; isolated Windows installs can fail while installing the test framework itself",
                    build_path.to_string_lossy().replace('\\', "/")
                ));
            }
        }
    }

    errors
}

fn lua_build_files(pkg_path: &Path) -> Vec<std::path::PathBuf> {
    let mut files = match fs::read_dir(pkg_path) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.starts_with("BUILD"))
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    files.sort();
    files
}

fn read_build_lines(build_path: &Path) -> Vec<String> {
    let contents = match fs::read_to_string(build_path) {
        Ok(contents) => contents,
        Err(_) => return Vec::new(),
    };

    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn first_foreign_lua_remove(lines: &[String], self_rock: &str) -> Option<String> {
    for line in lines {
        let marker = "luarocks remove --force ";
        let Some(start) = line.find(marker) else {
            continue;
        };
        let remainder = &line[start + marker.len()..];
        let target = remainder
            .split_whitespace()
            .next()
            .unwrap_or_default();
        if !target.is_empty() && target != self_rock {
            return Some(target.to_string());
        }
    }
    None
}

fn first_line_containing(lines: &[String], needles: &[&str]) -> Option<usize> {
    lines.iter().enumerate().find_map(|(index, line)| {
        needles
            .iter()
            .any(|needle| line.contains(needle))
            .then_some(index)
    })
}

fn has_guarded_local_lua_install(lines: &[String]) -> bool {
    lines
        .iter()
        .any(|line| line.contains("luarocks show ") && (line.contains("../") || line.contains("..\\")))
}

fn has_local_lua_sibling_install(lines: &[String]) -> bool {
    !lua_sibling_install_dirs(lines).is_empty()
}

fn self_install_disables_deps(lines: &[String], self_rock: &str) -> bool {
    lines.iter().any(|line| {
        line.contains("luarocks make")
            && line.contains(self_rock)
            && (line.contains("--deps-mode=none")
                || line.contains("--deps-mode none")
                || line.contains("--no-manifest"))
    })
}

fn missing_lua_sibling_installs(unix_lines: &[String], windows_lines: &[String]) -> Vec<String> {
    let windows_deps: std::collections::BTreeSet<String> =
        lua_sibling_install_dirs(windows_lines).into_iter().collect();
    lua_sibling_install_dirs(unix_lines)
        .into_iter()
        .filter(|dep| !windows_deps.contains(dep))
        .collect()
}

fn lua_sibling_install_dirs(lines: &[String]) -> Vec<String> {
    let mut dirs = BTreeSet::new();

    for line in lines {
        if !line.contains("luarocks make") {
            continue;
        }
        let Some(start) = line.find("cd ") else {
            continue;
        };
        let remainder = &line[start + 3..];
        let dep = remainder.split_whitespace().next().unwrap_or_default();
        if !(dep.starts_with("../") || dep.starts_with("..\\")) {
            continue;
        }
        dirs.insert(dep.replace('\\', "/"));
    }

    dirs.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::{validate_build_contracts, validate_ci_full_build_toolchains};
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

    #[test]
    fn validate_build_contracts_flags_lua_isolated_build_violations() {
        let root = make_temp_root("lua_violations");
        let _ = fs::remove_dir_all(&root);

        let problem_path = root.join("code/packages/lua/problem_pkg");
        fs::create_dir_all(&problem_path).unwrap();

        let packages = vec![Package {
            name: "lua/problem_pkg".to_string(),
            path: problem_path.clone(),
            build_commands: vec!["echo hi".to_string()],
            language: "lua".to_string(),
        }];

        fs::write(
            problem_path.join("BUILD"),
            r#"
luarocks remove --force coding-adventures-branch-predictor 2>/dev/null || true
(cd ../state_machine && luarocks make --local coding-adventures-state-machine-0.1.0-1.rockspec)
(cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks make --local coding-adventures-problem-pkg-0.1.0-1.rockspec
"#,
        )
        .unwrap();

        let error = validate_build_contracts(&root, &packages).unwrap();
        assert!(error.contains("coding-adventures-branch-predictor"));
        assert!(error.contains("state_machine before directed_graph"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_build_contracts_flags_guarded_lua_install_without_deps_mode() {
        let root = make_temp_root("lua_guarded");
        let _ = fs::remove_dir_all(&root);

        let guarded_path = root.join("code/packages/lua/guarded_pkg");
        fs::create_dir_all(&guarded_path).unwrap();

        let packages = vec![Package {
            name: "lua/guarded_pkg".to_string(),
            path: guarded_path.clone(),
            build_commands: vec!["echo hi".to_string()],
            language: "lua".to_string(),
        }];

        fs::write(
            guarded_path.join("BUILD"),
            r#"
luarocks show coding-adventures-transistors >/dev/null 2>&1 || (cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
luarocks make --local coding-adventures-guarded-pkg-0.1.0-1.rockspec
"#,
        )
        .unwrap();

        let error = validate_build_contracts(&root, &packages).unwrap();
        assert!(error.contains("--deps-mode=none or --no-manifest"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_build_contracts_allows_safe_lua_patterns() {
        let root = make_temp_root("lua_safe");
        let _ = fs::remove_dir_all(&root);

        let safe_path = root.join("code/packages/lua/safe_pkg");
        fs::create_dir_all(&safe_path).unwrap();

        let packages = vec![Package {
            name: "lua/safe_pkg".to_string(),
            path: safe_path.clone(),
            build_commands: vec!["echo hi".to_string()],
            language: "lua".to_string(),
        }];

        fs::write(
            safe_path.join("BUILD"),
            r#"
luarocks remove --force coding-adventures-safe-pkg 2>/dev/null || true
luarocks show coding-adventures-directed-graph >/dev/null 2>&1 || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine >/dev/null 2>&1 || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
"#,
        )
        .unwrap();
        fs::write(
            safe_path.join("BUILD_windows"),
            r#"
luarocks show coding-adventures-directed-graph 1>nul 2>nul || (cd ../directed_graph && luarocks make --local coding-adventures-directed-graph-0.1.0-1.rockspec)
luarocks show coding-adventures-state-machine 1>nul 2>nul || (cd ../state_machine && luarocks make --local --deps-mode=none coding-adventures-state-machine-0.1.0-1.rockspec)
luarocks make --local --deps-mode=none coding-adventures-safe-pkg-0.1.0-1.rockspec
"#,
        )
        .unwrap();

        assert!(validate_build_contracts(&root, &packages).is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_build_contracts_flags_windows_lua_sibling_drift() {
        let root = make_temp_root("lua_windows_drift");
        let _ = fs::remove_dir_all(&root);

        let package_path = root.join("code/packages/lua/arm1_gatelevel");
        fs::create_dir_all(&package_path).unwrap();

        let packages = vec![Package {
            name: "lua/arm1_gatelevel".to_string(),
            path: package_path.clone(),
            build_commands: vec!["echo hi".to_string()],
            language: "lua".to_string(),
        }];

        fs::write(
            package_path.join("BUILD"),
            r#"
(cd ../transistors && luarocks make --local coding-adventures-transistors-0.1.0-1.rockspec)
(cd ../logic_gates && luarocks make --local coding-adventures-logic-gates-0.1.0-1.rockspec)
(cd ../arithmetic && luarocks make --local coding-adventures-arithmetic-0.1.0-1.rockspec)
(cd ../arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
"#,
        )
        .unwrap();
        fs::write(
            package_path.join("BUILD_windows"),
            r#"
(cd ..\arm1_simulator && luarocks make --local coding-adventures-arm1-simulator-0.1.0-1.rockspec)
luarocks make --local coding-adventures-arm1-gatelevel-0.1.0-1.rockspec
"#,
        )
        .unwrap();

        let error = validate_build_contracts(&root, &packages).unwrap();
        assert!(error.contains("BUILD_windows is missing sibling installs present in BUILD"));
        assert!(error.contains("../logic_gates"));
        assert!(error.contains("../arithmetic"));
        assert!(error.contains("--deps-mode=none or --no-manifest"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_build_contracts_flags_perl_test2_bootstrap_without_notest() {
        let root = make_temp_root("perl_test2");
        let _ = fs::remove_dir_all(&root);

        let package_path = root.join("code/packages/perl/draw-instructions-svg");
        fs::create_dir_all(&package_path).unwrap();

        let packages = vec![Package {
            name: "perl/draw-instructions-svg".to_string(),
            path: package_path.clone(),
            build_commands: vec!["echo hi".to_string()],
            language: "perl".to_string(),
        }];

        fs::write(
            package_path.join("BUILD"),
            r#"
cpanm --quiet Test2::V0
prove -l -I../draw-instructions/lib -v t/
"#,
        )
        .unwrap();

        let error = validate_build_contracts(&root, &packages).unwrap();
        assert!(error.contains("Test2::V0 without --notest"));

        let _ = fs::remove_dir_all(&root);
    }
}
