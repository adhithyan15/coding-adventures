#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Descriptor {
    pub inode: usize,
    pub offset: usize,
}

pub struct DescriptorTable {
    slots: Vec<Option<Descriptor>>,
}

impl DescriptorTable {
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: vec![None; capacity],
        }
    }

    pub fn get(&self, fd: usize) -> Option<Descriptor> {
        self.slots.get(fd).copied().flatten()
    }

    pub fn open(&mut self, inode: usize) -> Option<usize> {
        let fd = self.slots.iter().position(Option::is_none)?;
        self.slots[fd] = Some(Descriptor { inode, offset: 0 });
        Some(fd)
    }

    pub fn advance(&mut self, fd: usize, count: usize) -> bool {
        let Some(slot) = self.slots.get_mut(fd).and_then(Option::as_mut) else {
            return false;
        };
        let Some(offset) = slot.offset.checked_add(count) else {
            return false;
        };
        slot.offset = offset;
        true
    }

    pub fn close(&mut self, fd: usize) -> bool {
        self.slots.get_mut(fd).and_then(Option::take).is_some()
    }
}
