export type FsError = "EINVAL" | "ENOENT" | "ENOTDIR" | "EISDIR";
type Directory = {
  readonly kind: "directory";
  readonly inode: number;
  readonly entries: readonly { readonly name: string; readonly inode: number }[];
};
type File = { readonly kind: "file"; readonly inode: number; readonly text: string };
export type Inode = Directory | File;
export type Image = ReadonlyMap<number, Inode>;

export const demoImage: Image = new Map<number, Inode>([
  [1, { inode: 1, kind: "directory", entries: [
    { name: "hello", inode: 2 }, { name: "empty", inode: 3 }, { name: "etc", inode: 4 },
  ] }],
  [2, { inode: 2, kind: "file", text: "Hello, Unix!\n" }],
  [3, { inode: 3, kind: "file", text: "" }],
  [4, { inode: 4, kind: "directory", entries: [{ name: "notes", inode: 5 }] }],
  [5, { inode: 5, kind: "file", text: "A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n" }],
]);

// Resolve directory entries as we go. Collapsing ".." before lookup would
// incorrectly accept a path such as "/hello/..", where hello is a file.
export function resolvePath(image: Image, path: string): Inode | FsError {
  if (!path || path.includes("\0")) return "EINVAL";
  const root = image.get(1);
  if (!root) return "ENOENT";
  const parents: Directory[] = [];
  let node = root;
  for (const name of path.split("/").filter(Boolean)) {
    if (node.kind !== "directory") return "ENOTDIR";
    if (name === ".") continue;
    if (name === "..") {
      node = parents.pop() ?? root;
      continue;
    }
    const entry = node.entries.find(candidate => candidate.name === name);
    const child = entry && image.get(entry.inode);
    if (!child) return "ENOENT";
    parents.push(node);
    node = child;
  }
  return path.endsWith("/") && node.kind !== "directory" ? "ENOTDIR" : node;
}

const encoder = new TextEncoder();

export function readFile(image: Image, inode: number, offset: number, count: number): Uint8Array | FsError {
  const node = image.get(inode);
  if (!node) return "ENOENT";
  if (node.kind !== "file") return "EISDIR";
  if (!Number.isSafeInteger(offset) || offset < 0 || !Number.isSafeInteger(count) || count < 0) return "EINVAL";
  return encoder.encode(node.text).slice(offset, offset + count);
}
