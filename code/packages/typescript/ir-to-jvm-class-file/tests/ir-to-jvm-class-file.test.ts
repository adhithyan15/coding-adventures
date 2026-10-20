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

describe("ir-to-jvm-class-file", () => {
  it("lowers a simple program to a parseable class file", () => {
    const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "Example" });
    const parsed = parseClassFile(artifact.classBytes);
    expect(parsed.thisClassName).toBe("Example");
    expect(parsed.findMethod("_start", "()I")).not.toBeNull();
    expect(parsed.findMethod("main", "([Ljava/lang/String;)V")).not.toBeNull();
    expect(parsed.findMethod("__ca_syscall", "(I)V")).not.toBeNull();
  });

  it("rejects invalid class names", () => {
    expect(() => lowerIrToJvmClassFile(simpleProgram(), { className: ".Bad" })).toThrow(
      JvmBackendError,
    );
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

  it("rejects class-file paths that Windows would reinterpret", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-bad-path-"));
    try {
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      for (const classFilename of [
        "demo\\Example.class",
        "C:/Example.class",
        "demo/Example.class:stream",
        "demo/\0Example.class",
        "../Example.class",
        "./",
        "demo./Example.class",
        "demo/Example.class ",
        "demo/.. /Example.class",
        "demo/NUL.class",
        "con/Example.class",
        "demo/COM1",
        "LPT9.tar.class",
      ]) {
        expect(() => writeClassFile({ ...artifact, classFilename }, tempdir)).toThrow(JvmBackendError);
      }
      expect(readdirSync(tempdir)).toEqual([]);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });
});

// The filesystem tests run under both write strategies. "auto" uses the
// directory-FD walk on POSIX; "path-checks" is what "auto" picks on Windows,
// where os.open() cannot open a directory and dir_fd= is unsupported. Forcing
// it here keeps the Windows code path covered on every CI platform.
const STRATEGIES: readonly ClassFileWriteStrategy[] = ["auto", "path-checks"];

describe.each(STRATEGIES)("writeClassFile (strategy %s)", (strategy) => {
  const write = (artifact: ReturnType<typeof lowerIrToJvmClassFile>, outputDir: string) =>
    writeClassFileWithStrategy(artifact, outputDir, strategy);
  // The dir-fd walk surfaces the kernel's refusal (ELOOP, ENOTDIR, EISDIR);
  // the path-check walk must refuse on its own, before touching the target,
  // so for it we also pin the message to prove the check (not luck) fired.
  const expectRejected = (attempt: () => unknown) => {
    expect(attempt).toThrow(JvmBackendError);
    if (strategy === "path-checks") {
      expect(attempt).toThrow(/Refusing to write through symlinked or invalid/);
    }
  };

  it("writes class files with classpath layout", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-backend-"));
    try {
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      const target = write(artifact, tempdir);
      expect(target).toBe(join(realpathSync.native(tempdir), "demo", "Example.class"));
      expect(new Uint8Array(readFileSync(target))).toEqual(artifact.classBytes);
      // No temp files left beside the class file.
      expect(readdirSync(join(tempdir, "demo"))).toEqual(["Example.class"]);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });

  it("creates missing output roots and overwrites existing class files", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-nested-"));
    try {
      const outputDir = join(tempdir, "out", "classes");
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "a.b.Example" });
      const target = join(realpathSync.native(tempdir), "out", "classes", "a", "b", "Example.class");
      mkdirSync(join(outputDir, "a", "b"), { recursive: true });
      writeFileSync(target, "stale");
      expect(write(artifact, outputDir)).toBe(target);
      expect(new Uint8Array(readFileSync(target))).toEqual(artifact.classBytes);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });

  it("rejects symlinked parent directories", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-symlink-"));
    const sink = mkdtempSync(join(tmpdir(), "ts-jvm-symlink-sink-"));
    try {
      symlinkSync(sink, join(tempdir, "demo"), "dir");
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      expectRejected(() => write(artifact, tempdir));
      expect(readdirSync(sink)).toEqual([]);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
      rmSync(sink, { recursive: true, force: true });
    }
  });

  it("rejects symlinked output files", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-file-link-"));
    const sink = mkdtempSync(join(tmpdir(), "ts-jvm-file-link-sink-"));
    try {
      const victim = join(sink, "victim.txt");
      writeFileSync(victim, "untouched");
      mkdirSync(join(tempdir, "demo"));
      symlinkSync(victim, join(tempdir, "demo", "Example.class"), "file");
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      expectRejected(() => write(artifact, tempdir));
      expect(readFileSync(victim, "utf8")).toBe("untouched");
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
      rmSync(sink, { recursive: true, force: true });
    }
  });

  it("rejects output roots with symlinked ancestors", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-root-link-"));
    const sink = mkdtempSync(join(tmpdir(), "ts-jvm-root-link-sink-"));
    try {
      symlinkSync(sink, join(tempdir, "linked-root"), "dir");
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      expectRejected(() => write(artifact, join(tempdir, "linked-root", "nested")));
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
      expectRejected(() => write(artifact, fileRoot));
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });

  it("rejects class-file paths that collide with directories", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-dir-target-"));
    try {
      mkdirSync(join(tempdir, "demo", "Example.class"), { recursive: true });
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      expectRejected(() => write(artifact, tempdir));
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });
});
