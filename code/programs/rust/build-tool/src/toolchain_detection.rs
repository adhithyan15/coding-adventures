#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;

    #[derive(Debug, Deserialize)]
    struct FixtureCase {
        id: String,
        input: FixtureInput,
        expected: FixtureExpected,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureInput {
        options: FixtureOptions,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureOptions {
        platform: String,
        force_full: bool,
        packages: Vec<PackageSnapshot>,
        scheduled_packages: Option<Vec<String>>,
        forced_toolchains: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    struct FixtureExpected {
        outcome: String,
        #[serde(default)]
        result: FixtureResult,
        diagnostics: Vec<ToolchainDiagnostic>,
    }

    #[derive(Debug, Default, Deserialize)]
    struct FixtureResult {
        #[serde(default)]
        toolchains: BTreeMap<String, bool>,
    }

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(4)
            .expect("repository root")
            .join("code/specs/fixtures/build-tool-v1/cases")
    }

    fn package(
        name: &str,
        language: &str,
        fronts: impl IntoIterator<Item = (&'static str, String)>,
    ) -> PackageSnapshot {
        PackageSnapshot {
            name: name.to_string(),
            language: language.to_string(),
            build_files: fronts
                .into_iter()
                .map(|(front, content)| (front.to_string(), content))
                .collect(),
        }
    }

    #[test]
    fn consumes_every_language_neutral_toolchain_fixture() {
        let mut fixture_paths: Vec<PathBuf> = fs::read_dir(fixture_root())
            .expect("fixture directory")
            .map(|entry| entry.expect("fixture entry").path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with("toolchain-detection-") && name.ends_with(".json")
                    })
            })
            .collect();
        fixture_paths.sort();

        let names: Vec<&str> = fixture_paths
            .iter()
            .filter_map(|path| path.file_name()?.to_str())
            .collect();
        assert_eq!(
            names,
            vec![
                "toolchain-detection-affected-only.json",
                "toolchain-detection-crlf-grammar.json",
                "toolchain-detection-declarations.json",
                "toolchain-detection-empty.json",
                "toolchain-detection-force-full.json",
                "toolchain-detection-null-all.json",
                "toolchain-detection-platform-darwin.json",
                "toolchain-detection-platform-linux.json",
                "toolchain-detection-platform-windows.json",
                "toolchain-detection-shared.json",
                "toolchain-detection-unsupported.json",
            ]
        );

        for path in fixture_paths {
            let case: FixtureCase =
                serde_json::from_slice(&fs::read(&path).expect("read fixture"))
                    .expect("parse fixture");
            let options = case.input.options;
            let actual = evaluate_snapshot(
                &options.platform,
                options.force_full,
                &options.packages,
                options.scheduled_packages.as_deref(),
                &options.forced_toolchains,
            )
            .unwrap_or_else(|error| panic!("{}: {error:?}", case.id));
            assert_eq!(actual.outcome, case.expected.outcome, "{}", case.id);
            assert_eq!(
                actual.toolchains, case.expected.result.toolchains,
                "{}",
                case.id
            );
            assert_eq!(
                actual.diagnostics, case.expected.diagnostics,
                "{}",
                case.id
            );
        }
    }

    #[test]
    fn parser_enforces_utf8_byte_and_logical_line_limits_before_splitting() {
        let ascii_at_limit = "x".repeat(MAX_BUILD_BYTES);
        assert_eq!(parse_extra_toolchains(&ascii_at_limit).unwrap(), Vec::<String>::new());
        assert_eq!(
            parse_extra_toolchains(&(ascii_at_limit + "x")),
            Err(SnapshotError::PerFileByteLimit)
        );

        let multibyte_at_limit = "é".repeat(MAX_BUILD_BYTES / 2);
        assert_eq!(multibyte_at_limit.as_bytes().len(), MAX_BUILD_BYTES);
        assert!(parse_extra_toolchains(&multibyte_at_limit).is_ok());
        assert_eq!(
            parse_extra_toolchains(&(multibyte_at_limit + "é")),
            Err(SnapshotError::PerFileByteLimit)
        );

        let at_line_limit = "x\n".repeat(MAX_BUILD_LINES - 1);
        assert!(parse_extra_toolchains(&at_line_limit).is_ok());
        assert_eq!(
            parse_extra_toolchains(&(at_line_limit + "\n")),
            Err(SnapshotError::PerFileLineLimit)
        );
    }

    #[test]
    fn declaration_grammar_is_byte_exact_and_stably_deduplicated() {
        let content = concat!(
            " # needs-toolchain: python \r\n",
            "# needs-toolchain:\tjava\t\n",
            "# needs-toolchain: python\n",
            "# needs-toolchain: ruby\r",
            "# needs-toolchain: lua\r  \n",
            "# needs-toolchain: perl\r\t\n",
            "# needs-toolchain: swift\r\r\n",
            "# needs-toolchain:python\n",
            "# Needs-toolchain: kotlin\n",
            "# needs-toolchain: zig\n",
            "# needs-toolchain: kotlin trailing\n",
        );
        assert_eq!(
            parse_extra_toolchains(content).unwrap(),
            vec!["python".to_string(), "java".to_string()]
        );
    }

    #[test]
    fn meters_every_front_and_the_aggregate_before_selection() {
        let unselected_oversized = package(
            "rust/app",
            "rust",
            [
                ("BUILD", String::new()),
                ("BUILD_windows", "x".repeat(MAX_BUILD_BYTES + 1)),
            ],
        );
        assert_eq!(
            evaluate_snapshot("linux", false, &[unselected_oversized], None, &[]),
            Err(SnapshotError::PerFileByteLimit)
        );

        let exact: Vec<PackageSnapshot> = (0..16)
            .map(|index| {
                package(
                    &format!("rust/exact-{index}"),
                    "rust",
                    [("BUILD", "x".repeat(MAX_BUILD_BYTES))],
                )
            })
            .collect();
        assert!(evaluate_snapshot("linux", false, &exact, Some(&[]), &[]).is_ok());

        let over: Vec<PackageSnapshot> = (0..17)
            .map(|index| {
                package(
                    &format!("rust/over-{index}"),
                    "rust",
                    [("BUILD", "x".repeat(MAX_BUILD_BYTES))],
                )
            })
            .collect();
        assert_eq!(
            evaluate_snapshot("linux", false, &over, Some(&[]), &[]),
            Err(SnapshotError::AggregateByteLimit)
        );
    }

    #[test]
    fn present_empty_platform_front_wins_over_generic_declarations() {
        let snapshot = package(
            "rust/app",
            "rust",
            [
                ("BUILD", "# needs-toolchain: python\n".to_string()),
                ("BUILD_linux", String::new()),
            ],
        );
        let actual = evaluate_snapshot("linux", false, &[snapshot], None, &[]).unwrap();
        assert_eq!(actual.toolchains["rust"], true);
        assert_eq!(actual.toolchains["python"], false);
    }

    #[test]
    fn null_and_empty_schedules_are_distinct() {
        let snapshots = [package("python/app", "python", [("BUILD", String::new())])];
        let all = evaluate_snapshot("linux", false, &snapshots, None, &[]).unwrap();
        let none = evaluate_snapshot("linux", false, &snapshots, Some(&[]), &[]).unwrap();
        assert_eq!(all.toolchains["python"], true);
        assert!(none.toolchains.values().all(|enabled| !enabled));
    }

    #[test]
    fn aliases_forced_toolchains_and_force_full_are_closed() {
        let snapshots = [
            package("c/app", "c", [("BUILD", String::new())]),
            package("fsharp/app", "fsharp", [("BUILD", String::new())]),
            package("wasm/app", "wasm", [("BUILD", String::new())]),
        ];
        let selected = evaluate_snapshot(
            "windows",
            false,
            &snapshots,
            None,
            &["kotlin".to_string()],
        )
        .unwrap();
        assert_eq!(selected.toolchains["cpp"], true);
        assert_eq!(selected.toolchains["dotnet"], true);
        assert_eq!(selected.toolchains["rust"], true);
        assert_eq!(selected.toolchains["kotlin"], true);

        let full = evaluate_snapshot("windows", true, &snapshots, None, &[]).unwrap();
        assert_eq!(full.toolchains.len(), CANONICAL_TOOLCHAINS.len());
        assert!(full.toolchains.values().all(|enabled| *enabled));
    }

    #[test]
    fn selected_package_error_precedes_forced_toolchain_error_even_when_full() {
        let snapshots = [package("zig/app", "zig", [("BUILD", String::new())])];
        let actual = evaluate_snapshot(
            "linux",
            true,
            &snapshots,
            None,
            &["zig".to_string()],
        )
        .unwrap();
        assert_eq!(actual.outcome, "error");
        assert_eq!(actual.toolchains, BTreeMap::new());
        assert_eq!(
            actual.diagnostics,
            vec![ToolchainDiagnostic {
                code: "TOOLCHAIN_UNSUPPORTED".to_string(),
                severity: "error".to_string(),
                package: Some("zig/app".to_string()),
            }]
        );
    }

    #[test]
    fn unsupported_platform_fails_before_full_or_empty_schedule_shortcuts() {
        assert_eq!(
            evaluate_snapshot("solaris", true, &[], None, &[]),
            Err(SnapshotError::UnsupportedPlatform)
        );
        assert_eq!(
            evaluate_snapshot("solaris", false, &[], Some(&[]), &[]),
            Err(SnapshotError::UnsupportedPlatform)
        );
    }

    #[test]
    fn force_full_requires_the_null_schedule_shape() {
        assert_eq!(
            evaluate_snapshot("linux", true, &[], Some(&[]), &[]),
            Err(SnapshotError::ForceFullRequiresAllPackages)
        );
    }

    #[test]
    fn results_are_fresh_sorted_maps_and_inputs_are_unchanged() {
        let snapshots = [package(
            "rust/app",
            "rust",
            [("BUILD", "# needs-toolchain: python\n".to_string())],
        )];
        let original = snapshots.clone();
        let mut first = evaluate_snapshot("linux", false, &snapshots, None, &[]).unwrap();
        let second = evaluate_snapshot("linux", false, &snapshots, None, &[]).unwrap();
        first.toolchains.insert("rust".to_string(), false);
        assert_eq!(second.toolchains["rust"], true);
        assert_eq!(snapshots, original);
        assert_eq!(
            second.toolchains.keys().map(String::as_str).collect::<Vec<_>>(),
            CANONICAL_TOOLCHAINS
        );
    }
}
