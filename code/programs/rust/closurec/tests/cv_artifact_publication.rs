//! Real-process CV02 publication failures must preserve the whole output set.
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture {
    dir: PathBuf,
    input: PathBuf,
    paths: [PathBuf; 4],
}
impl Fixture {
    fn new() -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "closurec-publication-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let input = dir.join("a.js");
        std::fs::write(&input, "report(1+2);").unwrap();
        let paths = ["out.js", "out.map", "manifest.txt", "trace.json"].map(|name| dir.join(name));
        for (index, path) in paths.iter().enumerate() {
            std::fs::write(path, format!("original {index}")).unwrap();
        }
        std::fs::write(dir.join("unrelated.txt"), "keep me").unwrap();
        Self { dir, input, paths }
    }
    fn run(&self, paths: &[PathBuf; 4], traced: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_closurec"));
        command
            .arg("--js")
            .arg(&self.input)
            .args(["--compilation_level", "SIMPLE"])
            .arg("--js_output_file")
            .arg(&paths[0])
            .arg("--create_source_map")
            .arg(&paths[1])
            .arg("--output_manifest")
            .arg(&paths[2]);
        if traced {
            command
                .arg("--correlation_vector")
                .arg("--correlation_vector_output")
                .arg(&paths[3]);
        }
        command.current_dir(&self.dir).output().unwrap()
    }
    fn snapshot(&self) -> Vec<Vec<u8>> {
        self.paths
            .iter()
            .map(|p| std::fs::read(p).unwrap())
            .collect()
    }
    fn preserved(&self, before: &[Vec<u8>], file_count: usize) {
        assert_eq!(self.snapshot(), before);
        assert_eq!(
            std::fs::read(self.dir.join("unrelated.txt")).unwrap(),
            b"keep me"
        );
        assert_eq!(
            std::fs::read_dir(&self.dir).unwrap().count(),
            file_count,
            "owned temporary paths leaked"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert_eq!(self.dir.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(&self.dir).unwrap();
    }
}

#[test]
fn colliding_output_destinations_reject_before_writing_any_file() {
    for mode in 0..4 {
        let fixture = Fixture::new();
        let mut paths = fixture.paths.clone();
        match mode {
            0 => paths[3] = paths[0].clone(),
            1 => paths[3] = fixture.dir.join(".").join("out.js"),
            2 => paths[1] = paths[0].join("nested.map"),
            3 => {
                std::fs::remove_file(&paths[1]).unwrap();
                std::fs::hard_link(&paths[0], &paths[1]).unwrap();
            }
            _ => unreachable!(),
        }
        let before = fixture.snapshot();
        let output = fixture.run(&paths, true);
        assert_eq!(
            output.status.code(),
            Some(2),
            "mode {mode}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
        fixture.preserved(&before, 6);
    }
}

#[test]
fn later_staging_failure_preserves_all_existing_outputs() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let blocked = fixture.dir.join("blocked");
    std::fs::write(&blocked, "unrelated blocker").unwrap();
    let mut paths = fixture.paths.clone();
    paths[3] = blocked.join("trace.json");
    let output = fixture.run(&paths, true);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    fixture.preserved(&before, 7);
    assert_eq!(std::fs::read(blocked).unwrap(), b"unrelated blocker");
}

#[test]
fn committed_trace_identifies_actual_input_and_output_content() {
    let fixture = Fixture::new();
    let plain = fixture.run(&fixture.paths, false);
    assert!(plain.status.success());
    let expected: Vec<_> = fixture.paths[..3]
        .iter()
        .map(|p| std::fs::read(p).unwrap())
        .collect();
    let output = fixture.run(&fixture.paths, true);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (path, bytes) in fixture.paths[..3].iter().zip(&expected) {
        assert_eq!(std::fs::read(path).unwrap(), *bytes);
    }
    let log: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&fixture.paths[3]).unwrap()).unwrap();
    let entries = log["entries"].as_object().unwrap();
    for (index, source) in ["js_output_file", "source_map_output", "manifest_output"]
        .iter()
        .enumerate()
    {
        let entry = entries
            .values()
            .find(|e| e["origin"]["source"] == *source)
            .unwrap();
        let event = entry["contributions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["source"] == "write_output_file" && e["tag"] == "wrote")
            .unwrap();
        assert_eq!(event["meta"]["byte_len"], expected[index].len());
        assert_eq!(
            event["meta"]["content_sha256"],
            coding_adventures_sha256::sha256_hex(&expected[index])
        );
    }
    let input = entries
        .values()
        .find(|e| e["origin"]["source"] == "input_file")
        .unwrap();
    assert_eq!(
        input["origin"]["meta"]["content_sha256"],
        coding_adventures_sha256::sha256_hex(&std::fs::read(&fixture.input).unwrap())
    );
    assert_eq!(std::fs::read_dir(&fixture.dir).unwrap().count(), 6);
}

#[cfg(windows)]
#[test]
fn committed_outputs_preserve_protected_owner_only_access_controls() {
    use std::os::windows::process::CommandExt;
    fn policies(fixture: &Fixture, setup: bool) -> serde_json::Value {
        let script = r#"
            $ErrorActionPreference='Stop'
            $root=$env:CLOSUREC_TEST_ACL_ROOT
            $user=[Security.Principal.WindowsIdentity]::GetCurrent().User
            if ($env:CLOSUREC_TEST_ACL_SETUP -eq 'true') {
                $acl=Get-Acl -LiteralPath $root
                $users=[Security.Principal.SecurityIdentifier]::new('S-1-5-32-545')
                $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($users,'ReadAndExecute','ContainerInherit,ObjectInherit','None','Allow'))
                Set-Acl -LiteralPath $root -AclObject $acl
                foreach ($name in @('out.js','out.map','manifest.txt','trace.json')) {
                    $path=Join-Path $root $name
                    $acl=Get-Acl -LiteralPath $path
                    $acl.SetAccessRuleProtection($true,$false)
                    $acl.SetAccessRule([Security.AccessControl.FileSystemAccessRule]::new($user,'FullControl','Allow'))
                    Set-Acl -LiteralPath $path -AclObject $acl
                }
            }
            $result=@(foreach ($name in @('out.js','out.map','manifest.txt','trace.json')) {
                $acl=Get-Acl -LiteralPath (Join-Path $root $name)
                $rules=@($acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier]) | ForEach-Object {
                    [pscustomobject]@{ sid=$_.IdentityReference.Value; rights=[int]$_.FileSystemRights; type=[int]$_.AccessControlType; inherited=$_.IsInherited; inheritance=[int]$_.InheritanceFlags; propagation=[int]$_.PropagationFlags }
                })
                [pscustomobject]@{ owner=$acl.GetOwner([Security.Principal.SecurityIdentifier]).Value; group=$acl.GetGroup([Security.Principal.SecurityIdentifier]).Value; protected=$acl.AreAccessRulesProtected; rules=$rules }
            })
            ConvertTo-Json -InputObject $result -Depth 8 -Compress
        "#;
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .env("CLOSUREC_TEST_ACL_ROOT", &fixture.dir)
            .env("CLOSUREC_TEST_ACL_SETUP", setup.to_string())
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
    let expected = policies(&fixture, true);
    let output = fixture.run(&fixture.paths, true);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        policies(&fixture, false),
        expected,
        "publication broadened protected output access"
    );
    assert_eq!(std::fs::read_dir(&fixture.dir).unwrap().count(), 6);
}
