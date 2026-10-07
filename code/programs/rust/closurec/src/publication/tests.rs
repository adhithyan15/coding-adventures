//! Deterministic filesystem faults exercise states normal process tests cannot.
use super::*;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix_policy_tests;

struct Fixture {
    dir: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "closurec-publish-unit-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        Self { dir }
    }
    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
    fn recovery(&self) -> Vec<PathBuf> {
        fs::read_dir(&self.dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_dir())
            .collect()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(self.dir.parent(), Some(std::env::temp_dir().as_path()));
        fs::remove_dir_all(&self.dir).unwrap();
    }
}
fn injected() -> io::Error {
    io::Error::other("injected filesystem failure")
}

#[cfg(windows)]
#[test]
fn windows_new_outputs_use_actual_parent_policy_and_inherited_outputs_preserve_it() {
    let fixture = Fixture::new();
    let parent = observe(&fixture.dir, true).unwrap().unwrap();
    let policy = windows_security::new_file_policy(&parent.file).unwrap();
    let path = fixture.path("output");
    publish_outputs(&[(path.clone(), "first".into())]).unwrap();
    assert_eq!(observe(&path, false).unwrap().unwrap().policy, policy);
    publish_outputs(&[(path.clone(), "second".into())]).unwrap();
    assert_eq!(observe(&path, false).unwrap().unwrap().policy, policy);
    assert_eq!(fs::read(path).unwrap(), b"second");
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn windows_acl_only_change_is_detected_without_identity_length_or_mtime_change() {
    let fixture = Fixture::new();
    let path = fixture.path("output");
    fs::write(&path, "original").unwrap();
    let original = observe(&path, false).unwrap().unwrap();
    let parent = observe(&fixture.dir, true).unwrap().unwrap();
    let changed = windows_security::new_file_policy(&parent.file).unwrap();
    assert_ne!(original.policy, changed);
    let result = publish_with_hook(&[(path.clone(), "new".into())], &mut |phase, _| {
        if phase == Phase::Install {
            changed.apply(&windows_security::policy_handle(&path)?)?;
        }
        Ok(())
    });
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("destination changed"));
    let current = observe(&path, false).unwrap().unwrap();
    assert_eq!(current.id, original.id);
    assert_eq!(current.len, original.len);
    assert_eq!(current.modified, original.modified);
    assert_eq!(current.policy, changed);
    assert_eq!(fs::read(path).unwrap(), b"original");
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn windows_policy_failure_restores_original_bytes_and_policy() {
    let fixture = Fixture::new();
    let a = fixture.path("existing");
    let b = fixture.path("missing");
    fs::write(&a, "original").unwrap();
    let original = observe(&a, false).unwrap().unwrap();
    let result = publish_with_hook(
        &[(a.clone(), "new a".into()), (b.clone(), "new b".into())],
        &mut |phase, path| {
            if phase == Phase::Policy && path.file_name() == b.file_name() {
                Err(injected())
            } else {
                Ok(())
            }
        },
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("injected filesystem failure"));
    assert!(same_original(
        &original,
        &observe(&a, false).unwrap().unwrap()
    ));
    assert_eq!(fs::read(a).unwrap(), b"original");
    assert!(!b.exists());
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn windows_orphan_inherited_policy_rejects_before_original_mutation() {
    let fixture = Fixture::new();
    let original_path = fixture.path("source");
    publish_outputs(&[(original_path.clone(), "original".into())]).unwrap();
    let original = observe(&original_path, false).unwrap().unwrap();
    assert!(original.policy.has_inherited_entries());
    let private_parent = fixture.path("different-parent");
    windows_security::create_directory(&private_parent).unwrap();
    let path = private_parent.join("output");
    fs::rename(original_path, &path).unwrap();
    let result = publish_outputs(&[(path.clone(), "new".into())]);
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("preflight native output policy"));
    assert!(same_original(
        &original,
        &observe(&path, false).unwrap().unwrap()
    ));
    assert_eq!(fs::read(path).unwrap(), b"original");
    assert_eq!(fs::read_dir(private_parent).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn windows_allow_before_owner_rights_denial_rejects_despite_successful_fresh_open() {
    let fixture = Fixture::new();
    let path = fixture.path("output.js");
    let mut file = windows_security::create_file(&path).unwrap();
    file.write_all(b"original").unwrap();
    let intended = windows_security::allow_then_owner_rights_denial(&file).unwrap();
    // Independent native precondition: the final policy actually permits the
    // production fresh open. Rejection must enforce the documented class,
    // rather than depending on the caller's observed effective permissions.
    let original = observe(&path, false).unwrap().unwrap();
    assert_eq!(original.policy, intended);
    let mut original_mutated = false;
    let result = publish_with_hook(&[(path.clone(), "replacement".into())], &mut |phase, _| {
        if phase == Phase::BackupRemove {
            original_mutated = true;
        }
        Ok(())
    });
    assert!(result.is_err(), "OWNER RIGHTS denial accepted: {result:?}");
    assert!(
        !original_mutated,
        "unsupported policy discovered after original mutation"
    );
    assert!(same_original(
        &original,
        &observe(&path, false).unwrap().unwrap()
    ));
    assert_eq!(fs::read(&path).unwrap(), b"original");
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn windows_supported_denials_preserve_policy_and_replace_bytes() {
    for (mask, sid) in [
        (2, [0x0000_0101, 0x0300_0000, 4]),
        (0x0002_0000, [0x0000_0101, 0x0100_0000, 0]),
    ] {
        let fixture = Fixture::new();
        let path = fixture.path("output.js");
        let mut file = windows_security::create_file(&path).unwrap();
        file.write_all(b"original").unwrap();
        let intended = windows_security::allow_then_denial(&file, mask, sid).unwrap();
        assert_eq!(observe(&path, false).unwrap().unwrap().policy, intended);
        publish_outputs(&[(path.clone(), "replacement".into())]).unwrap();
        assert_eq!(observe(&path, false).unwrap().unwrap().policy, intended);
        assert_eq!(fs::read(&path).unwrap(), b"replacement");
        assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 1);
    }
}

#[cfg(windows)]
fn windows_inherited_denial_setup(
    restricted: &Path,
    child_path: Option<&std::ffi::OsStr>,
) -> std::process::Output {
    use std::os::windows::process::CommandExt;
    let script = r#"
        $ErrorActionPreference='Stop'
        $acl=Get-Acl -LiteralPath $env:CLOSUREC_TEST_ACL_PATH
        $sid=[Security.Principal.SecurityIdentifier]::new('S-1-3-4')
        $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid,'ReadPermissions','ObjectInherit','InheritOnly','Deny'))
        Set-Acl -LiteralPath $env:CLOSUREC_TEST_ACL_PATH -AclObject $acl
        Write-Output ('parent_sddl='+(Get-Acl -LiteralPath $env:CLOSUREC_TEST_ACL_PATH).Sddl)
        whoami.exe /all
    "#;
    let mut command = std::process::Command::new("powershell.exe");
    command.args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env("CLOSUREC_TEST_ACL_PATH", restricted)
        .env_remove("PSModulePath")
        .creation_flags(0x0800_0000);
    if let Some(path) = child_path { command.env("PATH", path); }
    command.output().unwrap()
}

#[cfg(windows)]
#[test]
fn windows_acl_fixture_diagnostics_ignore_path_shadow_and_keep_setup_errors() {
    let fixture = Fixture::new();
    let shadow = fixture.path("shadow-bin");
    fs::create_dir(&shadow).unwrap();
    // A copied Rust test executable rejects /all, like Git's Unix utility.
    // It is a deterministic PATH shadow and requires no Git installation.
    let shadow_exe = shadow.join("whoami.exe");
    fs::copy(std::env::current_exe().unwrap(), &shadow_exe).unwrap();
    let inherited_path = std::env::var_os("PATH").unwrap();
    let child_path = std::env::join_paths(std::iter::once(shadow.clone())
        .chain(std::env::split_paths(&inherited_path))).unwrap();
    let restricted = fixture.path("restricted");
    fs::create_dir(&restricted).unwrap();
    let output = windows_inherited_denial_setup(&restricted, Some(&child_path));
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let diagnostics = String::from_utf8_lossy(&output.stdout);
    assert!(diagnostics.contains("native_whoami_exit=0"), "{diagnostics}");
    assert!(!diagnostics.contains(shadow_exe.to_str().unwrap()));
    let parent = observe(&restricted, true).unwrap().unwrap();
    let intended = windows_security::new_file_policy(&parent.file).unwrap();
    assert!(intended.check_verification_policy().is_err());
    // The diagnostic's success must not turn an actual ACL setup failure into
    // success. This also checks that the explicit script exit is correctly scoped.
    let failed_setup = windows_inherited_denial_setup(&fixture.path("missing"), Some(&child_path));
    assert!(!failed_setup.status.success());
}

#[cfg(windows)]
#[test]
fn windows_final_verification_denial_rejects_before_any_original_mutation() {
    let fixture = Fixture::new();
    let restricted = fixture.path("restricted");
    fs::create_dir(&restricted).unwrap();
    let output = windows_inherited_denial_setup(&restricted, None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parent = observe(&restricted, true).unwrap().unwrap();
    let intended = windows_security::new_file_policy(&parent.file).unwrap();
    let probe_path = fixture.path("empty-policy-diagnostic");
    let probe = windows_security::create_file(&probe_path).unwrap();
    let private = windows_security::Policy::capture(&probe).unwrap();
    let public_probe = restricted.join("empty-policy-diagnostic");
    fs::hard_link(&probe_path, &public_probe).unwrap();
    intended
        .apply(&windows_security::policy_handle(&public_probe).unwrap())
        .unwrap();
    let fresh = observe(&public_probe, false);
    let fresh_diagnostic = fresh.as_ref().map(|value| {
        value
            .as_ref()
            .map(|observed| format!("len={}; policy={:?}", observed.len, observed.policy))
    });
    eprintln!(
        "inherited OWNER RIGHTS fixture: {}; parent={:?}; derived={intended:?}; fresh_open={fresh_diagnostic:?}\n{}",
        windows_security::token_owner_diagnostics().unwrap(),
        parent.policy,
        String::from_utf8_lossy(&output.stdout)
    );
    private.apply(&probe).unwrap();
    fs::remove_file(&public_probe).unwrap();
    fs::remove_file(&probe_path).unwrap();
    let old = fixture.path("old.js");
    fs::write(&old, "original").unwrap();
    let original = observe(&old, false).unwrap().unwrap();
    let new = restricted.join("new.js");
    let mut original_mutated = false;
    let result = publish_with_hook(
        &[
            (old.clone(), "new first".into()),
            (new.clone(), "new second".into()),
        ],
        &mut |phase, _| {
            if phase == Phase::BackupRemove {
                original_mutated = true;
            }
            Ok(())
        },
    );
    assert!(result.is_err());
    assert!(
        !original_mutated,
        "unsupported final verification policy was discovered after original mutation"
    );
    assert!(same_original(
        &original,
        &observe(&old, false).unwrap().unwrap()
    ));
    assert_eq!(fs::read(old).unwrap(), b"original");
    assert!(!new.exists());
    assert_eq!(fs::read_dir(restricted).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 2);
}

#[test]
fn held_object_id_distinguishes_same_bytes_and_detects_hard_links() {
    let fixture = Fixture::new();
    let a = fixture.path("a");
    let b = fixture.path("b");
    let alias = fixture.path("alias");
    fs::write(&a, "same").unwrap();
    fs::write(&b, "same").unwrap();
    fs::hard_link(&a, &alias).unwrap();
    let first = observe(&a, false).unwrap().unwrap();
    assert_ne!(first.id, observe(&b, false).unwrap().unwrap().id);
    assert_eq!(first.id, observe(&alias, false).unwrap().unwrap().id);
    fs::remove_file(&a).unwrap();
    fs::write(&a, "same").unwrap();
    assert_ne!(first.id, observe(&a, false).unwrap().unwrap().id);
}

#[test]
fn later_install_failure_rolls_back_old_and_new_destinations() {
    let fixture = Fixture::new();
    let a = fixture.path("a");
    let b = fixture.path("b");
    let c = fixture.path("c");
    fs::write(&a, "old a").unwrap();
    fs::write(&c, "old c").unwrap();
    let error = publish_with_hook(
        &[
            (a.clone(), "new a".into()),
            (b.clone(), "new b".into()),
            (c.clone(), "new c".into()),
        ],
        &mut |phase, path| {
            if phase == Phase::Install && path.file_name() == c.file_name() {
                Err(injected())
            } else {
                Ok(())
            }
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("injected filesystem failure"));
    assert_eq!(fs::read(&a).unwrap(), b"old a");
    assert_eq!(fs::read(&c).unwrap(), b"old c");
    assert!(!b.exists());
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 2);
}

#[test]
fn staging_failure_removes_only_owned_created_parents() {
    let fixture = Fixture::new();
    let a = fixture.path("new/deep/a");
    let b = fixture.path("b");
    fs::write(&b, "old b").unwrap();
    let result = publish_with_hook(
        &[(a, "new a".into()), (b.clone(), "new b".into())],
        &mut |phase, path| {
            if phase == Phase::Stage && path.file_name() == b.file_name() {
                Err(injected())
            } else {
                Ok(())
            }
        },
    );
    assert!(result.is_err());
    assert_eq!(fs::read(&b).unwrap(), b"old b");
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 1);
}

#[test]
fn obstructed_restore_reports_and_retains_original_recovery_bytes() {
    let fixture = Fixture::new();
    let a = fixture.path("a");
    let b = fixture.path("b");
    fs::write(&a, "old a").unwrap();
    fs::write(&b, "old b").unwrap();
    let result = publish_with_hook(
        &[(a.clone(), "new a".into()), (b.clone(), "new b".into())],
        &mut |phase, path| {
            if (phase == Phase::Install && path.file_name() == b.file_name())
                || (phase == Phase::Restore && path.file_name() == a.file_name())
            {
                Err(injected())
            } else {
                Ok(())
            }
        },
    )
    .unwrap_err();
    assert!(result.to_string().contains("retained recovery"));
    assert!(!a.exists());
    assert_eq!(fs::read(&b).unwrap(), b"old b");
    let recovery = fixture.recovery();
    assert_eq!(recovery.len(), 1);
    assert_eq!(fs::read(recovery[0].join("old")).unwrap(), b"old a");
}

#[test]
fn rollback_preserves_an_unknown_replacement_and_original_backup() {
    let fixture = Fixture::new();
    let a = fixture.path("a");
    let b = fixture.path("b");
    fs::write(&a, "old a").unwrap();
    fs::write(&b, "old b").unwrap();
    let result = publish_with_hook(
        &[(a.clone(), "new a".into()), (b.clone(), "new b".into())],
        &mut |phase, path| {
            if phase == Phase::Install && path.file_name() == b.file_name() {
                fs::remove_file(&a).unwrap();
                fs::write(&a, "unknown replacement").unwrap();
                Err(injected())
            } else {
                Ok(())
            }
        },
    )
    .unwrap_err();
    assert!(result.to_string().contains("unknown replacement"));
    assert_eq!(fs::read(&a).unwrap(), b"unknown replacement");
    assert_eq!(fs::read(&b).unwrap(), b"old b");
    let recovery = fixture.recovery();
    assert_eq!(recovery.len(), 1);
    assert_eq!(fs::read(recovery[0].join("old")).unwrap(), b"old a");
}

#[test]
fn cleanup_failure_after_commit_reports_warning_with_complete_new_outputs() {
    let fixture = Fixture::new();
    let a = fixture.path("a");
    fs::write(&a, "old").unwrap();
    let warnings = publish_with_hook(&[(a.clone(), "new".into())], &mut |phase, _| {
        if phase == Phase::Cleanup {
            Err(injected())
        } else {
            Ok(())
        }
    })
    .unwrap();
    assert_eq!(fs::read(&a).unwrap(), b"new");
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("publication committed"));
    let recovery = fixture.recovery();
    assert_eq!(recovery.len(), 1);
    assert_eq!(fs::read(recovery[0].join("old")).unwrap(), b"old");
}

#[test]
fn missing_ancestor_collisions_reject_before_creating_parent_directories() {
    let fixture = Fixture::new();
    let a = fixture.path("missing");
    let child = a.join("child");
    assert!(publish_outputs(&[(a, "a".into()), (child, "child".into())]).is_err());
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 0);
}

#[test]
fn checked_staging_sequence_exhaustion_does_not_wrap_or_change_state() {
    let counter = AtomicU64::new(u64::MAX - 1);
    assert_eq!(reserve_sequence(&counter).unwrap(), u64::MAX - 1);
    assert!(reserve_sequence(&counter).is_err());
    assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
}

#[test]
fn readonly_and_directory_destinations_reject_before_staging() {
    let fixture = Fixture::new();
    let readonly = fixture.path("readonly");
    fs::write(&readonly, "old").unwrap();
    let original_permissions = fs::metadata(&readonly).unwrap().permissions();
    let mut permissions = original_permissions.clone();
    permissions.set_readonly(true);
    fs::set_permissions(&readonly, permissions).unwrap();
    let result = publish_outputs(&[(readonly.clone(), "new".into())]);
    fs::set_permissions(&readonly, original_permissions).unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read(&readonly).unwrap(), b"old");
    let directory = fixture.path("directory");
    fs::create_dir(&directory).unwrap();
    assert!(publish_outputs(&[(directory, "body".into())]).is_err());
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 2);
}

#[test]
fn unexpected_backup_occupants_are_never_overwritten_or_removed() {
    for hard_link in [false, true] {
        let fixture = Fixture::new();
        let output = fixture.path("output");
        fs::write(&output, "original").unwrap();
        let result = publish_with_hook(&[(output.clone(), "new".into())], &mut |phase, _| {
            if phase == Phase::Install {
                let stage = fixture.recovery().pop().unwrap();
                if hard_link {
                    fs::hard_link(&output, stage.join("old"))?;
                } else {
                    fs::write(stage.join("old"), "unknown backup occupant")?;
                }
            }
            Ok(())
        });
        assert!(
            result.is_err(),
            "unexpected backup occupant was overwritten"
        );
        assert_eq!(fs::read(&output).unwrap(), b"original");
        let recovery = fixture.recovery();
        assert_eq!(recovery.len(), 1);
        let expected: &[u8] = if hard_link {
            b"original"
        } else {
            b"unknown backup occupant"
        };
        assert_eq!(fs::read(recovery[0].join("old")).unwrap(), expected);
        assert_eq!(fs::read_dir(&recovery[0]).unwrap().count(), 1);
    }
}

#[test]
fn failed_rollback_removal_does_not_claim_a_nonexistent_original_backup() {
    let fixture = Fixture::new();
    let a = fixture.path("a");
    let b = fixture.path("b");
    let error = publish_with_hook(
        &[(a.clone(), "new a".into()), (b.clone(), "new b".into())],
        &mut |phase, path| {
            if (phase == Phase::Install && path.file_name() == b.file_name())
                || (phase == Phase::RollbackRemove && path.file_name() == a.file_name())
            {
                Err(injected())
            } else {
                Ok(())
            }
        },
    )
    .unwrap_err();
    assert_eq!(fs::read(&a).unwrap(), b"new a");
    assert!(!b.exists());
    assert!(fixture.recovery().is_empty());
    assert!(!error.to_string().contains("retained recovery"));
    assert!(error.to_string().contains("no original backup was created"));
    assert!(error.to_string().contains("removal unconfirmed"));
}

#[test]
fn failed_original_removal_cleans_only_its_created_backup_link() {
    let fixture = Fixture::new();
    let output = fixture.path("output");
    fs::write(&output, "original").unwrap();
    let result = publish_with_hook(&[(output.clone(), "new".into())], &mut |phase, _| {
        if phase == Phase::BackupRemove {
            Err(injected())
        } else {
            Ok(())
        }
    });
    assert!(result.is_err());
    assert_eq!(fs::read(&output).unwrap(), b"original");
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn windows_prepared_objects_have_protected_current_user_only_access() {
    use std::os::windows::process::CommandExt;
    fn script(path: &Path, body: &str) -> serde_json::Value {
        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", body])
            .env("CLOSUREC_TEST_ACL_PATH", path)
            .env_remove("PSModulePath")
            .creation_flags(0x0800_0000)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    let fixture = Fixture::new();
    script(
        &fixture.dir,
        r#"
        $ErrorActionPreference='Stop'
        $acl=Get-Acl -LiteralPath $env:CLOSUREC_TEST_ACL_PATH
        $users=[Security.Principal.SecurityIdentifier]::new('S-1-5-32-545')
        $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($users,'ReadAndExecute','ContainerInherit,ObjectInherit','None','Allow'))
        Set-Acl -LiteralPath $env:CLOSUREC_TEST_ACL_PATH -AclObject $acl
        'true'
    "#,
    );
    let output = fixture.path("output");
    let mut policies = Vec::new();
    let result = publish_with_hook(
        &[(output.clone(), "private prepared bytes".into())],
        &mut |phase, _| {
            if phase == Phase::Install {
                let directory = fixture.recovery().pop().unwrap();
                for path in [&directory, &directory.join("new")] {
                    policies.push(script(path, r#"
                    $ErrorActionPreference='Stop'
                    $acl=Get-Acl -LiteralPath $env:CLOSUREC_TEST_ACL_PATH
                    $user=[Security.Principal.WindowsIdentity]::GetCurrent().User.Value
                    $rules=@($acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier]))
                    $otherAllows=@($rules | Where-Object { $_.AccessControlType -eq 'Allow' -and $_.IdentityReference.Value -ne $user })
                    [pscustomobject]@{ protected=$acl.AreAccessRulesProtected; other_allow_count=$otherAllows.Count } | ConvertTo-Json -Compress
                "#));
                }
                return Err(injected());
            }
            Ok(())
        },
    );
    assert!(result.is_err());
    assert!(!output.exists());
    assert!(fixture.recovery().is_empty());
    assert_eq!(policies.len(), 2);
    for policy in policies {
        assert_eq!(
            policy["protected"], true,
            "staging inherited a parent policy"
        );
        assert_eq!(
            policy["other_allow_count"], 0,
            "staging grants non-user access"
        );
    }
}

#[cfg(unix)]
#[test]
fn final_symlinks_reject_and_parent_symlink_aliases_collide() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let target = fixture.path("target");
    let link = fixture.path("link");
    fs::write(&target, "old").unwrap();
    symlink(&target, &link).unwrap();
    assert!(publish_outputs(&[(link, "new".into())]).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"old");
    let directory = fixture.path("parent");
    let alias = fixture.path("alias");
    fs::create_dir(&directory).unwrap();
    symlink(&directory, &alias).unwrap();
    assert!(publish_outputs(&[
        (directory.join("new"), "a".into()),
        (alias.join("new"), "b".into()),
    ])
    .is_err());
    assert_eq!(fs::read_dir(directory).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 4);
}

#[test]
fn directory_spelling_and_missing_parent_traversal_fail_before_creation() {
    let fixture = Fixture::new();
    for path in [fixture.path("out/"), fixture.path("missing/../out")] {
        assert!(publish_outputs(&[(path, "body".into())]).is_err());
    }
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 0);
}

#[cfg(windows)]
#[test]
fn windows_missing_case_ancestor_aliases_and_ambiguous_components_reject() {
    let fixture = Fixture::new();
    let a = fixture.path("missing");
    let child = fixture.path("MISSING/child");
    assert!(publish_outputs(&[(a, "a".into()), (child, "child".into())]).is_err());
    for name in [
        "out.",
        "missing./child",
        "CON",
        "LPT1.txt",
        "file:stream",
        "missing /child",
    ] {
        assert!(
            publish_outputs(&[(fixture.path(name), "body".into())]).is_err(),
            "{name}"
        );
    }
    assert_eq!(fs::read_dir(&fixture.dir).unwrap().count(), 0);
}
