//! Deterministic filesystem faults exercise states normal process tests cannot.
use super::*;

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
