import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

const modulePath = resolve(process.cwd(), process.argv[2] ?? "");
const bytes = await readFile(modulePath);
const module = await WebAssembly.compile(bytes);
assert.deepEqual(WebAssembly.Module.imports(module), [], "guest must have no host imports");
const instance = await WebAssembly.instantiate(module);
const guest = instance.exports;
const memory = new Uint8Array(guest.memory.buffer);
const encoder = new TextEncoder();

function pathInGuest(path) {
  const encoded = encoder.encode(path);
  assert.ok(encoded.length <= 4096, "path exceeds guest contract");
  memory.set(encoded, 4096);
  return guest.resolve(4096, encoded.length);
}

assert.equal(guest.memory.buffer.byteLength, 65536);
assert.throws(() => guest.memory.grow(1), RangeError, "the guest memory maximum is one page");
assert.equal(guest.kind(1), 1);
assert.equal(guest.kind(2), 2);
assert.equal(guest.kind(99), -2);
guest.reset();
assert.equal(guest.descriptor_inode(0), -5, "reset leaves descriptors closed");
assert.equal(pathInGuest("/"), 1);
assert.equal(pathInGuest("/hello"), 2);
assert.equal(pathInGuest("/empty"), 3);
assert.equal(pathInGuest("/etc"), 4);
assert.equal(pathInGuest("/etc/notes"), 5);
assert.equal(pathInGuest("//etc/../"), 1);
assert.equal(pathInGuest("/missing"), -2);
assert.equal(pathInGuest("/hello/.."), -3, "must traverse before applying dot-dot");
assert.equal(pathInGuest("/hello/"), -3, "a regular file cannot satisfy a trailing slash");
assert.equal(pathInGuest(""), -1);
assert.equal(pathInGuest("/he\0llo"), -1);
assert.equal(guest.resolve(4095, 1), -1, "resolve cannot read before its input buffer");
assert.equal(guest.resolve(8192, 1), -1, "resolve cannot read after its input buffer");
memory[8191] = 47;
assert.equal(guest.resolve(8191, 1), 1, "input ending exactly at the upper bound is valid");

guest.reset();
assert.equal(guest.open(pathInGuest("/etc")), -4);
const hello = pathInGuest("/hello");
const fd0 = guest.open(hello);
const fd1 = guest.open(hello);
assert.equal(fd0, 0);
assert.equal(fd1, 1);
assert.equal(guest.descriptor_inode(fd0), 2);
assert.equal(guest.descriptor_offset(fd0), 0);
assert.equal(guest.descriptor_offset(fd1), 0, "opens of one inode have independent offsets");
assert.equal(guest.open(-1), -1, "an incoming guest error is preserved");
assert.equal(guest.open(99), -2, "unknown inode input is rejected");
assert.equal(guest.open(hello), 2);
assert.equal(guest.open(hello), 3);
assert.equal(guest.open(hello), -6, "descriptor table is bounded to four slots");

assert.equal(guest.read(fd0, 8192, -1), -1);
assert.equal(guest.descriptor_offset(fd0), 0, "rejected read does not advance offset");
const fileImage = memory.slice(256, 256 + 13);
assert.equal(guest.read(fd0, 256, 8), -1, "guest reads cannot write over the file image");
assert.deepEqual(memory.slice(256, 256 + 13), fileImage, "rejected guest write preserves the file image");
assert.equal(guest.descriptor_offset(fd0), 0, "image-overlap rejection is atomic");
assert.equal(guest.read(fd0, 4096, 8), -1, "read output cannot overlap the path input buffer");
assert.equal(guest.read(fd0, 65534, 8), -1, "destination range is checked before access");
assert.equal(guest.descriptor_offset(fd0), 0, "out-of-range read is atomic");
assert.equal(guest.read(fd0, 8191, 1), -1, "output starts at the documented lower bound");
assert.equal(guest.read(fd0, 12288, 1), -1, "output cannot cross its documented upper bound");
assert.equal(guest.read(fd0, 12288, 0), 0, "empty output at the exclusive upper endpoint is valid");
assert.equal(guest.descriptor_offset(fd0), 0);
assert.equal(guest.read(99, 8192, 8), -5);

const n0 = guest.read(fd0, 8192, 8);
assert.equal(n0, 8);
assert.equal(new TextDecoder().decode(memory.slice(8192, 8192 + n0)), "Hello, U");
assert.equal(guest.descriptor_offset(fd0), 8);
assert.equal(guest.descriptor_offset(fd1), 0);
const n1 = guest.read(fd0, 8192, 8);
assert.equal(n1, 5);
assert.equal(new TextDecoder().decode(memory.slice(8192, 8192 + n1)), "nix!\n");
assert.equal(guest.read(fd0, 8192, 8), 0);
assert.equal(guest.descriptor_offset(fd0), 13);
assert.equal(guest.close(fd0), 0);
assert.equal(guest.close(fd0), -5);
assert.equal(guest.open(hello), 0, "close returns the lowest slot for reuse");

guest.reset();
const empty = guest.open(pathInGuest("/empty"));
assert.equal(empty, 0);
assert.equal(guest.read(empty, 8192, 8), 0);
assert.equal(guest.descriptor_offset(empty), 0);
const notes = guest.open(pathInGuest("/etc/notes"));
const noteChunks = [];
while (true) {
  const count = guest.read(notes, 8192, 8);
  if (count <= 0) break;
  noteChunks.push(Buffer.from(memory.slice(8192, 8192 + count)));
}
assert.equal(Buffer.concat(noteChunks).toString("utf8"),
  "A directory maps a name to an inode.\nAn open descriptor tracks a byte offset.\n");

const rootCount = guest.entry_count(1);
assert.equal(rootCount, 3);
const rootNames = [];
for (let index = 0; index < rootCount; index += 1) {
  const pointer = guest.entry_name_ptr(1, index);
  const length = guest.entry_name_len(1, index);
  assert.ok(pointer >= 0 && length >= 0 && pointer + length <= memory.length);
  rootNames.push(new TextDecoder().decode(memory.slice(pointer, pointer + length)));
}
assert.deepEqual(rootNames, ["hello", "empty", "etc"]);
assert.equal(guest.entry_count(4), 1);
assert.equal(guest.entry_name_ptr(4, 0), 408);
assert.equal(guest.entry_name_len(4, 0), 5);
assert.equal(guest.entry_name_ptr(4, 1), -1);
assert.equal(guest.entry_count(2), 0);

guest.reset();
const endpointFd = guest.open(pathInGuest("/hello"));
assert.equal(guest.read(endpointFd, 8192, 4096), 13, "full output region endpoint is accepted");
assert.equal(guest.descriptor_offset(endpointFd), 13);
assert.equal(new TextDecoder().decode(memory.slice(8192, 8205)), "Hello, Unix!\n");

console.log("WAT guest component checks passed");
