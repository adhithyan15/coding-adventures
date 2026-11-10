use build_tool::cargo_trigger_snapshot::{
    resolve_cargo_trigger_snapshot, CargoCrateSnapshot, CargoTriggerPackageSnapshot,
    CargoTriggerRelation, CargoTriggerSnapshotError, CargoTriggerSnapshotInput,
    MAX_CARGO_TRIGGER_RELATIONS,
};

fn package(name: &str, root: &str, trigger_paths: &[&str]) -> CargoTriggerPackageSnapshot {
    CargoTriggerPackageSnapshot {
        name: name.to_string(),
        root: root.to_string(),
        trigger_paths: trigger_paths
            .iter()
            .map(|path| (*path).to_string())
            .collect(),
    }
}

fn crate_snapshot(path: &str, link_paths: &[&str]) -> CargoCrateSnapshot {
    CargoCrateSnapshot {
        path: path.to_string(),
        link_paths: link_paths.iter().map(|path| (*path).to_string()).collect(),
    }
}

fn input(
    packages: Vec<CargoTriggerPackageSnapshot>,
    crates: Vec<CargoCrateSnapshot>,
) -> CargoTriggerSnapshotInput {
    CargoTriggerSnapshotInput {
        packages,
        crates,
        exemptions: Vec::new(),
        config_redirected: false,
    }
}

fn relation(trigger: &str, dependent: &str) -> CargoTriggerRelation {
    CargoTriggerRelation {
        trigger: trigger.to_string(),
        dependent: dependent.to_string(),
    }
}

#[test]
fn resolves_package_nested_and_orphan_crate_closures() {
    let mut snapshot = input(
        vec![
            package(
                "app",
                "code/apps/app",
                &[
                    "../../packages/shared",
                    "../../packages/owner/native",
                    "../../orphan",
                ],
            ),
            package("shared", "code/packages/shared", &[]),
            package("core", "code/packages/core", &[]),
            package("owner", "code/packages/owner", &[]),
        ],
        vec![
            crate_snapshot("code/packages/shared", &["../core"]),
            crate_snapshot("code/packages/core", &[]),
            crate_snapshot("code/packages/owner/native", &["../../core"]),
            crate_snapshot("code/orphan", &["../packages/core"]),
        ],
    );
    snapshot.exemptions.push("code/orphan".to_string());

    assert_eq!(
        resolve_cargo_trigger_snapshot(&snapshot),
        Ok(vec![
            relation("core", "app"),
            relation("owner", "app"),
            relation("shared", "app"),
        ])
    );
}

#[test]
fn removes_self_relations_and_deduplicates_root_patch_paths() {
    let snapshot = input(
        vec![
            package("app", "code/app", &[".", "../shared", "../shared"]),
            package("shared", "code/shared", &[]),
        ],
        vec![
            crate_snapshot("code/app", &[]),
            crate_snapshot("code/shared", &[]),
        ],
    );

    assert_eq!(
        resolve_cargo_trigger_snapshot(&snapshot),
        Ok(vec![relation("shared", "app")])
    );
}

#[test]
fn shared_diamonds_are_order_independent() {
    let packages = vec![
        package("app", "code/app", &["../left", "../right"]),
        package("leaf", "code/leaf", &[]),
    ];
    let crates = vec![
        crate_snapshot("code/left", &["../leaf"]),
        crate_snapshot("code/right", &["../leaf"]),
        crate_snapshot("code/leaf", &[]),
    ];
    let mut first = input(packages.clone(), crates.clone());
    first.exemptions = vec!["code/left".to_string(), "code/right".to_string()];

    let mut reversed = input(
        packages.into_iter().rev().collect(),
        crates.into_iter().rev().collect(),
    );
    reversed.exemptions = vec!["code/right".to_string(), "code/left".to_string()];

    let expected = Ok(vec![relation("leaf", "app")]);
    assert_eq!(resolve_cargo_trigger_snapshot(&first), expected);
    assert_eq!(resolve_cargo_trigger_snapshot(&reversed), expected);
}

#[test]
fn rejects_cycles_without_returning_a_partial_relation() {
    let snapshot = input(
        vec![package("app", "code/app", &["../left"])],
        vec![
            crate_snapshot("code/left", &["../right"]),
            crate_snapshot("code/right", &["../left"]),
        ],
    );
    let mut snapshot = snapshot;
    snapshot.exemptions = vec!["code/left".to_string(), "code/right".to_string()];

    let error = resolve_cargo_trigger_snapshot(&snapshot).unwrap_err();
    assert_eq!(error, CargoTriggerSnapshotError::DependencyCycle);
    assert_eq!(error.code(), "CARGO_TRIGGER_CYCLE");
}

#[test]
fn accepts_sixteen_crates_but_rejects_the_seventeenth() {
    fn chain(length: usize) -> CargoTriggerSnapshotInput {
        let mut crates = Vec::new();
        let mut exemptions = Vec::new();
        for index in 0..length {
            let path = format!("code/c{index}");
            let links = if index + 1 == length {
                Vec::new()
            } else {
                vec![format!("../c{}", index + 1)]
            };
            crates.push(CargoCrateSnapshot {
                path: path.clone(),
                link_paths: links,
            });
            exemptions.push(path);
        }
        CargoTriggerSnapshotInput {
            packages: vec![package("app", "code/app", &["../c0"])],
            crates,
            exemptions,
            config_redirected: false,
        }
    }

    assert_eq!(resolve_cargo_trigger_snapshot(&chain(16)), Ok(Vec::new()));
    let error = resolve_cargo_trigger_snapshot(&chain(17)).unwrap_err();
    assert_eq!(error, CargoTriggerSnapshotError::DepthLimitExceeded);
    assert_eq!(error.code(), "CARGO_TRIGGER_DEPTH_LIMIT_EXCEEDED");
}

#[test]
fn cached_shallow_closure_cannot_hide_a_deep_path() {
    let mut crates = Vec::new();
    let mut exemptions = Vec::new();
    for index in 0..17 {
        let path = format!("code/c{index}");
        let links = if index == 16 {
            Vec::new()
        } else {
            vec![format!("../c{}", index + 1)]
        };
        crates.push(CargoCrateSnapshot {
            path: path.clone(),
            link_paths: links,
        });
        exemptions.push(path);
    }
    let snapshot = CargoTriggerSnapshotInput {
        // Sorted evaluation computes c10's seven-crate closure first.  The
        // later c0 route must still count that cached subtree at depth 11.
        packages: vec![
            package("a-shallow", "code/shallow", &["../c10"]),
            package("z-deep", "code/deep", &["../c0"]),
        ],
        crates,
        exemptions,
        config_redirected: false,
    };

    assert_eq!(
        resolve_cargo_trigger_snapshot(&snapshot),
        Err(CargoTriggerSnapshotError::DepthLimitExceeded)
    );
}

#[test]
fn rejects_redirects_missing_crates_and_unowned_crates() {
    let mut redirect = input(vec![package("app", "code/app", &[])], Vec::new());
    redirect.config_redirected = true;
    assert_eq!(
        resolve_cargo_trigger_snapshot(&redirect),
        Err(CargoTriggerSnapshotError::ConfigRedirect)
    );

    let missing = input(
        vec![package("app", "code/app", &["../missing"])],
        Vec::new(),
    );
    assert_eq!(
        resolve_cargo_trigger_snapshot(&missing),
        Err(CargoTriggerSnapshotError::MissingCrate)
    );

    let unowned = input(
        vec![package("app", "code/app", &["../orphan"])],
        vec![crate_snapshot("code/orphan", &[])],
    );
    assert_eq!(
        resolve_cargo_trigger_snapshot(&unowned),
        Err(CargoTriggerSnapshotError::UnownedCrate)
    );
}

#[test]
fn rejects_unsafe_and_duplicate_snapshot_identities() {
    let unsafe_root = input(vec![package("app", "code//app", &[])], Vec::new());
    assert_eq!(
        resolve_cargo_trigger_snapshot(&unsafe_root),
        Err(CargoTriggerSnapshotError::UnsafePath)
    );

    let escaping = input(
        vec![package("app", "code/app", &["../../../outside"])],
        vec![crate_snapshot("outside", &[])],
    );
    assert_eq!(
        resolve_cargo_trigger_snapshot(&escaping),
        Err(CargoTriggerSnapshotError::UnsafePath)
    );

    let reduced_drive = input(
        vec![package("app", "code/app", &["../../C:/outside"])],
        Vec::new(),
    );
    assert_eq!(
        resolve_cargo_trigger_snapshot(&reduced_drive),
        Err(CargoTriggerSnapshotError::UnsafePath)
    );

    let duplicate_name = input(
        vec![
            package("app", "code/app", &[]),
            package("app", "code/other", &[]),
        ],
        Vec::new(),
    );
    assert_eq!(
        resolve_cargo_trigger_snapshot(&duplicate_name),
        Err(CargoTriggerSnapshotError::DuplicatePackageName)
    );

    let duplicate_crate = input(
        vec![package("app", "code/app", &[])],
        vec![
            crate_snapshot("code/crate", &[]),
            crate_snapshot("code/crate", &[]),
        ],
    );
    assert_eq!(
        resolve_cargo_trigger_snapshot(&duplicate_crate),
        Err(CargoTriggerSnapshotError::DuplicateCratePath)
    );
}

#[test]
fn invalid_error_precedence_does_not_depend_on_input_order() {
    let invalid_name = package("", "code/valid", &[]);
    let unsafe_root = package("valid", "code//invalid", &[]);
    let first = input(vec![invalid_name.clone(), unsafe_root.clone()], Vec::new());
    let reversed = input(vec![unsafe_root, invalid_name], Vec::new());
    let expected = Err(CargoTriggerSnapshotError::InvalidPackageName);
    assert_eq!(resolve_cargo_trigger_snapshot(&first), expected);
    assert_eq!(resolve_cargo_trigger_snapshot(&reversed), expected);
}

fn amplified_relation_snapshot(dependent_count: usize) -> CargoTriggerSnapshotInput {
    let trigger_count = 128;
    let mut packages = Vec::new();
    let mut crates = vec![CargoCrateSnapshot {
        path: "code/hub".to_string(),
        link_paths: (0..trigger_count)
            .map(|index| format!("../trigger-{index:03}"))
            .collect(),
    }];
    for index in 0..trigger_count {
        let name = format!("trigger-{index:03}");
        let root = format!("code/{name}");
        packages.push(package(&name, &root, &[]));
        crates.push(CargoCrateSnapshot {
            path: root,
            link_paths: Vec::new(),
        });
    }
    for index in 0..dependent_count {
        packages.push(package(
            &format!("dependent-{index:03}"),
            &format!("code/dependent-{index:03}"),
            &["../hub"],
        ));
    }
    CargoTriggerSnapshotInput {
        packages,
        crates,
        exemptions: vec!["code/hub".to_string()],
        config_redirected: false,
    }
}

#[test]
fn relation_ceiling_is_exact_and_fails_before_amplified_output() {
    let at_limit = resolve_cargo_trigger_snapshot(&amplified_relation_snapshot(128))
        .expect("exactly 16,384 relations must proceed");
    assert_eq!(at_limit.len(), MAX_CARGO_TRIGGER_RELATIONS);

    assert_eq!(
        resolve_cargo_trigger_snapshot(&amplified_relation_snapshot(129)),
        Err(CargoTriggerSnapshotError::RelationLimitExceeded)
    );
    assert_eq!(
        CargoTriggerSnapshotError::RelationLimitExceeded.code(),
        "CARGO_TRIGGER_RELATION_LIMIT_EXCEEDED"
    );
}

#[test]
fn implementation_source_acquires_no_host_authority() {
    let source = include_str!("../src/cargo_trigger_snapshot.rs");
    for forbidden in [
        "std::fs",
        "std::process",
        "std::env",
        "std::net",
        "std::time",
        "PathBuf",
        "Path::",
    ] {
        assert!(
            !source.contains(forbidden),
            "pure snapshot resolver must not contain {forbidden}"
        );
    }
}
