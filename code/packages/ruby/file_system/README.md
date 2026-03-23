# File System (Ruby)

An inode-based file system inspired by ext2, implementing the Virtual File System (VFS) layer with directories, file descriptors, and block I/O.

## Where It Fits

```
User Program
    open("/data/log.txt", O_RDWR)
    write(fd, "hello", 5)
    close(fd)
        |
        v
VFS (this package)
    Path Resolution -> Inode Table -> Block Bitmap -> Data Blocks
        |
        v
Block Device (simulated in-memory disk)
```

## Components

| Component | Description |
|-----------|-------------|
| `Superblock` | File system metadata: magic number, block/inode counts |
| `Inode` | File metadata: type, size, permissions, block pointers |
| `DirectoryEntry` | Name-to-inode mapping stored in directory data blocks |
| `BlockBitmap` | Tracks which data blocks are free/used |
| `InodeTable` | Manages the fixed array of 128 inodes |
| `OpenFile` | System-wide entry for one opening of a file |
| `OpenFileTable` | System-wide table of all open files |
| `FileDescriptorTable` | Per-process fd-to-OpenFile mapping |
| `VFS` | Main interface: format, open, read, write, mkdir, etc. |

## Usage

```ruby
require "coding_adventures_file_system"

include CodingAdventures::FileSystem

vfs = VFS.new
vfs.format

# Create a directory and write a file
vfs.mkdir("/data")
fd = vfs.open("/data/hello.txt", O_WRONLY | O_CREAT)
vfs.write(fd, "Hello, world!")
vfs.close(fd)

# Read it back
fd = vfs.open("/data/hello.txt", O_RDONLY)
puts vfs.read(fd, 100)  # => "Hello, world!"
vfs.close(fd)

# List directory contents
entries = vfs.readdir("/data")
entries.each { |e| puts "#{e.name} -> inode #{e.inode_number}" }
```

## Running Tests

```bash
bundle install
bundle exec rake test
```
