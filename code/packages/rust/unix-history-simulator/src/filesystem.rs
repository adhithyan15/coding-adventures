use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub enum Node {
    Directory(Vec<(String, usize)>),
    File(Vec<u8>),
}

pub type Error = &'static str;

pub struct FileSystem {
    nodes: BTreeMap<usize, Node>,
}

impl FileSystem {
    pub fn new(nodes: BTreeMap<usize, Node>) -> Self {
        Self { nodes }
    }

    pub fn demo() -> Self {
        Self::new(BTreeMap::from([
            (1, Node::Directory(vec![("hello".into(), 2), ("empty".into(), 3), ("etc".into(), 4)])),
            (2, Node::File(b"Hello, Unix!\n".to_vec())),
            (3, Node::File(Vec::new())),
            (4, Node::Directory(vec![("notes".into(), 5)])),
            (5, Node::File(b"A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n".to_vec())),
        ]))
    }

    /// Resolves names against the root and checks each intermediate inode.
    pub fn resolve(&self, path: &str) -> Result<(usize, &Node), Error> {
        if path.is_empty() || path.contains('\0') {
            return Err("EINVAL");
        }
        let mut current = 1;
        let mut parents = Vec::new();
        for name in path.split('/').filter(|name| !name.is_empty()) {
            let Node::Directory(entries) = self.nodes.get(&current).ok_or("ENOENT")? else {
                return Err("ENOTDIR");
            };
            match name {
                "." => {}
                ".." => current = parents.pop().unwrap_or(1),
                _ => {
                    let (_, inode) = entries
                        .iter()
                        .find(|(entry, _)| entry == name)
                        .ok_or("ENOENT")?;
                    parents.push(current);
                    current = *inode;
                }
            }
        }
        let node = self.nodes.get(&current).ok_or("ENOENT")?;
        if path.ends_with('/') && matches!(node, Node::File(_)) {
            return Err("ENOTDIR");
        }
        Ok((current, node))
    }

    pub fn read(&self, inode: usize, offset: usize, count: usize) -> Result<&[u8], Error> {
        let Node::File(bytes) = self.nodes.get(&inode).ok_or("ENOENT")? else {
            return Err("EISDIR");
        };
        let start = offset.min(bytes.len());
        let end = start + count.min(bytes.len() - start);
        Ok(&bytes[start..end])
    }
}
