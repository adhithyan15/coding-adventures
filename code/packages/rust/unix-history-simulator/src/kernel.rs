use crate::{Descriptor, DescriptorTable, Error, FileSystem, Node};

/// Owns the dispatch policy and connects independently usable mechanisms.
pub struct Kernel {
    files: FileSystem,
    descriptors: DescriptorTable,
}

impl Kernel {
    pub fn new(files: FileSystem, descriptors: DescriptorTable) -> Self {
        Self { files, descriptors }
    }

    pub fn open(&mut self, path: &str) -> Result<(usize, usize), Error> {
        let (inode, node) = self.files.resolve(path)?;
        if matches!(node, Node::Directory(_)) {
            return Err("EISDIR");
        }
        Ok((self.descriptors.open(inode).ok_or("EMFILE")?, inode))
    }

    pub fn read(&mut self, fd: usize, count: usize) -> Result<(Descriptor, &[u8]), Error> {
        let slot = self.descriptors.get(fd).ok_or("EBADF")?;
        let bytes = self.files.read(slot.inode, slot.offset, count)?;
        self.descriptors.advance(fd, bytes.len());
        Ok((slot, bytes))
    }

    pub fn close(&mut self, fd: usize) -> Result<(), Error> {
        self.descriptors.close(fd).then_some(()).ok_or("EBADF")
    }

    pub fn list(&self, path: &str) -> Result<(usize, Vec<&str>), Error> {
        let (inode, node) = self.files.resolve(path)?;
        let Node::Directory(entries) = node else {
            return Err("ENOTDIR");
        };
        Ok((
            inode,
            entries.iter().map(|(name, _)| name.as_str()).collect(),
        ))
    }
}
