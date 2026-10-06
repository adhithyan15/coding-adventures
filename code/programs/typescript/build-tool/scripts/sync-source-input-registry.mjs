// Keep the executable's package data byte-identical to the reviewed corpus.
// Runtime hashing never looks outside its installed package.
import { readFileSync, lstatSync, openSync, writeFileSync, closeSync, renameSync, unlinkSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const checked = resolve(here, "../../../../specs/fixtures/build-tool-v1/language-source-input-registry.json");
const packaged = resolve(here, "../src/language-source-input-registry.json");
const mode = process.argv.slice(2);
if (mode.length !== 1 || !["--check", "--sync"].includes(mode[0])) {
  throw new Error("usage: sync-source-input-registry.mjs --check|--sync");
}
if (!lstatSync(checked).isFile()) throw new Error("refusing non-regular checked registry source");
const expected = readFileSync(checked);

let stat;
try {
  stat = lstatSync(packaged);
} catch (error) {
  if (error.code !== "ENOENT") throw error;
}
if (stat && !stat.isFile()) throw new Error("refusing non-regular package registry target");

if (mode[0] === "--check") {
  if (!stat || !readFileSync(packaged).equals(expected)) {
    throw new Error("packaged source-input registry differs from checked corpus");
  }
} else {
  const temp = `${packaged}.${process.pid}.tmp`;
  const handle = openSync(temp, "wx");
  try {
    writeFileSync(handle, expected);
  } finally {
    closeSync(handle);
  }
  try {
    renameSync(temp, packaged);
  } catch (error) {
    unlinkSync(temp);
    throw error;
  }
}
