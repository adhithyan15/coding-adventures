import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  ACC_PUBLIC,
  ACC_STATIC,
  buildMinimalClassFile,
  ClassFileFormatError,
  parseClassFile,
} from "../src/index.js";

function helloWorldFixture(): Uint8Array {
  return buildMinimalClassFile({
    className: "demo/HelloWorld",
    methodName: "main",
    descriptor: "([Ljava/lang/String;)V",
    code: Uint8Array.of(0xb1),
    maxStack: 0,
    maxLocals: 1,
    classAccessFlags: ACC_PUBLIC,
    methodAccessFlags: ACC_PUBLIC | ACC_STATIC,
  });
}

function richFixture(): Uint8Array {
  const utf8 = (value: string): Uint8Array => {
    const encoded = new TextEncoder().encode(value);
    return bytes(1, u2(encoded.length), encoded);
  };
  const klass = (nameIndex: number): Uint8Array => bytes(7, u2(nameIndex));
  const stringInfo = (stringIndex: number): Uint8Array => bytes(8, u2(stringIndex));
  const integer = (value: number): Uint8Array => {
    const buffer = new ArrayBuffer(4);
    new DataView(buffer).setInt32(0, value, false);
    return bytes(3, new Uint8Array(buffer));
  };
  const nameAndType = (nameIndex: number, descriptorIndex: number): Uint8Array =>
    bytes(12, u2(nameIndex), u2(descriptorIndex));
  const fieldref = (classIndex: number, nameAndTypeIndex: number): Uint8Array =>
    bytes(9, u2(classIndex), u2(nameAndTypeIndex));
  const methodref = (classIndex: number, nameAndTypeIndex: number): Uint8Array =>
    bytes(10, u2(classIndex), u2(nameAndTypeIndex));

  const entries = [
    utf8("demo/Rich"),
    klass(1),
    utf8("java/lang/Object"),
    klass(3),
    utf8("Code"),
    utf8("message"),
    utf8("Ljava/lang/String;"),
    nameAndType(6, 7),
    utf8("println"),
    utf8("(I)V"),
    nameAndType(9, 10),
    utf8("java/io/PrintStream"),
    klass(12),
    fieldref(13, 8),
    methodref(13, 11),
    utf8("hello"),
    stringInfo(16),
    integer(7),
    utf8("main"),
    utf8("([Ljava/lang/String;)V"),
  ];

  const code = Uint8Array.of(0xb1);
  const codeAttribute = bytes(
    u2(5),
    u4(19),
    u2(0),
    u2(1),
    u4(code.length),
    code,
    u2(0),
    u2(1),
    u2(5),
    u4(0),
  );
  const method = bytes(
    u2(ACC_PUBLIC | ACC_STATIC),
    u2(19),
    u2(20),
    u2(1),
    codeAttribute,
  );

  return bytes(
    u4(0xcafebabe),
    u2(0),
    u2(49),
    u2(entries.length + 1),
    ...entries,
    u2(ACC_PUBLIC),
    u2(2),
    u2(4),
    u2(0),
    u2(0),
    u2(1),
    method,
    u2(0),
  );
}

describe("jvm-class-file", () => {
  it("builds and parses a minimal class file", () => {
    const bytes = buildMinimalClassFile({
      className: "Example",
      methodName: "_start",
      descriptor: "()I",
      code: Uint8Array.of(0x03, 0xac),
      maxStack: 1,
      maxLocals: 0,
    });

    const parsed = parseClassFile(bytes);
    assert.equal(parsed.thisClassName, "Example");
    assert.equal(parsed.superClassName, "java/lang/Object");
    assert.deepEqual(
      parsed.findMethod("_start", "()I")?.codeAttribute()?.code,
      Uint8Array.of(0x03, 0xac),
    );
  });

  it("rejects invalid magic", () => {
    assert.throws(() => parseClassFile(Uint8Array.of(0, 1, 2, 3)), ClassFileFormatError);
  });

  it("resolves constants and method lookups", () => {
    const parsed = parseClassFile(helloWorldFixture());
    const method = parsed.findMethod("main", "([Ljava/lang/String;)V");
    assert.notEqual(method, null);
    assert.equal(method?.codeAttribute()?.name, "Code");
  });

  it("resolves class, field, method, and loadable constants", () => {
    const parsed = parseClassFile(richFixture());
    assert.equal(parsed.resolveClassName(2), "demo/Rich");
    assert.deepEqual(parsed.resolveFieldref(14), {
      className: "java/io/PrintStream",
      name: "message",
      descriptor: "Ljava/lang/String;",
    });
    assert.deepEqual(parsed.resolveMethodref(15), {
      className: "java/io/PrintStream",
      name: "println",
      descriptor: "(I)V",
    });
    assert.equal(parsed.resolveConstant(17), "hello");
    assert.equal(parsed.resolveConstant(18), 7);
    assert.equal(parsed.ldcConstants().get(17), "hello");
    assert.equal(parsed.findMethod("missing"), null);
  });

  it("parses nested non-Code attributes inside Code", () => {
    const parsed = parseClassFile(richFixture());
    const nested = parsed.findMethod("main")?.codeAttribute()?.nestedAttributes;
    assert.equal(nested?.length, 1);
    assert.equal(nested?.[0]?.name, "Code");
    assert.equal(nested?.[0]?.info.length, 0);
  });

  it("raises format errors for bad constant lookups", () => {
    const parsed = parseClassFile(richFixture());
    assert.throws(() => parsed.getUtf8(2), ClassFileFormatError);
    assert.throws(() => parsed.resolveClassName(1), ClassFileFormatError);
    assert.throws(() => parsed.resolveNameAndType(17), ClassFileFormatError);
    assert.throws(() => parsed.resolveConstant(14), ClassFileFormatError);
    assert.throws(() => parsed.resolveFieldref(15), ClassFileFormatError);
    assert.throws(() => parsed.resolveMethodref(14), ClassFileFormatError);
  });

  it("rejects malformed attribute bodies", () => {
    const malformed = richFixture().slice();
    malformed[malformed.length - 1] = 1;
    assert.throws(() => parseClassFile(malformed), ClassFileFormatError);
  });
});

function u2(value: number): Uint8Array {
  return Uint8Array.of((value >>> 8) & 0xff, value & 0xff);
}

function u4(value: number): Uint8Array {
  return Uint8Array.of(
    (value >>> 24) & 0xff,
    (value >>> 16) & 0xff,
    (value >>> 8) & 0xff,
    value & 0xff,
  );
}

function bytes(...parts: Array<number | Uint8Array>): Uint8Array {
  const normalized = parts.map((part) => (typeof part === "number" ? Uint8Array.of(part) : part));
  const total = normalized.reduce((sum, part) => sum + part.length, 0);
  const result = new Uint8Array(total);
  let offset = 0;
  for (const part of normalized) {
    result.set(part, offset);
    offset += part.length;
  }
  return result;
}
