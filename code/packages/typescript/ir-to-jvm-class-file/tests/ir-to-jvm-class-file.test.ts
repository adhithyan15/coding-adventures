import {
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { IrOp, IrProgram, imm, lbl, reg } from "@coding-adventures/compiler-ir";
import { parseClassFile } from "@coding-adventures/jvm-class-file";

import {
  JvmBackendError,
  lowerIrToJvmClassFile,
  writeClassFile,
} from "../src/index.js";
import { type ClassFileWriteStrategy, writeClassFileWithStrategy } from "../src/backend.js";

function simpleProgram(): IrProgram {
  const program = new IrProgram("_start");
  program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
  program.addInstruction({ opcode: IrOp.LOAD_IMM, operands: [reg(1), imm(0)], id: 0 });
  program.addInstruction({ opcode: IrOp.HALT, operands: [], id: 1 });
  return program;
}

function programWithCallAndStaticData(): IrProgram {
  const program = new IrProgram("_start");
  program.addData({ label: "message", size: 4, init: 65 });
  program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
  program.addInstruction({ opcode: IrOp.CALL, operands: [lbl("_fn_main")], id: 0 });
  program.addInstruction({ opcode: IrOp.HALT, operands: [], id: 1 });
  program.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_fn_main")], id: -1 });
  program.addInstruction({ opcode: IrOp.LOAD_IMM, operands: [reg(1), imm(7)], id: 2 });
  program.addInstruction({ opcode: IrOp.RET, operands: [], id: 3 });
  return program;
}

// A directory link that every platform can create without privileges.
// On Windows the "junction" type makes a junction — exactly the reparse
// point an attacker without SeCreateSymbolicLinkPrivilege could plant.
// POSIX ignores the type argument and makes an ordinary symlink.
function linkDirectory(target: string, linkPath: string): void {
  symlinkSync(target, linkPath, "junction");
}

// File symlinks need a privilege on Windows that a developer machine may not
// grant.  Report whether the link was made so the caller can skip instead of
// passing (or failing) for the wrong reason.
function tryLinkFile(target: string, linkPath: string): boolean {
  try {
    symlinkSync(target, linkPath, "file");
    return true;
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "EPERM") {
      return false;
    }
    throw error;
  }
}

describe("ir-to-jvm-class-file", () => {
  it("lowers a simple program to a parseable class file", () => {
    const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "Example" });
    const parsed = parseClassFile(artifact.classBytes);
    expect(parsed.thisClassName).toBe("Example");
    expect(parsed.findMethod("_start", "()I")).not.toBeNull();
    expect(parsed.findMethod("main", "([Ljava/lang/String;)V")).not.toBeNull();
    expect(parsed.findMethod("__ca_syscall", "(I)V")).not.toBeNull();
  });

  it("writes class files with classpath layout", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-backend-"));
    try {
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      const target = writeClassFile(artifact, tempdir);
      expect(target).toBe(join(realpathSync.native(tempdir), "demo", "Example.class"));
      expect(new Uint8Array(readFileSync(target))).toEqual(artifact.classBytes);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });

  it("rejects invalid class names", () => {
    expect(() => lowerIrToJvmClassFile(simpleProgram(), { className: ".Bad" })).toThrow(
      JvmBackendError,
    );
  });

  it("rejects hand-built artifacts whose class filename escapes the root", () => {
    const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "Example" });
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-escape-"));
    try {
      for (const classFilename of [
        "../Example.class",
        "demo/./Example.class",
        "demo\\..\\..\\Example.class",
        "C:Example.class",
        "demo/Exa\0mple.class",
        "/",
      ]) {
        expect(() => writeClassFile({ ...artifact, classFilename }, tempdir)).toThrow(
          JvmBackendError,
        );
      }
      expect(readdirSync(tempdir)).toEqual([]);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });

  it("lowers callable splits and static data layout", () => {
    const artifact = lowerIrToJvmClassFile(programWithCallAndStaticData(), { className: "NibProgram" });
    const parsed = parseClassFile(artifact.classBytes);
    expect(parsed.findMethod("_start", "()I")).not.toBeNull();
    expect(parsed.findMethod("_fn_main", "()I")).not.toBeNull();
    expect(artifact.dataOffsets.get("message")).toBe(0);
  });

  it("rejects malformed operands that violate the backend contract", () => {
    const badImmediate = new IrProgram("_start");
    badImmediate.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    badImmediate.addInstruction({ opcode: IrOp.SYSCALL, operands: [lbl("not-an-immediate")], id: 0 });
    expect(() => lowerIrToJvmClassFile(badImmediate, { className: "BadImmediate" })).toThrow(
      JvmBackendError,
    );

    const badLabel = new IrProgram("_start");
    badLabel.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    badLabel.addInstruction({ opcode: IrOp.JUMP, operands: [imm(1)], id: 0 });
    expect(() => lowerIrToJvmClassFile(badLabel, { className: "BadLabel" })).toThrow(
      JvmBackendError,
    );
  });
});

// Every write-safety property is checked under both walks.  On POSIX "auto"
// is the descriptor walk, so running "path-checks" here is what keeps the
// Windows walk tested on every platform; on Windows both rows exercise it.
const STRATEGIES: readonly ClassFileWriteStrategy[] = ["auto", "path-checks"];

describe.each(STRATEGIES)("secure class-file writer (strategy: %s)", (strategy) => {
  const write = (artifact: ReturnType<typeof lowerIrToJvmClassFile>, outputDir: string) =>
    writeClassFileWithStrategy(artifact, outputDir, strategy);

  it("creates missing root and package directories", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-create-"));
    try {
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "a.b.Example" });
      const target = write(artifact, join(tempdir, "out", "classes"));
      expect(target).toBe(
        join(realpathSync.native(tempdir), "out", "classes", "a", "b", "Example.class"),
      );
      expect(new Uint8Array(readFileSync(target))).toEqual(artifact.classBytes);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });

  it("overwrites an existing class file and leaves no temp files behind", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-overwrite-"));
    try {
      const first = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      const second = lowerIrToJvmClassFile(programWithCallAndStaticData(), { className: "demo.Example" });
      write(first, tempdir);
      const target = write(second, tempdir);
      expect(new Uint8Array(readFileSync(target))).toEqual(second.classBytes);
      expect(readdirSync(join(tempdir, "demo"))).toEqual(["Example.class"]);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });

  it("rejects symlinked parent directories", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-symlink-"));
    const sink = mkdtempSync(join(tmpdir(), "ts-jvm-symlink-sink-"));
    try {
      linkDirectory(sink, join(tempdir, "demo"));
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      expect(() => write(artifact, tempdir)).toThrow(JvmBackendError);
      expect(readdirSync(sink)).toEqual([]);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
      rmSync(sink, { recursive: true, force: true });
    }
  });

  it("rejects output roots with symlinked ancestors", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-root-link-"));
    const sink = mkdtempSync(join(tmpdir(), "ts-jvm-root-link-sink-"));
    try {
      linkDirectory(sink, join(tempdir, "linked-root"));
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      expect(() => write(artifact, join(tempdir, "linked-root", "nested"))).toThrow(
        JvmBackendError,
      );
      expect(readdirSync(sink)).toEqual([]);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
      rmSync(sink, { recursive: true, force: true });
    }
  });

  it("rejects an output root that is itself a symlink", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-root-self-link-"));
    const sink = mkdtempSync(join(tmpdir(), "ts-jvm-root-self-link-sink-"));
    try {
      linkDirectory(sink, join(tempdir, "linked-root"));
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "Example" });
      expect(() => write(artifact, join(tempdir, "linked-root"))).toThrow(JvmBackendError);
      expect(readdirSync(sink)).toEqual([]);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
      rmSync(sink, { recursive: true, force: true });
    }
  });

  it("rejects output roots that are files instead of directories", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-file-root-"));
    const fileRoot = join(tempdir, "not-a-directory");
    try {
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "Example" });
      writeFileSync(fileRoot, "x");
      expect(() => write(artifact, fileRoot)).toThrow(JvmBackendError);
      expect(readFileSync(fileRoot, "utf8")).toBe("x");
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });

  it("rejects a package directory that is a file", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-file-package-"));
    try {
      writeFileSync(join(tempdir, "demo"), "x");
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      expect(() => write(artifact, tempdir)).toThrow(JvmBackendError);
      expect(readFileSync(join(tempdir, "demo"), "utf8")).toBe("x");
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });

  it("rejects an output file that is a directory", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-dir-target-"));
    try {
      mkdirSync(join(tempdir, "Example.class"));
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "Example" });
      expect(() => write(artifact, tempdir)).toThrow(JvmBackendError);
      expect(readdirSync(tempdir)).toEqual(["Example.class"]);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });

  it("refuses to write through a symlinked output file", (context) => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-file-link-"));
    const sink = mkdtempSync(join(tmpdir(), "ts-jvm-file-link-sink-"));
    try {
      const sentinel = join(sink, "sentinel.txt");
      writeFileSync(sentinel, "untouched");
      if (!tryLinkFile(sentinel, join(tempdir, "Example.class"))) {
        context.skip();
      }
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "Example" });
      expect(() => write(artifact, tempdir)).toThrow(JvmBackendError);
      expect(readFileSync(sentinel, "utf8")).toBe("untouched");
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
      rmSync(sink, { recursive: true, force: true });
    }
  });
});
