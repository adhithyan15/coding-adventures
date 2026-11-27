package unixhistory

// Descriptor owns its offset. Two slots can refer to the same inode.
type Descriptor struct {
	Inode  int
	Offset int
}

type DescriptorTable struct {
	slots []*Descriptor
}

func NewDescriptorTable(capacity int) *DescriptorTable {
	return &DescriptorTable{slots: make([]*Descriptor, capacity)}
}

func (table *DescriptorTable) Get(fd int) (Descriptor, bool) {
	if fd < 0 || fd >= len(table.slots) || table.slots[fd] == nil {
		return Descriptor{}, false
	}
	return *table.slots[fd], true
}

func (table *DescriptorTable) Open(inode int) (int, bool) {
	for fd, slot := range table.slots {
		if slot == nil {
			table.slots[fd] = &Descriptor{Inode: inode}
			return fd, true
		}
	}
	return 0, false
}

func (table *DescriptorTable) Advance(fd, count int) bool {
	slot, ok := table.Get(fd)
	if !ok || count < 0 || slot.Offset+count < slot.Offset {
		return false
	}
	table.slots[fd].Offset += count
	return true
}

func (table *DescriptorTable) Close(fd int) bool {
	if _, ok := table.Get(fd); !ok {
		return false
	}
	table.slots[fd] = nil
	return true
}
