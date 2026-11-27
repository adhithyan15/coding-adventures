import { advanceDescriptor, closeDescriptor, openDescriptor, type DescriptorTable } from "./descriptors.js";
import { demoImage, readFile, resolvePath, type Image } from "./filesystem.js";

export type OpenEvent =
  | { call: "open"; path: string; inode: number; fd: number; offset: number }
  | { call: "open"; path: string; error: string };
export type ReadEvent =
  | { call: "read"; fd: number; inode: number; offset: number; bytes: number; nextOffset: number }
  | { call: "read"; fd: number; error: string };
export type CloseEvent = { call: "close"; fd: number; error?: string };
export type ListEvent =
  | { call: "list"; path: string; inode: number; names: string[] }
  | { call: "list"; path: string; error: string };
export type SyscallEvent = OpenEvent | ReadEvent | CloseEvent | ListEvent;

// The kernel combines two independent mechanisms. The file system knows inode
// contents and the descriptor table knows what this process has open.
export class Kernel {
  private table: DescriptorTable;
  private readonly image: Image;

  constructor(image: Image = demoImage, slots = 4) {
    if (!Number.isInteger(slots) || slots < 1 || slots > 1024) {
      throw new RangeError("descriptor slots must be an integer from 1 through 1024");
    }
    this.image = image;
    this.table = Array<null>(slots).fill(null);
  }

  get descriptors(): DescriptorTable {
    return this.table;
  }

  open(path: string): OpenEvent {
    const node = resolvePath(this.image, path);
    if (typeof node === "string") return { call: "open", path, error: node };
    if (node.kind !== "file") return { call: "open", path, error: "EISDIR" };
    const result = openDescriptor(this.table, node.inode);
    if (!result) return { call: "open", path, error: "EMFILE" };
    this.table = result.table;
    return { call: "open", path, inode: node.inode, fd: result.fd, offset: 0 };
  }

  read(fd: number, count: number): { event: ReadEvent; data?: Uint8Array } {
    const slot = this.table[fd];
    if (!slot) return { event: { call: "read", fd, error: "EBADF" } };
    if (!Number.isSafeInteger(count) || count < 0) {
      return { event: { call: "read", fd, error: "EINVAL" } };
    }
    const { inode, offset } = slot;
    const data = readFile(this.image, inode, offset, count);
    if (typeof data === "string") return { event: { call: "read", fd, error: data } };
    // Data is a slice of an in-memory file, so its end cannot exceed the
    // safe-integer offset range or change the validity of the open slot.
    this.table = advanceDescriptor(this.table, fd, data.length)!;
    return {
      event: { call: "read", fd, inode, offset, bytes: data.length, nextOffset: offset + data.length },
      data,
    };
  }

  close(fd: number): CloseEvent {
    const table = closeDescriptor(this.table, fd);
    if (!table) return { call: "close", fd, error: "EBADF" };
    this.table = table;
    return { call: "close", fd };
  }

  list(path: string): ListEvent {
    const node = resolvePath(this.image, path);
    if (typeof node === "string") return { call: "list", path, error: node };
    if (node.kind !== "directory") return { call: "list", path, error: "ENOTDIR" };
    return { call: "list", path, inode: node.inode, names: node.entries.map(entry => entry.name) };
  }
}
