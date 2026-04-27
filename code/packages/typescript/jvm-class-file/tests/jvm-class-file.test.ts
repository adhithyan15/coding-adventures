import { describe, expect, it } from "vitest";

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
    expect(parsed.thisClassName).toBe("Example");
    expect(parsed.superClassName).toBe("java/lang/Object");
    expect(parsed.findMethod("_start", "()I")?.codeAttribute()?.code).toEqual(
      Uint8Array.of(0x03, 0xac),
    );
  });

  it("rejects invalid magic", () => {
    expect(() => parseClassFile(Uint8Array.of(0, 1, 2, 3))).toThrow(ClassFileFormatError);
  });

  it("resolves constants and method lookups", () => {
    const parsed = parseClassFile(helloWorldFixture());
    const method = parsed.findMethod("main", "([Ljava/lang/String;)V");
    expect(method).not.toBeNull();
    expect(method?.codeAttribute()?.name).toBe("Code");
  });

  it("resolves class, field, method, and loadable constants", () => {
    const parsed = parseClassFile(richFixture());
    expect(parsed.resolveClassName(2)).toBe("demo/Rich");
    expect(parsed.resolveFieldref(14)).toEqual({
      className: "java/io/PrintStream",
      name: "message",
      descriptor: "Ljava/lang/String;",
    });
    expect(parsed.resolveMethodref(15)).toEqual({
      className: "java/io/PrintStream",
      name: "println",
      descriptor: "(I)V",
    });
    expect(parsed.resolveConstant(17)).toBe("hello");
    expect(parsed.resolveConstant(18)).toBe(7);
    expect(parsed.ldcConstants().get(17)).toBe("hello");
    expect(parsed.findMethod("missing")).toBeNull();
  });

  it("parses nested non-Code attributes inside Code", () => {
    const parsed = parseClassFile(richFixture());
    const nested = parsed.findMethod("main")?.codeAttribute()?.nestedAttributes;
    expect(nested).toHaveLength(1);
    expect(nested?.[0]?.name).toBe("Code");
    expect(nested?.[0]?.info.length).toBe(0);
  });

  it("raises format errors for bad constant lookups", () => {
    const parsed = parseClassFile(richFixture());
    expect(() => parsed.getUtf8(2)).toThrow(ClassFileFormatError);
    expect(() => parsed.resolveClassName(1)).toThrow(ClassFileFormatError);
    expect(() => parsed.resolveNameAndType(17)).toThrow(ClassFileFormatError);
    expect(() => parsed.resolveConstant(14)).toThrow(ClassFileFormatError);
    expect(() => parsed.resolveFieldref(15)).toThrow(ClassFileFormatError);
    expect(() => parsed.resolveMethodref(14)).toThrow(ClassFileFormatError);
  });

  it("rejects malformed attribute bodies", () => {
    const malformed = richFixture().slice();
    malformed[malformed.length - 1] = 1;
    expect(() => parseClassFile(malformed)).toThrow(ClassFileFormatError);
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
