// Package filesystem implements a simplified inode-based file system inspired
// by ext2, the classic Linux file system.
//
// # What Is a File System?
//
// A file system is the abstraction that turns a raw disk --- billions of
// identical bytes with no structure --- into the familiar world of files and
// directories. Without a file system, every program would need to remember
// "my data starts at byte 4,194,304 and is 8,192 bytes long." With a file
// system, you just say open("/home/alice/notes.txt") and the OS figures out
// the rest.
//
// # Architecture
//
//	User Program
//	|   vfs.Open("/data/log.txt", O_RDWR)
//	|   vfs.Write(fd, []byte("hello"))
//	|   vfs.Close(fd)
//	v
//	VFS (this package)
//	|   Path Resolution, Inode Table, Block Bitmap,
//	|   Open File Table, Superblock
//	v
//	In-Memory Block Storage ([]byte)
//
// # Analogy
//
// Think of a library. The disk is the building full of shelves. The file
// system is the cataloging system --- the card catalog (inode table), the
// Dewey Decimal numbers (block pointers), the shelf labels (directories),
// and the checkout desk (file descriptors).
package filesystem

// ---------------------------------------------------------------------------
// Block and inode geometry
// ---------------------------------------------------------------------------

const (
	// BlockSize is the size of each block on our simulated disk, in bytes.
	// We use 512 bytes --- the traditional hard-disk sector size. Every
	// read and write operates in units of exactly this many bytes.
	BlockSize = 512

	// MaxBlocks is the total number of blocks on our simulated disk.
	// 512 blocks * 512 bytes = 262,144 bytes = 256 KB.
	MaxBlocks = 512

	// MaxInodes is the maximum number of inodes (files + directories)
	// the file system can hold.
	MaxInodes = 128

	// DirectBlocks is the number of direct block pointers in each inode.
	// With 512-byte blocks, this allows files up to 12 * 512 = 6,144 bytes
	// without needing any indirection.
	DirectBlocks = 12

	// RootInode is the inode number of the root directory "/".
	// This is always 0 by convention.
	RootInode = 0

	// MaxNameLength is the maximum length of a file or directory name.
	MaxNameLength = 255
)

// ---------------------------------------------------------------------------
// Open flags --- how a file is opened
// ---------------------------------------------------------------------------
// These mirror the POSIX constants from <fcntl.h>. They can be combined
// with bitwise OR, e.g., O_WRONLY | O_CREAT | O_APPEND.

const (
	// O_RDONLY opens a file for reading only.
	O_RDONLY = 0

	// O_WRONLY opens a file for writing only.
	O_WRONLY = 1

	// O_RDWR opens a file for both reading and writing.
	O_RDWR = 2

	// O_CREAT creates the file if it does not exist.
	O_CREAT = 64

	// O_TRUNC truncates the file to zero length on open.
	O_TRUNC = 512

	// O_APPEND positions the offset at end-of-file before each write.
	O_APPEND = 1024
)

// ---------------------------------------------------------------------------
// Seek whence --- where to measure from when repositioning
// ---------------------------------------------------------------------------

const (
	// SeekSet sets the offset to exactly the given value.
	SeekSet = 0

	// SeekCur sets the offset relative to the current position.
	SeekCur = 1

	// SeekEnd sets the offset relative to the end of the file.
	SeekEnd = 2
)
