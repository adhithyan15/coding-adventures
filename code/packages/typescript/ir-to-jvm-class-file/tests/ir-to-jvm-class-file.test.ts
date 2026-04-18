import { mkdtempSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
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

  it("writes class files with classpath layout", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-backend-"));
    try {
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      const target = writeClassFile(artifact, tempdir);
      expect(target).toBe(join(realpathSync.native(tempdir), "demo", "Example.class"));
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
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

  it("rejects symlinked parent directories", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-symlink-"));
    const sink = mkdtempSync(join(tmpdir(), "ts-jvm-symlink-sink-"));
    try {
      symlinkSync(sink, join(tempdir, "demo"));
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      expect(() => writeClassFile(artifact, tempdir)).toThrow(JvmBackendError);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
      rmSync(sink, { recursive: true, force: true });
    }
  });

  it("rejects output roots with symlinked ancestors", () => {
    const tempdir = mkdtempSync(join(tmpdir(), "ts-jvm-root-link-"));
    const sink = mkdtempSync(join(tmpdir(), "ts-jvm-root-link-sink-"));
    try {
      symlinkSync(sink, join(tempdir, "linked-root"));
      const artifact = lowerIrToJvmClassFile(simpleProgram(), { className: "demo.Example" });
      expect(() => writeClassFile(artifact, join(tempdir, "linked-root", "nested"))).toThrow(
        JvmBackendError,
      );
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
      rmSync(fileRoot, { force: true });
      writeFileSync(fileRoot, "x");
      expect(() => writeClassFile(artifact, fileRoot)).toThrow(JvmBackendError);
    } finally {
      rmSync(tempdir, { recursive: true, force: true });
    }
  });
});
