# frozen_string_literal: true

# = CodingAdventures::FileSystem
#
# A simplified inode-based file system inspired by ext2, the classic Linux
# file system. This package implements the Virtual File System (VFS) layer
# that sits between user programs and the raw disk, providing the familiar
# abstraction of files and directories.
#
# == What Is a File System?
#
# A file system turns a raw disk — billions of identical bytes with no
# structure — into the familiar world of files and directories. Without a
# file system, every program would need to remember "my data starts at byte
# 4,194,304 and is 8,192 bytes long." With a file system, you just say
# open("/home/alice/notes.txt") and the OS figures out the rest.
#
# == Analogy
#
# Think of a library:
#   - The *disk* is the building full of shelves
#   - The *file system* is the cataloging system
#   - The *inode table* is the card catalog
#   - The *block pointers* are the Dewey Decimal numbers
#   - The *directories* are the shelf labels
#   - The *file descriptors* are the checkout desk
#
# Without the cataloging system, you would have a warehouse of unlabeled
# books with no way to find anything.
#
# == Components
#
#   - Constants: disk geometry, file types, flags
#   - Superblock: file system metadata (magic number, free counts)
#   - Inode: file metadata (type, size, permissions, block pointers)
#   - DirectoryEntry: name-to-inode mapping
#   - BlockBitmap: tracks which blocks are free/used
#   - InodeTable: manages the array of all inodes
#   - OpenFile/OpenFileTable: system-wide open file entries
#   - FileDescriptorTable: per-process fd-to-OpenFile mapping
#   - VFS: the main interface tying everything together
#
# == Quick Start
#
#   require "coding_adventures_file_system"
#
#   vfs = CodingAdventures::FileSystem::VFS.new
#   vfs.format
#
#   # Create a directory and a file
#   vfs.mkdir("/data")
#   fd = vfs.open("/data/hello.txt", CodingAdventures::FileSystem::O_WRONLY | CodingAdventures::FileSystem::O_CREAT)
#   vfs.write(fd, "Hello, world!")
#   vfs.close(fd)
#
#   # Read it back
#   fd = vfs.open("/data/hello.txt", CodingAdventures::FileSystem::O_RDONLY)
#   puts vfs.read(fd, 100)  # => "Hello, world!"
#   vfs.close(fd)

module CodingAdventures
  module FileSystem
  end
end

require_relative "coding_adventures/file_system/version"
require_relative "coding_adventures/file_system/constants"
require_relative "coding_adventures/file_system/superblock"
require_relative "coding_adventures/file_system/inode"
require_relative "coding_adventures/file_system/directory"
require_relative "coding_adventures/file_system/block_bitmap"
require_relative "coding_adventures/file_system/inode_table"
require_relative "coding_adventures/file_system/file_descriptor"
require_relative "coding_adventures/file_system/vfs"
