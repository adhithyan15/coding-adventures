import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const modulePath = resolve(process.cwd(), process.argv[2] ?? "");
const command = process.argv[3] ?? "";
const moduleBytes = await readFile(modulePath);
const { instance } = await WebAssembly.instantiate(moduleBytes);
const guest = instance.exports;
const memory = new Uint8Array(guest.memory.buffer);
const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });
const errorNames = new Map([
  [-1, "EINVAL"], [-2, "ENOENT"], [-3, "ENOTDIR"], [-4, "EISDIR"],
  [-5, "EBADF"], [-6, "EMFILE"],
]);

function errorName(code) {
  return errorNames.get(code) ?? "EINVAL";
}

function splitAsciiWords(text) {
  return text.match(/[^ \t\n\r\f\v]+/g) ?? [];
}

function passPath(path) {
  const bytes = encoder.encode(path);
  if (bytes.length === 0 || bytes.length > 4096) return -1;
  memory.set(bytes, 4096);
  return guest.resolve(4096, bytes.length);
}

function listing(inode) {
  const count = guest.entry_count(inode);
  if (count < 0 || count > 512) return { error: errorName(count < 0 ? count : -1) };
  const names = [];
  for (let index = 0; index < count; index += 1) {
    const pointer = guest.entry_name_ptr(inode, index);
    const length = guest.entry_name_len(inode, index);
    if (pointer < 0 || length < 0 || length > 255 || pointer > memory.length || length > memory.length - pointer) {
      return { error: "EINVAL" };
    }
    names.push(decoder.decode(memory.slice(pointer, pointer + length)));
  }
  return { names };
}

function cat(path) {
  const inode = passPath(path);
  if (inode < 0) return { output: `error: ${errorName(inode)}\n`, events: [{ call: "open", path, error: errorName(inode) }] };
  const fd = guest.open(inode);
  if (fd < 0) return { output: `error: ${errorName(fd)}\n`, events: [{ call: "open", path, error: errorName(fd) }] };
  const events = [{ call: "open", path, inode, fd, offset: guest.descriptor_offset(fd) }];
  const chunks = [];
  while (true) {
    const currentInode = guest.descriptor_inode(fd);
    const offset = guest.descriptor_offset(fd);
    const count = guest.read(fd, 8192, 8);
    if (count < 0) {
      events.push({ call: "read", fd, error: errorName(count) });
      break;
    }
    events.push({ call: "read", fd, inode: currentInode, offset, bytes: count, nextOffset: guest.descriptor_offset(fd) });
    if (count === 0) break;
    chunks.push(Buffer.from(memory.slice(8192, 8192 + count)));
  }
  const closeResult = guest.close(fd);
  events.push(closeResult < 0 ? { call: "close", fd, error: errorName(closeResult) } : { call: "close", fd });
  return { output: Buffer.concat(chunks).toString("utf8"), events };
}

function ls(path) {
  const inode = passPath(path);
  if (inode < 0) return { output: `error: ${errorName(inode)}\n`, events: [{ call: "list", path, error: errorName(inode) }] };
  const kind = guest.kind(inode);
  if (kind < 0) return { output: `error: ${errorName(kind)}\n`, events: [{ call: "list", path, error: errorName(kind) }] };
  if (kind !== 1) return { output: "error: ENOTDIR\n", events: [{ call: "list", path, error: "ENOTDIR" }] };
  const result = listing(inode);
  if (result.error) return { output: `error: ${result.error}\n`, events: [{ call: "list", path, error: result.error }] };
  return {
    output: result.names.length === 0 ? "" : `${result.names.join("\n")}\n`,
    events: [{ call: "list", path, inode, names: result.names }],
  };
}

function run(commandText) {
  guest.reset();
  const words = splitAsciiWords(commandText);
  if (words.length === 0) return { output: "", events: [] };
  if (words[0] === "cat") {
    if (words.length !== 2) return { output: "usage: cat PATH or ls [PATH]\n", events: [] };
    return cat(words[1]);
  }
  if (words[0] === "ls") {
    if (words.length > 2) return { output: "usage: cat PATH or ls [PATH]\n", events: [] };
    return ls(words[1] ?? "/");
  }
  return { output: "usage: cat PATH or ls [PATH]\n", events: [] };
}

process.stdout.write(`${JSON.stringify(run(command))}\n`);
