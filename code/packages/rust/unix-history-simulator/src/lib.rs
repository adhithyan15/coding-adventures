//! Original First Edition Unix file-read model with a portable trace.
mod descriptors;
mod filesystem;
mod kernel;

pub use descriptors::{Descriptor, DescriptorTable};
pub use filesystem::{Error, FileSystem, Node};
pub use kernel::Kernel;
use serde_json::{json, Value};

pub fn run_command(command: &str) -> Value {
    let mut kernel = Kernel::new(FileSystem::demo(), DescriptorTable::new(4));
    let mut events = Vec::new();
    let mut output = String::new();
    let words: Vec<_> = command
        .split([' ', '\t', '\n', '\r', '\u{c}', '\u{b}'])
        .filter(|word| !word.is_empty())
        .collect();
    match words.as_slice() {
        ["cat", path] => match kernel.open(path) {
            Ok((fd, inode)) => {
                events.push(json!({"call":"open","path":path,"fd":fd,"inode":inode,"offset":0}));
                let mut data = Vec::new();
                loop {
                    match kernel.read(fd, 8) {
                        Ok((slot, bytes)) => {
                            events.push(json!({"call":"read","fd":fd,"inode":slot.inode,
                                "offset":slot.offset,"bytes":bytes.len(),"nextOffset":slot.offset+bytes.len()}));
                            if bytes.is_empty() {
                                break;
                            }
                            data.extend(bytes);
                        }
                        Err(code) => {
                            events.push(json!({"call":"read","fd":fd,"error":code}));
                            break;
                        }
                    }
                }
                output.push_str(&String::from_utf8_lossy(&data));
                events.push(match kernel.close(fd) {
                    Ok(()) => json!({"call":"close","fd":fd}),
                    Err(code) => json!({"call":"close","fd":fd,"error":code}),
                });
            }
            Err(code) => events.push(json!({"call":"open","path":path,"error":code})),
        },
        ["ls"] | ["ls", _] => {
            let path = words.get(1).copied().unwrap_or("/");
            match kernel.list(path) {
                Ok((inode, names)) => {
                    for name in &names {
                        output.push_str(name);
                        output.push('\n');
                    }
                    events.push(json!({"call":"list","path":path,"inode":inode,"names":names}));
                }
                Err(code) => events.push(json!({"call":"list","path":path,"error":code})),
            }
        }
        [] => {}
        _ => output.push_str("usage: cat PATH or ls [PATH]\n"),
    }
    for code in events.iter().filter_map(|event| event["error"].as_str()) {
        output.push_str("error: ");
        output.push_str(code);
        output.push('\n');
    }
    json!({"output":output,"events":events})
}
