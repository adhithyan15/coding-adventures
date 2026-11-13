// smoke.test.js -- load the freshly built addon into a real Node.js and call it.
//
// font-parser-node has no Rust unit tests: it is a thin N-API shim over the
// font-parser crate, and the only thing that can go wrong in the shim is the
// boundary itself -- a symbol that does not link, a handle that is mishandled,
// an exception that is not thrown.  Those failures only show up when a real
// Node.js process loads the compiled library, so this test does exactly that.
//
//   cargo build --release -p font-parser-node --target-dir target
//   node tests/smoke.test.js
//
// What it proves on Windows in particular: the cdylib that rustc links against
// node.exe's raw-dylib import table (see node-bridge/src/lib.rs) really loads
// into node.exe and every napi_* import resolves.
//
// The built library is copied to a temporary `font_parser_native.node` first,
// because Node only `require`s files that end in `.node`, and cargo's file
// name differs per platform (lib*.so, lib*.dylib, *.dll).

"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const PACKAGE_DIR = path.resolve(__dirname, "..");
const FIXTURE = path.resolve(PACKAGE_DIR, "..", "..", "..", "fixtures", "fonts", "Inter-Regular.ttf");

function builtLibraryName() {
  switch (process.platform) {
    case "win32":
      return "font_parser_native.dll";
    case "darwin":
      return "libfont_parser_native.dylib";
    default:
      return "libfont_parser_native.so";
  }
}

function loadAddon() {
  // The recipe passes `--target-dir target`, deliberately NOT the shared
  // workspace target: font-parser-node, -python and -ruby all emit a cdylib
  // named font_parser_native and would overwrite each other's file.
  const targetDir = path.join(PACKAGE_DIR, "target");
  const built = path.join(targetDir, "release", builtLibraryName());
  assert.ok(fs.existsSync(built), `built addon not found at ${built}; run the cargo build first`);
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "font-parser-node-"));
  const addon = path.join(dir, "font_parser_native.node");
  fs.copyFileSync(built, addon);
  return require(addon);
}

const fp = loadAddon();
const bytes = fs.readFileSync(FIXTURE);

test("exports the documented surface", () => {
  for (const name of ["load", "fontMetrics", "glyphId", "glyphMetrics", "kerning"]) {
    assert.equal(typeof fp[name], "function", name);
  }
  assert.equal(typeof fp.FontFile, "function");
});

test("load() returns a FontFile and fontMetrics() reads Inter's head/hhea/OS2/name", () => {
  const font = fp.load(bytes);
  assert.ok(font instanceof fp.FontFile);
  const m = fp.fontMetrics(font);
  assert.equal(m.unitsPerEm, 2048);
  assert.equal(m.ascender, 1984);
  assert.equal(m.descender, -494);
  assert.equal(m.lineGap, 0);
  assert.equal(m.xHeight, 1118);
  assert.equal(m.capHeight, 1490);
  assert.equal(m.numGlyphs, 2926);
  assert.equal(m.familyName, "Inter");
  assert.equal(m.subfamilyName, "Regular");
});

test("glyphId() and glyphMetrics() follow the cmap and hmtx tables", () => {
  const font = fp.load(bytes);
  const a = fp.glyphId(font, 0x41);
  assert.equal(a, 2);
  assert.deepEqual(fp.glyphMetrics(font, a), { advanceWidth: 1413, leftSideBearing: 52 });
  // U+10FFFF has no glyph: null, not an exception.
  assert.equal(fp.glyphId(font, 0x10ffff), null);
});

test("kerning() is zero for Inter (GPOS, not the legacy kern table)", () => {
  const font = fp.load(bytes);
  assert.equal(fp.kerning(font, fp.glyphId(font, 0x41), fp.glyphId(font, 0x56)), 0);
});

test("bad input throws instead of crashing the process", () => {
  const font = fp.load(bytes);
  assert.throws(() => fp.load(Buffer.from("junk")), /buffer too short/);
  assert.throws(() => fp.load("not a buffer"), /must be a Buffer/);
  assert.throws(() => fp.load(), /requires one argument/);
  assert.throws(() => fp.fontMetrics({}), /not a valid FontFile/);
  assert.throws(() => fp.glyphId(font, -1), /non-negative/);
  assert.throws(() => fp.glyphMetrics(font, 70000), /0\.\.65535/);
  assert.throws(() => fp.kerning(font, 1), /requires three arguments/);
});
