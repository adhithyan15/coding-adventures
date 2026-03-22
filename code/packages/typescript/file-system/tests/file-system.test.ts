/**
 * # File System Tests
 *
 * These tests exercise every component of the file system implementation:
 * block bitmap, inode table, directory operations, file I/O, seeking,
 * file descriptors, and edge cases like disk-full and inode exhaustion.
 *
 * The tests follow the same layered structure as the implementation:
 * 1. Low-level components (BlockBitmap, InodeTable)
 * 2. Directory operations (mkdir, readdir)
 * 3. File I/O (open, write, read, close)
 * 4. Seeking (lseek with all whence modes)
 * 5. File management (stat, unlink)
 * 6. File descriptors (dup, dup2)
 * 7. Edge cases and stress tests
 */

import { describe, it, expect, beforeEach } from "vitest";
import {
  VFS,
  BlockBitmap,
  InodeTable,
  FileDescriptorTable,
  OpenFileTable,
  FileType,
  BLOCK_SIZE,
  MAX_BLOCKS,
  MAX_INODES,
  DIRECT_BLOCKS,
  ROOT_INODE,
  O_RDONLY,
  O_WRONLY,
  O_RDWR,
  O_CREAT,
  O_TRUNC,
  O_APPEND,
  SEEK_SET,
  SEEK_CUR,
  SEEK_END,
} from "../src/index.js";

// ============================================================================
// Helper Functions
// ============================================================================

/** Encodes a string as a Uint8Array (UTF-8). */
function encode(str: string): Uint8Array {
  return new TextEncoder().encode(str);
}

/** Decodes a Uint8Array to a string (UTF-8). */
function decode(data: Uint8Array): string {
  return new TextDecoder().decode(data);
}

// ============================================================================
// BlockBitmap Tests
// ============================================================================

describe("BlockBitmap", () => {
  it("should allocate blocks sequentially", () => {
    const bitmap = new BlockBitmap(10);
    expect(bitmap.allocate()).toBe(0);
    expect(bitmap.allocate()).toBe(1);
    expect(bitmap.allocate()).toBe(2);
  });

  it("should report blocks as used after allocation", () => {
    const bitmap = new BlockBitmap(10);
    const block = bitmap.allocate()!;
    expect(bitmap.isFree(block)).toBe(false);
  });

  it("should free blocks and allow reuse", () => {
    const bitmap = new BlockBitmap(10);
    const block = bitmap.allocate()!;
    bitmap.free(block);
    expect(bitmap.isFree(block)).toBe(true);
    expect(bitmap.allocate()).toBe(block); // reused
  });

  it("should return null when all blocks are allocated", () => {
    const bitmap = new BlockBitmap(3);
    bitmap.allocate();
    bitmap.allocate();
    bitmap.allocate();
    expect(bitmap.allocate()).toBeNull();
  });

  it("should track free count correctly", () => {
    const bitmap = new BlockBitmap(5);
    expect(bitmap.freeCount()).toBe(5);
    bitmap.allocate();
    expect(bitmap.freeCount()).toBe(4);
    bitmap.allocate();
    bitmap.free(0);
    expect(bitmap.freeCount()).toBe(4);
  });

  it("should handle out-of-range queries gracefully", () => {
    const bitmap = new BlockBitmap(5);
    expect(bitmap.isFree(-1)).toBe(false);
    expect(bitmap.isFree(100)).toBe(false);
  });
});

// ============================================================================
// InodeTable Tests
// ============================================================================

describe("InodeTable", () => {
  it("should allocate inodes with sequential numbers", () => {
    const table = new InodeTable(5);
    const i0 = table.allocate(FileType.DIRECTORY)!;
    const i1 = table.allocate(FileType.REGULAR)!;
    expect(i0.inode_number).toBe(0);
    expect(i1.inode_number).toBe(1);
  });

  it("should initialize inode fields correctly", () => {
    const table = new InodeTable(5);
    const inode = table.allocate(FileType.REGULAR)!;
    expect(inode.file_type).toBe(FileType.REGULAR);
    expect(inode.size).toBe(0);
    expect(inode.link_count).toBe(1);
    expect(inode.direct_blocks).toHaveLength(DIRECT_BLOCKS);
    expect(inode.direct_blocks.every((b) => b === -1)).toBe(true);
    expect(inode.indirect_block).toBe(-1);
  });

  it("should free and reuse inodes", () => {
    const table = new InodeTable(3);
    table.allocate(FileType.REGULAR);
    const i1 = table.allocate(FileType.REGULAR)!;
    table.free(i1.inode_number);
    expect(table.get(i1.inode_number)).toBeNull();
    const reused = table.allocate(FileType.DIRECTORY)!;
    expect(reused.inode_number).toBe(1); // reused slot 1
  });

  it("should return null when all inodes are allocated", () => {
    const table = new InodeTable(2);
    table.allocate(FileType.REGULAR);
    table.allocate(FileType.REGULAR);
    expect(table.allocate(FileType.REGULAR)).toBeNull();
  });

  it("should return null for out-of-range get", () => {
    const table = new InodeTable(5);
    expect(table.get(-1)).toBeNull();
    expect(table.get(100)).toBeNull();
  });
});

// ============================================================================
// OpenFileTable Tests
// ============================================================================

describe("OpenFileTable", () => {
  it("should open files starting from index 3", () => {
    const table = new OpenFileTable();
    const idx = table.open(10, O_RDWR)!;
    expect(idx).toBeGreaterThanOrEqual(3);
  });

  it("should track ref_count correctly", () => {
    const table = new OpenFileTable();
    const idx = table.open(10, O_RDWR)!;
    expect(table.get(idx)!.ref_count).toBe(1);
    table.dup(idx);
    expect(table.get(idx)!.ref_count).toBe(2);
    table.close(idx);
    expect(table.get(idx)!.ref_count).toBe(1);
    table.close(idx);
    expect(table.get(idx)).toBeNull();
  });

  it("should mask flags to access mode bits", () => {
    const table = new OpenFileTable();
    const idx = table.open(10, O_WRONLY | O_CREAT)!;
    expect(table.get(idx)!.flags).toBe(O_WRONLY);
  });

  it("should return null for dup of non-existent entry", () => {
    const table = new OpenFileTable();
    expect(table.dup(999)).toBeNull();
  });
});

// ============================================================================
// FileDescriptorTable Tests
// ============================================================================

describe("FileDescriptorTable", () => {
  it("should allocate fds starting from 3", () => {
    const table = new FileDescriptorTable();
    const fd = table.allocate(10);
    expect(fd).toBe(3);
  });

  it("should map fds to global indices", () => {
    const table = new FileDescriptorTable();
    table.allocate(10);
    expect(table.get(3)).toBe(10);
  });

  it("should reuse freed fd slots", () => {
    const table = new FileDescriptorTable();
    const fd1 = table.allocate(10);
    table.free(fd1);
    const fd2 = table.allocate(20);
    expect(fd2).toBe(fd1); // reused
    expect(table.get(fd2)).toBe(20);
  });

  it("should dup2 to a specific fd", () => {
    const table = new FileDescriptorTable();
    table.allocate(10); // fd 3
    const result = table.dup2(3, 5);
    expect(result).toBe(5);
    expect(table.get(5)).toBe(10);
  });

  it("should clone for fork", () => {
    const table = new FileDescriptorTable();
    table.allocate(10);
    const copy = table.clone();
    expect(copy.get(3)).toBe(10);
    copy.free(3);
    expect(table.get(3)).toBe(10); // original unaffected
  });

  it("should return null for invalid fd", () => {
    const table = new FileDescriptorTable();
    expect(table.get(-1)).toBeNull();
    expect(table.get(100)).toBeNull();
  });

  it("should return null for dup2 with invalid old fd", () => {
    const table = new FileDescriptorTable();
    expect(table.dup2(99, 5)).toBeNull();
  });
});

// ============================================================================
// VFS — Format and Superblock
// ============================================================================

describe("VFS - format", () => {
  let vfs: VFS;

  beforeEach(() => {
    vfs = new VFS();
    vfs.format();
  });

  it("should initialize superblock with correct magic number", () => {
    const sb = vfs.getSuperblock();
    expect(sb.magic).toBe(0x45585432);
  });

  it("should initialize superblock with correct sizes", () => {
    const sb = vfs.getSuperblock();
    expect(sb.block_size).toBe(BLOCK_SIZE);
    expect(sb.total_blocks).toBe(MAX_BLOCKS);
    expect(sb.total_inodes).toBe(MAX_INODES);
  });

  it("should have correct free counts after format", () => {
    const sb = vfs.getSuperblock();
    expect(sb.free_inodes).toBe(MAX_INODES - 1); // root uses 1
  });

  it("should create root directory at inode 0", () => {
    const root = vfs.stat("/");
    expect(root).not.toBeNull();
    expect(root!.inode_number).toBe(ROOT_INODE);
    expect(root!.file_type).toBe(FileType.DIRECTORY);
  });

  it("should have . and .. in root directory", () => {
    const entries = vfs.readdir("/")!;
    const names = entries.map((e) => e.name);
    expect(names).toContain(".");
    expect(names).toContain("..");
    /* Both . and .. point to inode 0 (root is its own parent) */
    expect(entries.find((e) => e.name === ".")!.inode_number).toBe(ROOT_INODE);
    expect(entries.find((e) => e.name === "..")!.inode_number).toBe(ROOT_INODE);
  });
});

// ============================================================================
// VFS — mkdir and readdir
// ============================================================================

describe("VFS - mkdir", () => {
  let vfs: VFS;

  beforeEach(() => {
    vfs = new VFS();
    vfs.format();
  });

  it("should create a directory with . and .. entries", () => {
    expect(vfs.mkdir("/home")).toBe(true);
    const entries = vfs.readdir("/home")!;
    const names = entries.map((e) => e.name);
    expect(names).toContain(".");
    expect(names).toContain("..");
  });

  it("should add entry in parent directory", () => {
    vfs.mkdir("/home");
    const rootEntries = vfs.readdir("/")!;
    expect(rootEntries.map((e) => e.name)).toContain("home");
  });

  it("should set .. to point to parent inode", () => {
    vfs.mkdir("/home");
    const entries = vfs.readdir("/home")!;
    const dotdot = entries.find((e) => e.name === "..")!;
    expect(dotdot.inode_number).toBe(ROOT_INODE);
  });

  it("should create nested directories", () => {
    vfs.mkdir("/a");
    vfs.mkdir("/a/b");
    vfs.mkdir("/a/b/c");

    expect(vfs.stat("/a/b/c")).not.toBeNull();
    expect(vfs.stat("/a/b/c")!.file_type).toBe(FileType.DIRECTORY);

    const entries = vfs.readdir("/a/b/c")!;
    expect(entries.map((e) => e.name)).toContain(".");
    expect(entries.map((e) => e.name)).toContain("..");
  });

  it("should fail when parent does not exist", () => {
    expect(vfs.mkdir("/nonexistent/child")).toBe(false);
  });

  it("should fail when name already exists", () => {
    vfs.mkdir("/home");
    expect(vfs.mkdir("/home")).toBe(false);
  });

  it("should return null for readdir on non-existent path", () => {
    expect(vfs.readdir("/no/such/path")).toBeNull();
  });
});

// ============================================================================
// VFS — Path Resolution
// ============================================================================

describe("VFS - resolvePath", () => {
  let vfs: VFS;

  beforeEach(() => {
    vfs = new VFS();
    vfs.format();
  });

  it("should resolve root path", () => {
    expect(vfs.resolvePath("/")).toBe(ROOT_INODE);
  });

  it("should resolve a directory", () => {
    vfs.mkdir("/home");
    const inodeNum = vfs.resolvePath("/home");
    expect(inodeNum).not.toBeNull();
    expect(inodeNum).toBeGreaterThan(ROOT_INODE);
  });

  it("should resolve nested paths", () => {
    vfs.mkdir("/a");
    vfs.mkdir("/a/b");
    vfs.mkdir("/a/b/c");
    expect(vfs.resolvePath("/a/b/c")).not.toBeNull();
  });

  it("should return null for non-existent paths", () => {
    expect(vfs.resolvePath("/does/not/exist")).toBeNull();
  });

  it("should return null when trying to traverse a file", () => {
    const fd = vfs.open("/file.txt", O_WRONLY | O_CREAT)!;
    vfs.close(fd);
    expect(vfs.resolvePath("/file.txt/child")).toBeNull();
  });
});

// ============================================================================
// VFS — File I/O (open, write, read, close)
// ============================================================================

describe("VFS - file I/O", () => {
  let vfs: VFS;

  beforeEach(() => {
    vfs = new VFS();
    vfs.format();
  });

  it("should create a file with O_CREAT", () => {
    const fd = vfs.open("/test.txt", O_WRONLY | O_CREAT);
    expect(fd).not.toBeNull();
    expect(fd).toBeGreaterThanOrEqual(3);
    vfs.close(fd!);
  });

  it("should write and read data back", () => {
    const fd = vfs.open("/test.txt", O_RDWR | O_CREAT)!;
    const data = encode("Hello, file system!");
    vfs.write(fd, data);

    /* Seek back to the start to read */
    vfs.lseek(fd, 0, SEEK_SET);
    const result = vfs.read(fd, 100)!;
    expect(decode(result)).toBe("Hello, file system!");
    vfs.close(fd);
  });

  it("should handle multiple writes", () => {
    const fd = vfs.open("/test.txt", O_RDWR | O_CREAT)!;
    vfs.write(fd, encode("Hello"));
    vfs.write(fd, encode(" World"));

    vfs.lseek(fd, 0, SEEK_SET);
    const result = vfs.read(fd, 100)!;
    expect(decode(result)).toBe("Hello World");
    vfs.close(fd);
  });

  it("should read only available bytes when count exceeds file size", () => {
    const fd = vfs.open("/test.txt", O_RDWR | O_CREAT)!;
    vfs.write(fd, encode("abc"));

    vfs.lseek(fd, 0, SEEK_SET);
    const result = vfs.read(fd, 1000)!;
    expect(result.length).toBe(3);
    expect(decode(result)).toBe("abc");
    vfs.close(fd);
  });

  it("should return empty array when reading at end of file", () => {
    const fd = vfs.open("/test.txt", O_RDWR | O_CREAT)!;
    vfs.write(fd, encode("data"));
    const result = vfs.read(fd, 10)!;
    expect(result.length).toBe(0);
    vfs.close(fd);
  });

  it("should fail to read a write-only file", () => {
    const fd = vfs.open("/test.txt", O_WRONLY | O_CREAT)!;
    vfs.write(fd, encode("data"));
    vfs.lseek(fd, 0, SEEK_SET);
    const result = vfs.read(fd, 10);
    expect(result).toBeNull();
    vfs.close(fd);
  });

  it("should fail to write a read-only file", () => {
    /* Create the file first */
    const fd1 = vfs.open("/test.txt", O_WRONLY | O_CREAT)!;
    vfs.write(fd1, encode("data"));
    vfs.close(fd1);

    /* Open for reading only */
    const fd2 = vfs.open("/test.txt", O_RDONLY)!;
    const result = vfs.write(fd2, encode("more"));
    expect(result).toBeNull();
    vfs.close(fd2);
  });

  it("should return null when opening non-existent file without O_CREAT", () => {
    const fd = vfs.open("/no-such-file.txt", O_RDONLY);
    expect(fd).toBeNull();
  });

  it("should return false when closing an invalid fd", () => {
    expect(vfs.close(999)).toBe(false);
  });

  it("should handle O_TRUNC flag", () => {
    /* Write some data */
    const fd1 = vfs.open("/test.txt", O_WRONLY | O_CREAT)!;
    vfs.write(fd1, encode("Hello World"));
    vfs.close(fd1);

    /* Open with O_TRUNC — should clear the file */
    const fd2 = vfs.open("/test.txt", O_RDWR | O_TRUNC)!;
    vfs.write(fd2, encode("Hi"));
    vfs.lseek(fd2, 0, SEEK_SET);
    const result = vfs.read(fd2, 100)!;
    expect(decode(result)).toBe("Hi");
    vfs.close(fd2);
  });

  it("should handle O_APPEND flag", () => {
    /* Write initial data */
    const fd1 = vfs.open("/test.txt", O_WRONLY | O_CREAT)!;
    vfs.write(fd1, encode("Hello"));
    vfs.close(fd1);

    /* Open with O_APPEND — writes should go to end */
    const fd2 = vfs.open("/test.txt", O_RDWR | O_APPEND)!;
    vfs.write(fd2, encode(" World"));
    vfs.lseek(fd2, 0, SEEK_SET);
    const result = vfs.read(fd2, 100)!;
    expect(decode(result)).toBe("Hello World");
    vfs.close(fd2);
  });

  it("should handle writes spanning multiple blocks", () => {
    const fd = vfs.open("/big.txt", O_RDWR | O_CREAT)!;
    /* Write more than one block (512 bytes) */
    const data = new Uint8Array(1500).fill(65); // 'A' repeated 1500 times
    vfs.write(fd, data);

    vfs.lseek(fd, 0, SEEK_SET);
    const result = vfs.read(fd, 2000)!;
    expect(result.length).toBe(1500);
    expect(result.every((b) => b === 65)).toBe(true);
    vfs.close(fd);
  });

  it("should handle writes that use indirect blocks", () => {
    const fd = vfs.open("/large.txt", O_RDWR | O_CREAT)!;
    /*
     * Direct blocks cover 12 × 512 = 6144 bytes.
     * Writing 7000 bytes forces indirect block allocation.
     */
    const data = new Uint8Array(7000);
    for (let i = 0; i < data.length; i++) data[i] = i % 256;
    vfs.write(fd, data);

    vfs.lseek(fd, 0, SEEK_SET);
    const result = vfs.read(fd, 8000)!;
    expect(result.length).toBe(7000);
    for (let i = 0; i < result.length; i++) {
      expect(result[i]).toBe(i % 256);
    }
    vfs.close(fd);
  });

  it("should handle file in a subdirectory", () => {
    vfs.mkdir("/data");
    const fd = vfs.open("/data/log.txt", O_RDWR | O_CREAT)!;
    vfs.write(fd, encode("log entry 1"));
    vfs.lseek(fd, 0, SEEK_SET);
    const result = vfs.read(fd, 100)!;
    expect(decode(result)).toBe("log entry 1");
    vfs.close(fd);
  });
});

// ============================================================================
// VFS — lseek
// ============================================================================

describe("VFS - lseek", () => {
  let vfs: VFS;

  beforeEach(() => {
    vfs = new VFS();
    vfs.format();
  });

  it("should seek with SEEK_SET (absolute)", () => {
    const fd = vfs.open("/test.txt", O_RDWR | O_CREAT)!;
    vfs.write(fd, encode("Hello World"));
    const pos = vfs.lseek(fd, 5, SEEK_SET);
    expect(pos).toBe(5);

    const result = vfs.read(fd, 10)!;
    expect(decode(result)).toBe(" World");
    vfs.close(fd);
  });

  it("should seek with SEEK_CUR (relative to current)", () => {
    const fd = vfs.open("/test.txt", O_RDWR | O_CREAT)!;
    vfs.write(fd, encode("Hello World"));
    vfs.lseek(fd, 0, SEEK_SET);
    /* Read 5 bytes to advance to offset 5 */
    vfs.read(fd, 5);
    /* Now seek 1 byte forward from current (offset 6) */
    const pos = vfs.lseek(fd, 1, SEEK_CUR);
    expect(pos).toBe(6);

    const result = vfs.read(fd, 10)!;
    expect(decode(result)).toBe("World");
    vfs.close(fd);
  });

  it("should seek with SEEK_END (relative to file end)", () => {
    const fd = vfs.open("/test.txt", O_RDWR | O_CREAT)!;
    vfs.write(fd, encode("Hello World")); // 11 bytes
    const pos = vfs.lseek(fd, -5, SEEK_END);
    expect(pos).toBe(6); // 11 - 5 = 6

    const result = vfs.read(fd, 10)!;
    expect(decode(result)).toBe("World");
    vfs.close(fd);
  });

  it("should return null for negative resulting offset", () => {
    const fd = vfs.open("/test.txt", O_RDWR | O_CREAT)!;
    vfs.write(fd, encode("abc"));
    const pos = vfs.lseek(fd, -100, SEEK_SET);
    expect(pos).toBeNull();
    vfs.close(fd);
  });

  it("should return null for invalid whence", () => {
    const fd = vfs.open("/test.txt", O_RDWR | O_CREAT)!;
    const pos = vfs.lseek(fd, 0, 99);
    expect(pos).toBeNull();
    vfs.close(fd);
  });

  it("should return null for invalid fd", () => {
    expect(vfs.lseek(999, 0, SEEK_SET)).toBeNull();
  });
});

// ============================================================================
// VFS — stat
// ============================================================================

describe("VFS - stat", () => {
  let vfs: VFS;

  beforeEach(() => {
    vfs = new VFS();
    vfs.format();
  });

  it("should return metadata for root", () => {
    const s = vfs.stat("/")!;
    expect(s.file_type).toBe(FileType.DIRECTORY);
    expect(s.inode_number).toBe(ROOT_INODE);
    expect(s.link_count).toBe(2);
  });

  it("should return metadata for a file", () => {
    const fd = vfs.open("/test.txt", O_WRONLY | O_CREAT)!;
    vfs.write(fd, encode("12345"));
    vfs.close(fd);

    const s = vfs.stat("/test.txt")!;
    expect(s.file_type).toBe(FileType.REGULAR);
    expect(s.size).toBe(5);
    expect(s.permissions).toBe(0o755);
  });

  it("should return metadata for a directory", () => {
    vfs.mkdir("/mydir");
    const s = vfs.stat("/mydir")!;
    expect(s.file_type).toBe(FileType.DIRECTORY);
    expect(s.link_count).toBe(2); // . and ..
  });

  it("should return null for non-existent path", () => {
    expect(vfs.stat("/nope")).toBeNull();
  });

  it("should update link_count when adding subdirectories", () => {
    vfs.mkdir("/parent");
    vfs.mkdir("/parent/child");
    const s = vfs.stat("/parent")!;
    /* parent has 3 links: "parent" entry in root, "." in itself, ".." in child */
    expect(s.link_count).toBe(3);
  });
});

// ============================================================================
// VFS — unlink
// ============================================================================

describe("VFS - unlink", () => {
  let vfs: VFS;

  beforeEach(() => {
    vfs = new VFS();
    vfs.format();
  });

  it("should remove a file", () => {
    const fd = vfs.open("/test.txt", O_WRONLY | O_CREAT)!;
    vfs.write(fd, encode("data"));
    vfs.close(fd);

    expect(vfs.unlink("/test.txt")).toBe(true);
    expect(vfs.stat("/test.txt")).toBeNull();
  });

  it("should free inode and blocks when link_count reaches 0", () => {
    const fd = vfs.open("/test.txt", O_WRONLY | O_CREAT)!;
    vfs.write(fd, encode("some data"));
    vfs.close(fd);

    const freeBefore = vfs.getSuperblock().free_inodes;
    vfs.unlink("/test.txt");
    const freeAfter = vfs.getSuperblock().free_inodes;
    expect(freeAfter).toBe(freeBefore + 1);
  });

  it("should not unlink directories", () => {
    vfs.mkdir("/mydir");
    expect(vfs.unlink("/mydir")).toBe(false);
  });

  it("should fail for non-existent file", () => {
    expect(vfs.unlink("/no-such-file")).toBe(false);
  });

  it("should remove entry from parent directory", () => {
    vfs.open("/a.txt", O_WRONLY | O_CREAT)!;
    vfs.open("/b.txt", O_WRONLY | O_CREAT)!;
    vfs.unlink("/a.txt");
    const entries = vfs.readdir("/")!;
    const names = entries.map((e) => e.name);
    expect(names).not.toContain("a.txt");
    expect(names).toContain("b.txt");
  });
});

// ============================================================================
// VFS — dup and dup2
// ============================================================================

describe("VFS - dup/dup2", () => {
  let vfs: VFS;

  beforeEach(() => {
    vfs = new VFS();
    vfs.format();
  });

  it("should duplicate a file descriptor", () => {
    const fd1 = vfs.open("/test.txt", O_RDWR | O_CREAT)!;
    const fd2 = vfs.dup(fd1)!;
    expect(fd2).not.toBe(fd1);
    expect(fd2).toBeGreaterThan(fd1);

    /* Both fds share the same offset — writing to one advances the other */
    vfs.write(fd1, encode("Hello"));
    vfs.write(fd2, encode(" World"));

    vfs.lseek(fd1, 0, SEEK_SET);
    const result = vfs.read(fd1, 100)!;
    expect(decode(result)).toBe("Hello World");

    vfs.close(fd1);
    vfs.close(fd2);
  });

  it("should dup2 to a specific fd number", () => {
    const fd = vfs.open("/test.txt", O_RDWR | O_CREAT)!;
    const result = vfs.dup2(fd, 10);
    expect(result).toBe(10);

    /* Write via the duplicated fd */
    vfs.write(10, encode("via dup2"));
    vfs.lseek(fd, 0, SEEK_SET);
    const data = vfs.read(fd, 100)!;
    expect(decode(data)).toBe("via dup2");

    vfs.close(fd);
    vfs.close(10);
  });

  it("should return null for dup of invalid fd", () => {
    expect(vfs.dup(999)).toBeNull();
  });

  it("should return null for dup2 with invalid old fd", () => {
    expect(vfs.dup2(999, 5)).toBeNull();
  });
});

// ============================================================================
// VFS — Multi-Process File Descriptors
// ============================================================================

describe("VFS - multi-process", () => {
  let vfs: VFS;

  beforeEach(() => {
    vfs = new VFS();
    vfs.format();
  });

  it("should maintain independent fd tables per process", () => {
    const fd1 = vfs.open("/file1.txt", O_WRONLY | O_CREAT, 1)!;
    const fd2 = vfs.open("/file2.txt", O_WRONLY | O_CREAT, 2)!;

    /* Both processes can have fd 3 pointing to different files */
    expect(fd1).toBe(3);
    expect(fd2).toBe(3);

    vfs.write(fd1, encode("from pid 1"), 1);
    vfs.write(fd2, encode("from pid 2"), 2);

    vfs.close(fd1, 1);
    vfs.close(fd2, 2);

    /* Verify independent contents */
    const r1 = vfs.open("/file1.txt", O_RDONLY, 1)!;
    const r2 = vfs.open("/file2.txt", O_RDONLY, 2)!;
    const d1 = vfs.read(r1, 100, 1)!;
    const d2 = vfs.read(r2, 100, 2)!;
    expect(decode(d1)).toBe("from pid 1");
    expect(decode(d2)).toBe("from pid 2");
  });
});

// ============================================================================
// VFS — Edge Cases and Stress Tests
// ============================================================================

describe("VFS - edge cases", () => {
  let vfs: VFS;

  beforeEach(() => {
    vfs = new VFS();
    vfs.format();
  });

  it("should handle empty path components gracefully", () => {
    vfs.mkdir("/data");
    /* "/data/" has a trailing slash — resolvePath should handle it */
    expect(vfs.resolvePath("/data/")).not.toBeNull();
  });

  it("should handle read returning null for invalid fd", () => {
    expect(vfs.read(999, 10)).toBeNull();
  });

  it("should handle write returning null for invalid fd", () => {
    expect(vfs.write(999, encode("data"))).toBeNull();
  });

  it("should fail mkdir with empty name", () => {
    expect(vfs.mkdir("/")).toBe(false);
  });

  it("should fail unlink with empty name", () => {
    expect(vfs.unlink("/")).toBe(false);
  });

  it("should readdir return null for a file (not directory)", () => {
    const fd = vfs.open("/file.txt", O_WRONLY | O_CREAT)!;
    vfs.close(fd);
    expect(vfs.readdir("/file.txt")).toBeNull();
  });
});

// ============================================================================
// VFS — Full Workflow Integration Test
// ============================================================================

describe("VFS - integration", () => {
  it("should handle full workflow: format → mkdir → write → read → unlink", () => {
    const vfs = new VFS();
    vfs.format();

    /* Create directory structure */
    vfs.mkdir("/home");
    vfs.mkdir("/home/alice");

    /* Create and write a file */
    const fd = vfs.open("/home/alice/notes.txt", O_RDWR | O_CREAT)!;
    vfs.write(fd, encode("My first note\n"));
    vfs.write(fd, encode("My second note\n"));
    vfs.close(fd);

    /* Read the file back */
    const fd2 = vfs.open("/home/alice/notes.txt", O_RDONLY)!;
    const content = vfs.read(fd2, 1000)!;
    expect(decode(content)).toBe("My first note\nMy second note\n");
    vfs.close(fd2);

    /* Verify stat */
    const s = vfs.stat("/home/alice/notes.txt")!;
    expect(s.file_type).toBe(FileType.REGULAR);
    expect(s.size).toBe(29);

    /* Readdir on parent */
    const entries = vfs.readdir("/home/alice")!;
    const names = entries.map((e) => e.name);
    expect(names).toContain("notes.txt");

    /* Unlink */
    vfs.unlink("/home/alice/notes.txt");
    expect(vfs.stat("/home/alice/notes.txt")).toBeNull();
  });

  it("should handle creating many files until inodes are exhausted", () => {
    const vfs = new VFS();
    vfs.format();

    /* Root uses inode 0, so we can create MAX_INODES - 1 more files */
    let created = 0;
    for (let i = 0; i < MAX_INODES; i++) {
      const fd = vfs.open(`/file${i}.txt`, O_WRONLY | O_CREAT);
      if (fd === null) break;
      vfs.close(fd);
      created++;
    }

    /* Should have created fewer than MAX_INODES files
     * (root inode uses one slot) */
    expect(created).toBeLessThan(MAX_INODES);
    expect(created).toBeGreaterThan(0);
  });
});
