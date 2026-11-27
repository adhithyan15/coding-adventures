// Package unixhistory implements a small, deterministic early Unix file read.
package unixhistory

import "strings"

type Entry struct {
	Name  string
	Inode int
}

type Inode struct {
	Number    int
	Directory bool
	Entries   []Entry
	Data      string
}

type FileSystem map[int]Inode

func DemoImage() FileSystem {
	return FileSystem{
		1: {Number: 1, Directory: true, Entries: []Entry{{"hello", 2}, {"empty", 3}, {"etc", 4}}},
		2: {Number: 2, Data: "Hello, Unix!\n"},
		3: {Number: 3},
		4: {Number: 4, Directory: true, Entries: []Entry{{"notes", 5}}},
		5: {Number: 5, Data: "A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n"},
	}
}

// Resolve walks entries in order. A file may never be used as a directory,
// even when the next component is "." or "..".
func (fs FileSystem) Resolve(path string) (Inode, string) {
	if path == "" || strings.ContainsRune(path, 0) {
		return Inode{}, "EINVAL"
	}
	root, ok := fs[1]
	if !ok {
		return Inode{}, "ENOENT"
	}
	current := root
	var parents []Inode
	for _, name := range strings.Split(path, "/") {
		if name == "" {
			continue
		}
		if !current.Directory {
			return Inode{}, "ENOTDIR"
		}
		switch name {
		case ".":
			continue
		case "..":
			if len(parents) != 0 {
				current = parents[len(parents)-1]
				parents = parents[:len(parents)-1]
			}
		default:
			next, found := fs.child(current, name)
			if !found {
				return Inode{}, "ENOENT"
			}
			parents = append(parents, current)
			current = next
		}
	}
	if strings.HasSuffix(path, "/") && !current.Directory {
		return Inode{}, "ENOTDIR"
	}
	return current, ""
}

func (fs FileSystem) child(directory Inode, name string) (Inode, bool) {
	for _, entry := range directory.Entries {
		if entry.Name == name {
			node, exists := fs[entry.Inode]
			return node, exists
		}
	}
	return Inode{}, false
}

func (fs FileSystem) Read(inode, offset, count int) (string, string) {
	file, exists := fs[inode]
	switch {
	case offset < 0 || count < 0:
		return "", "EINVAL"
	case !exists:
		return "", "ENOENT"
	case file.Directory:
		return "", "EISDIR"
	case offset >= len(file.Data):
		return "", ""
	default:
		return file.Data[offset : offset+min(count, len(file.Data)-offset)], ""
	}
}
