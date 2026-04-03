package filesystem

import (
	"errors"
	"fmt"
	"strconv"
	"strings"
)

// ---------------------------------------------------------------------------
// DirectoryEntry --- the name-to-inode mapping
// ---------------------------------------------------------------------------
// A directory in Unix is just a special file whose data blocks contain a
// list of directory entries. Each entry is a (name, inode_number) pair.
// The entry says "the name 'notes.txt' corresponds to inode 23." All the
// actual metadata (size, permissions, timestamps) lives in the inode, not
// in the directory entry.
//
// Every directory must contain at least two entries:
//   - "."  -> the directory's own inode
//   - ".." -> the parent directory's inode
//
// For the root directory (/), both "." and ".." point to inode 0.
//
// Serialization format:
//
//	name:inode_number\n
//
// For example:
//
//	.:0\n
//	..:0\n
//	home:5\n

// DirectoryEntry maps a name to an inode number within a directory.
type DirectoryEntry struct {
	// Name is the file or directory name (up to MaxNameLength characters).
	// Must not contain '/' or null bytes.
	Name string

	// InodeNumber is the inode this name refers to.
	InodeNumber int
}

// NewDirectoryEntry creates a validated directory entry.
// Returns an error if the name is empty, too long, or contains forbidden
// characters (/ or null byte).
func NewDirectoryEntry(name string, inodeNumber int) (*DirectoryEntry, error) {
	return StartNew[*DirectoryEntry]("file-system.NewDirectoryEntry", nil,
		func(op *Operation[*DirectoryEntry], rf *ResultFactory[*DirectoryEntry]) *OperationResult[*DirectoryEntry] {
			op.AddProperty("name", name)
			op.AddProperty("inodeNumber", inodeNumber)
			if name == "" {
				return rf.Fail(nil, errors.New("directory entry name cannot be empty"))
			}
			if len(name) > MaxNameLength {
				return rf.Fail(nil, fmt.Errorf("directory entry name exceeds %d characters", MaxNameLength))
			}
			if strings.Contains(name, "/") {
				return rf.Fail(nil, errors.New("directory entry name cannot contain '/'"))
			}
			if strings.ContainsRune(name, 0) {
				return rf.Fail(nil, errors.New("directory entry name cannot contain null byte"))
			}
			return rf.Generate(true, false, &DirectoryEntry{Name: name, InodeNumber: inodeNumber})
		}).GetResult()
}

// Serialize converts this entry to its on-disk text representation.
// Format: "name:inode_number\n"
func (de *DirectoryEntry) Serialize() string {
	result, _ := StartNew[string]("file-system.DirectoryEntry.Serialize", "",
		func(op *Operation[string], rf *ResultFactory[string]) *OperationResult[string] {
			return rf.Generate(true, false, fmt.Sprintf("%s:%d\n", de.Name, de.InodeNumber))
		}).GetResult()
	return result
}

// DeserializeDirectoryEntry parses a directory entry from its on-disk
// text representation. The input should be in the format "name:inode_number"
// (newline optional).
func DeserializeDirectoryEntry(line string) (*DirectoryEntry, error) {
	return StartNew[*DirectoryEntry]("file-system.DeserializeDirectoryEntry", nil,
		func(op *Operation[*DirectoryEntry], rf *ResultFactory[*DirectoryEntry]) *OperationResult[*DirectoryEntry] {
			line = strings.TrimSpace(line)
			if line == "" {
				return rf.Fail(nil, errors.New("empty directory entry line"))
			}

			// Use LastIndex to handle names that might contain colons
			lastColon := strings.LastIndex(line, ":")
			if lastColon < 0 {
				return rf.Fail(nil, fmt.Errorf("invalid directory entry format: %q", line))
			}

			name := line[:lastColon]
			inodeStr := line[lastColon+1:]

			inodeNum, err := strconv.Atoi(inodeStr)
			if err != nil {
				return rf.Fail(nil, fmt.Errorf("invalid inode number %q: %w", inodeStr, err))
			}

			entry, err := NewDirectoryEntry(name, inodeNum)
			if err != nil {
				return rf.Fail(nil, err)
			}
			return rf.Generate(true, false, entry)
		}).GetResult()
}
