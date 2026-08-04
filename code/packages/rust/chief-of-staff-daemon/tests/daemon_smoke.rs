#![cfg(unix)]

use std::fs;
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "chief-daemon-smoke-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn wait_until_listening(child: &mut Child, port: u16) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        if let Some(status) = child.try_wait().unwrap() {
            let mut stderr = String::new();
            child
                .stderr
                .as_mut()
                .unwrap()
                .read_to_string(&mut stderr)
                .unwrap();
            panic!("daemon exited before binding ({status}): {stderr}");
        }
        assert!(Instant::now() < deadline, "daemon did not bind in time");
        thread::sleep(Duration::from_millis(20));
    }
}

fn wait_for_exit(child: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        assert!(Instant::now() < deadline, "daemon did not stop in time");
        thread::sleep(Duration::from_millis(20));
    }
}

fn write_config(home: &Path, port: u16) -> PathBuf {
    fs::create_dir(home.join("run")).unwrap();
    fs::create_dir(home.join("keys")).unwrap();
    let public_key = coding_adventures_ed25519::generate_keypair(&[7; 32]).0;
    fs::write(home.join("keys/production.pub"), public_key).unwrap();
    let config = format!(
        r#"
[orchestrator]
bind = "127.0.0.1"
port = {port}
packages_dir = "~/agents"
state_dir = "~/state"
credential_path = "~/run/operator.credential"

[keyring]
trusted_keys = [
  {{ id = "prod-001", path = "~/keys/production.pub", type = "production" }},
]

[hosts.defaults]
restart_policy = "on-failure"
health_check_interval = 20
executable = "~/bin/chief-of-staff-host"
bootstrap_timeout = 1000
graceful_stop_timeout = 1000

[vault]
storage_path = "~/vault"
default_lease_ttl = 30
container = true

[privilege]
tier_1_auto_approve_timeout = 5
biometric_timeout = 30
hardware_key_timeout = 60
"#
    );
    let path = home.join("config.toml");
    fs::write(&path, config).unwrap();
    path
}

#[test]
fn daemon_binds_reconciles_and_stops_on_sigterm() {
    let directory = TestDir::new();
    let port = TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let config = write_config(&directory.0, port);
    let mut child = Command::new(env!("CARGO_BIN_EXE_chief-of-staff-daemon"))
        .arg(config)
        .env("HOME", &directory.0)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    wait_until_listening(&mut child, port);
    // SAFETY: `child.id()` is the live subprocess created above, and SIGTERM is
    // installed by that subprocess's process-shutdown listener.
    assert_eq!(unsafe { libc::kill(child.id() as i32, libc::SIGTERM) }, 0);
    let status = wait_for_exit(&mut child);
    let mut stderr = String::new();
    child
        .stderr
        .as_mut()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(status.success(), "daemon failed ({status}): {stderr}");
    assert!(directory.0.join("run/operator.credential").is_file());
    assert!(directory.0.join("state").is_dir());
}
