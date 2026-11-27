package unixhistory

type Event map[string]any

type Kernel struct {
	Files       FileSystem
	Descriptors *DescriptorTable
}

func NewKernel(files FileSystem, descriptors *DescriptorTable) *Kernel {
	return &Kernel{Files: files, Descriptors: descriptors}
}

func (k *Kernel) Open(path string) Event {
	event := Event{"call": "open", "path": path}
	file, code := k.Files.Resolve(path)
	if code != "" {
		event["error"] = code
	} else if file.Directory {
		event["error"] = "EISDIR"
	} else if fd, ok := k.Descriptors.Open(file.Number); ok {
		event["fd"], event["inode"], event["offset"] = fd, file.Number, 0
	} else {
		event["error"] = "EMFILE"
	}
	return event
}

func (k *Kernel) Read(fd, count int) (Event, string) {
	event := Event{"call": "read", "fd": fd}
	slot, ok := k.Descriptors.Get(fd)
	if !ok {
		event["error"] = "EBADF"
		return event, ""
	}
	data, code := k.Files.Read(slot.Inode, slot.Offset, count)
	if code != "" {
		event["error"] = code
		return event, ""
	}
	k.Descriptors.Advance(fd, len(data))
	event["inode"], event["offset"] = slot.Inode, slot.Offset
	event["bytes"], event["nextOffset"] = len(data), slot.Offset+len(data)
	return event, data
}

func (k *Kernel) Close(fd int) Event {
	event := Event{"call": "close", "fd": fd}
	if !k.Descriptors.Close(fd) {
		event["error"] = "EBADF"
	}
	return event
}

func (k *Kernel) List(path string) ([]string, Event) {
	event := Event{"call": "list", "path": path}
	directory, code := k.Files.Resolve(path)
	if code != "" {
		event["error"] = code
		return nil, event
	}
	if !directory.Directory {
		event["error"] = "ENOTDIR"
		return nil, event
	}
	names := make([]string, 0, len(directory.Entries))
	for _, entry := range directory.Entries {
		names = append(names, entry.Name)
	}
	event["names"], event["inode"] = names, directory.Number
	return names, event
}
