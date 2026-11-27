use serde_json::{json, Value};
use unix_history_simulator::{run_command, Descriptor, DescriptorTable, FileSystem, Kernel};

#[test]
fn shared_traces() {
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../specs/fixtures/historical-unix-v1/file-read.json"
    ))
    .unwrap();
    for fixture in fixtures {
        let command = fixture["command"].as_str().unwrap();
        assert_eq!(
            run_command(command),
            json!({"output": fixture["output"], "events": fixture["events"]}),
            "{command:?}"
        );
    }
}

#[test]
fn lowest_slot_and_independent_offsets() {
    let mut table = DescriptorTable::new(2);
    assert_eq!(table.open(2), Some(0));
    assert_eq!(table.open(2), Some(1));
    assert_eq!(table.open(2), None);
    assert!(table.advance(0, 4));
    assert_eq!(
        table.get(1),
        Some(Descriptor {
            inode: 2,
            offset: 0
        })
    );
    assert!(!table.close(2));
    assert!(table.close(0));
    assert_eq!(table.open(3), Some(0));
    assert_eq!(
        table.get(0),
        Some(Descriptor {
            inode: 3,
            offset: 0
        })
    );
}

#[test]
fn invalid_lookup_and_syscalls() {
    let mut kernel = Kernel::new(FileSystem::demo(), DescriptorTable::new(1));
    assert_eq!(kernel.open("/hello/"), Err("ENOTDIR"));
    assert_eq!(kernel.open("he\0llo"), Err("EINVAL"));
    assert_eq!(kernel.open("/"), Err("EISDIR"));
    assert_eq!(kernel.read(0, 1), Err("EBADF"));
    assert_eq!(kernel.open("/hello"), Ok((0, 2)));
    assert_eq!(kernel.open("/empty"), Err("EMFILE"));
    assert_eq!(kernel.read(0, 2).unwrap().1, b"He");
}
