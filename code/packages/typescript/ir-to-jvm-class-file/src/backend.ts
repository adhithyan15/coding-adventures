import { realpathSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { join, resolve } from "node:path";

import {
  ACC_PUBLIC,
  ACC_STATIC,
  ACC_SUPER,
} from "@coding-adventures/jvm-class-file";
import {
  IrOp,
  type IrInstruction,
  type IrLabel,
  type IrOperand,
  type IrProgram,
  type IrRegister,
} from "@coding-adventures/compiler-ir";

const ACC_PRIVATE = 0x0002;
const ACC_FINAL = 0x0010;

const OP_NOP = 0x00;
const OP_ICONST_M1 = 0x02;
const OP_ICONST_0 = 0x03;
const OP_ICONST_1 = 0x04;
const OP_ICONST_2 = 0x05;
const OP_ICONST_3 = 0x06;
const OP_ICONST_4 = 0x07;
const OP_ICONST_5 = 0x08;
const OP_BIPUSH = 0x10;
const OP_SIPUSH = 0x11;
const OP_LDC = 0x12;
const OP_ILOAD = 0x15;
const OP_ALOAD_0 = 0x2a;
const OP_IALOAD = 0x2e;
const OP_BALOAD = 0x33;
const OP_ISTORE = 0x36;
const OP_IASTORE = 0x4f;
const OP_BASTORE = 0x54;
const OP_POP = 0x57;
const OP_IADD = 0x60;
const OP_ISUB = 0x64;
const OP_ISHL = 0x78;
const OP_ISHR = 0x7a;
const OP_IAND = 0x7e;
const OP_I2B = 0x91;
const OP_IFEQ = 0x99;
const OP_IFNE = 0x9a;
const OP_IF_ICMPEQ = 0x9f;
const OP_IF_ICMPNE = 0xa0;
const OP_IF_ICMPLT = 0xa1;
const OP_IF_ICMPGT = 0xa3;
const OP_GOTO = 0xa7;
const OP_IRETURN = 0xac;
const OP_RETURN = 0xb1;
const OP_GETSTATIC = 0xb2;
const OP_PUTSTATIC = 0xb3;
const OP_INVOKEVIRTUAL = 0xb6;
const OP_INVOKESTATIC = 0xb8;
const OP_NEWARRAY = 0xbc;

const ATYPE_INT = 10;
const ATYPE_BYTE = 8;

const DESC_INT = "I";
const DESC_VOID = "V";
const DESC_INT_ARRAY = "[I";
const DESC_BYTE_ARRAY = "[B";
const DESC_MAIN = "([Ljava/lang/String;)V";
const DESC_NOARGS_INT = "()I";
const DESC_INT_TO_INT = "(I)I";
const DESC_INT_INT_TO_VOID = "(II)V";
const DESC_ARRAYS_FILL_BYTE_RANGE = "([BIIB)V";
const DESC_PRINTSTREAM_WRITE = "(I)V";
const DESC_PRINTSTREAM_FLUSH = "()V";
const DESC_INPUTSTREAM_READ = "()I";

const JAVA_BINARY_NAME_RE = /^[A-Za-z_$][A-Za-z0-9_$]*(?:\.[A-Za-z_$][A-Za-z0-9_$]*)*$/;
const MAX_STATIC_DATA_BYTES = 16 * 1024 * 1024;

export class JvmBackendError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "JvmBackendError";
  }
}

export interface JvmBackendConfig {
  readonly className: string;
  readonly classFileMajor?: number;
  readonly classFileMinor?: number;
  readonly emitMainWrapper?: boolean;
}

export interface JVMClassArtifact {
  readonly className: string;
  readonly classBytes: Uint8Array;
  readonly callableLabels: readonly string[];
  readonly dataOffsets: ReadonlyMap<string, number>;
  readonly classFilename: string;
}

interface FieldSpec {
  readonly accessFlags: number;
  readonly name: string;
  readonly descriptor: string;
}

interface MethodSpec {
  readonly accessFlags: number;
  readonly name: string;
  readonly descriptor: string;
  readonly code: Uint8Array;
  readonly maxStack: number;
  readonly maxLocals: number;
}

interface CallableRegion {
  readonly label: string;
  readonly instructions: readonly IrInstruction[];
}

class ConstantPoolBuilder {
  private readonly entries: Uint8Array[] = [];
  private readonly indices = new Map<string, number>();

  utf8(value: string): number {
    const encoded = new TextEncoder().encode(value);
    return this.add(`Utf8:${value}`, concatBytes(u1(1), u2(encoded.length), encoded));
  }

  integer(value: number): number {
    const buffer = new ArrayBuffer(4);
    new DataView(buffer).setInt32(0, value, false);
      return this.add(`Integer:${value}`, concatBytes(u1(3), new Uint8Array(buffer)));
  }

  classRef(internalName: string): number {
    return this.add(`Class:${internalName}`, concatBytes(u1(7), u2(this.utf8(internalName))));
  }

  string(value: string): number {
    return this.add(`String:${value}`, concatBytes(u1(8), u2(this.utf8(value))));
  }

  nameAndType(name: string, descriptor: string): number {
    return this.add(
      `NameAndType:${name}:${descriptor}`,
      concatBytes(u1(12), u2(this.utf8(name)), u2(this.utf8(descriptor))),
    );
  }

  fieldRef(owner: string, name: string, descriptor: string): number {
    return this.add(
      `Fieldref:${owner}:${name}:${descriptor}`,
      concatBytes(u1(9), u2(this.classRef(owner)), u2(this.nameAndType(name, descriptor))),
    );
  }

  methodRef(owner: string, name: string, descriptor: string): number {
    return this.add(
      `Methodref:${owner}:${name}:${descriptor}`,
      concatBytes(u1(10), u2(this.classRef(owner)), u2(this.nameAndType(name, descriptor))),
    );
  }

  entriesBytes(): Uint8Array {
    return concatBytes(...this.entries);
  }

  size(): number {
    return this.entries.length + 1;
  }

  private add(key: string, payload: Uint8Array): number {
    const existing = this.indices.get(key);
    if (existing !== undefined) {
      return existing;
    }
    this.entries.push(payload);
    const index = this.entries.length;
    this.indices.set(key, index);
    return index;
  }
}

class BytecodeBuilder {
  private readonly bytes: number[] = [];
  private readonly labels = new Map<string, number>();
  private readonly branches: Array<{ offset: number; label: string }> = [];

  position(): number {
    return this.bytes.length;
  }

  mark(name: string): void {
    this.labels.set(name, this.bytes.length);
  }

  emitU1(value: number): void {
    this.bytes.push(value & 0xff);
  }

  emitU2(value: number): void {
    this.bytes.push((value >>> 8) & 0xff, value & 0xff);
  }

  emitPushInt(value: number, constants: ConstantPoolBuilder): void {
    if (value === -1) {
      this.emitU1(OP_ICONST_M1);
    } else if (value >= 0 && value <= 5) {
      this.emitU1(OP_ICONST_0 + value);
    } else if (value >= -128 && value <= 127) {
      this.emitU1(OP_BIPUSH);
      this.emitU1(value & 0xff);
    } else if (value >= -32768 && value <= 32767) {
      this.emitU1(OP_SIPUSH);
      this.emitU2(value & 0xffff);
    } else {
      const index = constants.integer(value);
      if (index > 0xff) {
        throw new JvmBackendError("Constant pool index for integer literal exceeds ldc support");
      }
      this.emitU1(OP_LDC);
      this.emitU1(index);
    }
  }

  emitBranch(opcode: number, label: string): void {
    this.emitU1(opcode);
    this.branches.push({ offset: this.bytes.length, label });
    this.emitU2(0);
  }

  patch(): Uint8Array {
    const result = Uint8Array.from(this.bytes);
    for (const branch of this.branches) {
      const target = this.labels.get(branch.label);
      if (target === undefined) {
        throw new JvmBackendError(`Unknown branch label ${branch.label}`);
      }
      const branchStart = branch.offset - 1;
      const delta = target - branchStart;
      if (delta < -32768 || delta > 32767) {
        throw new JvmBackendError(`Branch to ${branch.label} exceeds 16-bit offset range`);
      }
      result[branch.offset] = (delta >>> 8) & 0xff;
      result[branch.offset + 1] = delta & 0xff;
    }
    return result;
  }
}

class Lowerer {
  private readonly className: string;
  private readonly internalClassName: string;
  private readonly classFileMajor: number;
  private readonly classFileMinor: number;
  private readonly emitMainWrapper: boolean;
  private readonly regField = "__ca_regs";
  private readonly memField = "__ca_memory";
  private readonly regGetHelper = "__ca_regGet";
  private readonly regSetHelper = "__ca_regSet";
  private readonly memLoadByteHelper = "__ca_memLoadByte";
  private readonly memStoreByteHelper = "__ca_memStoreByte";
  private readonly loadWordHelper = "__ca_loadWord";
  private readonly storeWordHelper = "__ca_storeWord";
  private readonly syscallHelper = "__ca_syscall";
  private compareId = 0;

  constructor(
    private readonly program: IrProgram,
    config: JvmBackendConfig,
  ) {
    if (!JAVA_BINARY_NAME_RE.test(config.className)) {
      throw new JvmBackendError(`Class name "${config.className}" is not a legal Java binary name`);
    }
    this.className = config.className;
    this.internalClassName = config.className.replaceAll(".", "/");
    this.classFileMajor = config.classFileMajor ?? 49;
    this.classFileMinor = config.classFileMinor ?? 0;
    this.emitMainWrapper = config.emitMainWrapper ?? true;
  }

  lower(): JVMClassArtifact {
    const callableLabels = collectCallableLabels(this.program);
    this.ensureNoHelperCollisions(callableLabels);
    const regions = splitRegions(this.program, callableLabels);
    const dataOffsets = layoutData(this.program);
    const registerCount = Math.max(2, maxRegisterIndex(this.program) + 1);

    const pool = new ConstantPoolBuilder();
    const fields: FieldSpec[] = [
      { accessFlags: ACC_PRIVATE | ACC_STATIC, name: this.regField, descriptor: DESC_INT_ARRAY },
      { accessFlags: ACC_PRIVATE | ACC_STATIC, name: this.memField, descriptor: DESC_BYTE_ARRAY },
    ];
    const methods: MethodSpec[] = [];

    methods.push(this.buildClassInitializer(pool, registerCount, dataOffsets));
    methods.push(this.buildRegGet(pool));
    methods.push(this.buildRegSet(pool));
    methods.push(this.buildMemLoadByte(pool));
    methods.push(this.buildMemStoreByte(pool));
    methods.push(this.buildLoadWord(pool));
    methods.push(this.buildStoreWord(pool));
    methods.push(this.buildSyscall(pool));
    for (const region of regions) {
      methods.push(this.buildCallableRegion(pool, region, dataOffsets));
    }
    if (this.emitMainWrapper) {
      methods.push(this.buildMainWrapper(pool));
    }

    const classBytes = encodeClassFile({
      pool,
      accessFlags: ACC_PUBLIC | ACC_FINAL | ACC_SUPER,
      internalClassName: this.internalClassName,
      superClassName: "java/lang/Object",
      fields,
      methods,
      majorVersion: this.classFileMajor,
      minorVersion: this.classFileMinor,
    });

    return {
      className: this.className,
      classBytes,
      callableLabels,
      dataOffsets,
      classFilename: this.className.replaceAll(".", "/") + ".class",
    };
  }

  private ensureNoHelperCollisions(callableLabels: readonly string[]): void {
    const reserved = new Set([
      this.regGetHelper,
      this.regSetHelper,
      this.memLoadByteHelper,
      this.memStoreByteHelper,
      this.loadWordHelper,
      this.storeWordHelper,
      this.syscallHelper,
      "main",
      "<clinit>",
    ]);
    for (const label of callableLabels) {
      if (reserved.has(label)) {
        throw new JvmBackendError(`IR callable label ${label} collides with a generated JVM helper`);
      }
    }
  }

  private buildClassInitializer(
    pool: ConstantPoolBuilder,
    registerCount: number,
    dataOffsets: ReadonlyMap<string, number>,
  ): MethodSpec {
    const code = new BytecodeBuilder();
    code.emitPushInt(registerCount, pool);
    code.emitU1(OP_NEWARRAY);
    code.emitU1(ATYPE_INT);
    code.emitU1(OP_PUTSTATIC);
    code.emitU2(pool.fieldRef(this.internalClassName, this.regField, DESC_INT_ARRAY));

    const memoryBytes = totalStaticData(this.program);
    code.emitPushInt(memoryBytes, pool);
    code.emitU1(OP_NEWARRAY);
    code.emitU1(ATYPE_BYTE);
    code.emitU1(OP_PUTSTATIC);
    code.emitU2(pool.fieldRef(this.internalClassName, this.memField, DESC_BYTE_ARRAY));

    if (memoryBytes > MAX_STATIC_DATA_BYTES) {
      throw new JvmBackendError(`Total static data exceeds ${MAX_STATIC_DATA_BYTES} bytes`);
    }

    for (const decl of this.program.data) {
      if ((decl.init & 0xff) === 0 || decl.size === 0) {
        continue;
      }
      const start = dataOffsets.get(decl.label);
      if (start === undefined) {
        throw new JvmBackendError(`Missing data offset for ${decl.label}`);
      }
      code.emitU1(OP_GETSTATIC);
      code.emitU2(pool.fieldRef(this.internalClassName, this.memField, DESC_BYTE_ARRAY));
      code.emitPushInt(start, pool);
      code.emitPushInt(start + decl.size, pool);
      code.emitPushInt(decl.init & 0xff, pool);
      code.emitU1(OP_I2B);
      code.emitU1(OP_INVOKESTATIC);
      code.emitU2(pool.methodRef("java/util/Arrays", "fill", DESC_ARRAYS_FILL_BYTE_RANGE));
    }

    code.emitU1(OP_RETURN);
    return {
      accessFlags: ACC_STATIC,
      name: "<clinit>",
      descriptor: "()V",
      code: code.patch(),
      maxStack: 6,
      maxLocals: 0,
    };
  }

  private buildRegGet(pool: ConstantPoolBuilder): MethodSpec {
    const code = new BytecodeBuilder();
    code.emitU1(OP_GETSTATIC);
    code.emitU2(pool.fieldRef(this.internalClassName, this.regField, DESC_INT_ARRAY));
    emitLoadIntLocal(code, 0);
    code.emitU1(OP_IALOAD);
    code.emitU1(OP_IRETURN);
    return {
      accessFlags: ACC_PRIVATE | ACC_STATIC,
      name: this.regGetHelper,
      descriptor: DESC_INT_TO_INT,
      code: code.patch(),
      maxStack: 2,
      maxLocals: 1,
    };
  }

  private buildRegSet(pool: ConstantPoolBuilder): MethodSpec {
    const code = new BytecodeBuilder();
    code.emitU1(OP_GETSTATIC);
    code.emitU2(pool.fieldRef(this.internalClassName, this.regField, DESC_INT_ARRAY));
    emitLoadIntLocal(code, 0);
    emitLoadIntLocal(code, 1);
    code.emitU1(OP_IASTORE);
    code.emitU1(OP_RETURN);
    return {
      accessFlags: ACC_PRIVATE | ACC_STATIC,
      name: this.regSetHelper,
      descriptor: DESC_INT_INT_TO_VOID,
      code: code.patch(),
      maxStack: 3,
      maxLocals: 2,
    };
  }

  private buildMemLoadByte(pool: ConstantPoolBuilder): MethodSpec {
    const code = new BytecodeBuilder();
    code.emitU1(OP_GETSTATIC);
    code.emitU2(pool.fieldRef(this.internalClassName, this.memField, DESC_BYTE_ARRAY));
    emitLoadIntLocal(code, 0);
    code.emitU1(OP_BALOAD);
    code.emitPushInt(0xff, pool);
    code.emitU1(OP_IAND);
    code.emitU1(OP_IRETURN);
    return {
      accessFlags: ACC_PRIVATE | ACC_STATIC,
      name: this.memLoadByteHelper,
      descriptor: DESC_INT_TO_INT,
      code: code.patch(),
      maxStack: 2,
      maxLocals: 1,
    };
  }

  private buildMemStoreByte(pool: ConstantPoolBuilder): MethodSpec {
    const code = new BytecodeBuilder();
    code.emitU1(OP_GETSTATIC);
    code.emitU2(pool.fieldRef(this.internalClassName, this.memField, DESC_BYTE_ARRAY));
    emitLoadIntLocal(code, 0);
    emitLoadIntLocal(code, 1);
    code.emitU1(OP_I2B);
    code.emitU1(OP_BASTORE);
    code.emitU1(OP_RETURN);
    return {
      accessFlags: ACC_PRIVATE | ACC_STATIC,
      name: this.memStoreByteHelper,
      descriptor: DESC_INT_INT_TO_VOID,
      code: code.patch(),
      maxStack: 3,
      maxLocals: 2,
    };
  }

  private buildLoadWord(pool: ConstantPoolBuilder): MethodSpec {
    const code = new BytecodeBuilder();
    emitLoadIntLocal(code, 0);
    code.emitU1(OP_INVOKESTATIC);
    code.emitU2(pool.methodRef(this.internalClassName, this.memLoadByteHelper, DESC_INT_TO_INT));

    for (const shift of [8, 16, 24]) {
      emitLoadIntLocal(code, 0);
      code.emitPushInt(shift / 8, pool);
      code.emitU1(OP_IADD);
      code.emitU1(OP_INVOKESTATIC);
      code.emitU2(pool.methodRef(this.internalClassName, this.memLoadByteHelper, DESC_INT_TO_INT));
      code.emitPushInt(shift, pool);
      code.emitU1(OP_ISHL);
      code.emitU1(OP_IADD);
    }

    code.emitU1(OP_IRETURN);
    return {
      accessFlags: ACC_PRIVATE | ACC_STATIC,
      name: this.loadWordHelper,
      descriptor: DESC_INT_TO_INT,
      code: code.patch(),
      maxStack: 3,
      maxLocals: 1,
    };
  }

  private buildStoreWord(pool: ConstantPoolBuilder): MethodSpec {
    const code = new BytecodeBuilder();
    for (const shift of [0, 8, 16, 24]) {
      emitLoadIntLocal(code, 0);
      if (shift > 0) {
        code.emitPushInt(shift / 8, pool);
        code.emitU1(OP_IADD);
      }
      emitLoadIntLocal(code, 1);
      if (shift > 0) {
        code.emitPushInt(shift, pool);
        code.emitU1(OP_ISHR);
      }
      code.emitPushInt(0xff, pool);
      code.emitU1(OP_IAND);
      code.emitU1(OP_INVOKESTATIC);
      code.emitU2(pool.methodRef(this.internalClassName, this.memStoreByteHelper, DESC_INT_INT_TO_VOID));
    }
    code.emitU1(OP_RETURN);
    return {
      accessFlags: ACC_PRIVATE | ACC_STATIC,
      name: this.storeWordHelper,
      descriptor: DESC_INT_INT_TO_VOID,
      code: code.patch(),
      maxStack: 3,
      maxLocals: 2,
    };
  }

  private buildSyscall(pool: ConstantPoolBuilder): MethodSpec {
    const code = new BytecodeBuilder();
    const done = this.freshLabel("sys_done");
    const checkRead = this.freshLabel("sys_read");
    const checkExit = this.freshLabel("sys_exit");
    const writePath = this.freshLabel("sys_write");
    const readPositive = this.freshLabel("read_positive");

    emitLoadIntLocal(code, 0);
    code.emitPushInt(1, pool);
    code.emitBranch(OP_IF_ICMPEQ, writePath);
    emitLoadIntLocal(code, 0);
    code.emitPushInt(2, pool);
    code.emitBranch(OP_IF_ICMPEQ, checkRead);
    emitLoadIntLocal(code, 0);
    code.emitPushInt(10, pool);
    code.emitBranch(OP_IF_ICMPEQ, checkExit);
    code.emitBranch(OP_GOTO, done);

    code.mark(writePath);
    code.emitU1(OP_GETSTATIC);
    code.emitU2(pool.fieldRef("java/lang/System", "out", "Ljava/io/PrintStream;"));
    code.emitPushInt(4, pool);
    code.emitU1(OP_INVOKESTATIC);
    code.emitU2(pool.methodRef(this.internalClassName, this.regGetHelper, DESC_INT_TO_INT));
    code.emitU1(OP_INVOKEVIRTUAL);
    code.emitU2(pool.methodRef("java/io/PrintStream", "write", DESC_PRINTSTREAM_WRITE));
    code.emitU1(OP_GETSTATIC);
    code.emitU2(pool.fieldRef("java/lang/System", "out", "Ljava/io/PrintStream;"));
    code.emitU1(OP_INVOKEVIRTUAL);
    code.emitU2(pool.methodRef("java/io/PrintStream", "flush", DESC_PRINTSTREAM_FLUSH));
    code.emitBranch(OP_GOTO, done);

    code.mark(checkRead);
    code.emitU1(OP_GETSTATIC);
    code.emitU2(pool.fieldRef("java/lang/System", "in", "Ljava/io/InputStream;"));
    code.emitU1(OP_INVOKEVIRTUAL);
    code.emitU2(pool.methodRef("java/io/InputStream", "read", DESC_INPUTSTREAM_READ));
    emitStoreIntLocal(code, 1);
    emitLoadIntLocal(code, 1);
    code.emitPushInt(0, pool);
    code.emitPushInt(4, pool);
    code.emitBranch(OP_IF_ICMPGT, readPositive);
    code.emitPushInt(4, pool);
    code.emitPushInt(0, pool);
    code.emitU1(OP_INVOKESTATIC);
    code.emitU2(pool.methodRef(this.internalClassName, this.regSetHelper, DESC_INT_INT_TO_VOID));
    code.emitBranch(OP_GOTO, done);
    code.mark(readPositive);
    code.emitPushInt(4, pool);
    emitLoadIntLocal(code, 1);
    code.emitU1(OP_INVOKESTATIC);
    code.emitU2(pool.methodRef(this.internalClassName, this.regSetHelper, DESC_INT_INT_TO_VOID));
    code.emitBranch(OP_GOTO, done);

    code.mark(checkExit);
    code.emitBranch(OP_GOTO, done);

    code.mark(done);
    code.emitU1(OP_RETURN);
    return {
      accessFlags: ACC_PRIVATE | ACC_STATIC,
      name: this.syscallHelper,
      descriptor: "(I)V",
      code: code.patch(),
      maxStack: 3,
      maxLocals: 2,
    };
  }

  private buildCallableRegion(
    pool: ConstantPoolBuilder,
    region: CallableRegion,
    dataOffsets: ReadonlyMap<string, number>,
  ): MethodSpec {
    const code = new BytecodeBuilder();
    for (const instruction of region.instructions) {
      this.lowerInstruction(code, pool, instruction, dataOffsets);
    }
    if (region.instructions.length === 0 || region.instructions.at(-1)?.opcode !== IrOp.RET) {
      code.emitPushInt(0, pool);
      code.emitU1(OP_IRETURN);
    }
    return {
      accessFlags: ACC_PUBLIC | ACC_STATIC,
      name: region.label,
      descriptor: DESC_NOARGS_INT,
      code: code.patch(),
      maxStack: 8,
      maxLocals: 0,
    };
  }

  private buildMainWrapper(pool: ConstantPoolBuilder): MethodSpec {
    const code = new BytecodeBuilder();
    code.emitU1(OP_INVOKESTATIC);
    code.emitU2(pool.methodRef(this.internalClassName, this.program.entryLabel, DESC_NOARGS_INT));
    code.emitU1(OP_POP);
    code.emitU1(OP_RETURN);
    return {
      accessFlags: ACC_PUBLIC | ACC_STATIC,
      name: "main",
      descriptor: DESC_MAIN,
      code: code.patch(),
      maxStack: 1,
      maxLocals: 1,
    };
  }

  private lowerInstruction(
    code: BytecodeBuilder,
    pool: ConstantPoolBuilder,
    instruction: IrInstruction,
    dataOffsets: ReadonlyMap<string, number>,
  ): void {
    switch (instruction.opcode) {
      case IrOp.LABEL:
        code.mark(asLabel(instruction.operands[0]).name);
        return;
      case IrOp.COMMENT:
        return;
      case IrOp.NOP:
        code.emitU1(OP_NOP);
        return;
      case IrOp.LOAD_IMM:
        this.emitRegSet(code, pool, registerIndex(instruction.operands[0]), immediateValue(instruction.operands[1]));
        return;
      case IrOp.LOAD_ADDR: {
        const label = asLabel(instruction.operands[1]).name;
        const offset = dataOffsets.get(label);
        if (offset === undefined) {
          throw new JvmBackendError(`Unknown data label ${label}`);
        }
        this.emitRegSet(code, pool, registerIndex(instruction.operands[0]), offset);
        return;
      }
      case IrOp.ADD:
        this.emitBinaryOp(code, pool, instruction, OP_IADD);
        return;
      case IrOp.ADD_IMM:
        this.emitRegSetFromStack(code, pool, registerIndex(instruction.operands[0]), () => {
          this.emitRegGet(code, pool, registerIndex(instruction.operands[1]));
          code.emitPushInt(immediateValue(instruction.operands[2]), pool);
          code.emitU1(OP_IADD);
        });
        return;
      case IrOp.SUB:
        this.emitBinaryOp(code, pool, instruction, OP_ISUB);
        return;
      case IrOp.AND:
        this.emitBinaryOp(code, pool, instruction, OP_IAND);
        return;
      case IrOp.AND_IMM:
        this.emitRegSetFromStack(code, pool, registerIndex(instruction.operands[0]), () => {
          this.emitRegGet(code, pool, registerIndex(instruction.operands[1]));
          code.emitPushInt(immediateValue(instruction.operands[2]), pool);
          code.emitU1(OP_IAND);
        });
        return;
      case IrOp.CMP_EQ:
        this.emitCompare(code, pool, instruction, OP_IF_ICMPEQ);
        return;
      case IrOp.CMP_NE:
        this.emitCompare(code, pool, instruction, OP_IF_ICMPNE);
        return;
      case IrOp.CMP_LT:
        this.emitCompare(code, pool, instruction, OP_IF_ICMPLT);
        return;
      case IrOp.CMP_GT:
        this.emitCompare(code, pool, instruction, OP_IF_ICMPGT);
        return;
      case IrOp.JUMP:
        code.emitBranch(OP_GOTO, asLabel(instruction.operands[0]).name);
        return;
      case IrOp.BRANCH_Z:
        this.emitRegGet(code, pool, registerIndex(instruction.operands[0]));
        code.emitBranch(OP_IFEQ, asLabel(instruction.operands[1]).name);
        return;
      case IrOp.BRANCH_NZ:
        this.emitRegGet(code, pool, registerIndex(instruction.operands[0]));
        code.emitBranch(OP_IFNE, asLabel(instruction.operands[1]).name);
        return;
      case IrOp.CALL:
        code.emitPushInt(1, pool);
        code.emitU1(OP_INVOKESTATIC);
        code.emitU2(pool.methodRef(this.internalClassName, asLabel(instruction.operands[0]).name, DESC_NOARGS_INT));
        code.emitU1(OP_INVOKESTATIC);
        code.emitU2(pool.methodRef(this.internalClassName, this.regSetHelper, DESC_INT_INT_TO_VOID));
        return;
      case IrOp.RET:
        this.emitRegGet(code, pool, 1);
        code.emitU1(OP_IRETURN);
        return;
      case IrOp.SYSCALL:
        code.emitPushInt(immediateValue(instruction.operands[0]), pool);
        code.emitU1(OP_INVOKESTATIC);
        code.emitU2(pool.methodRef(this.internalClassName, this.syscallHelper, "(I)V"));
        return;
      case IrOp.HALT:
        code.emitPushInt(0, pool);
        code.emitU1(OP_IRETURN);
        return;
      case IrOp.LOAD_BYTE:
        this.emitRegSetFromStack(code, pool, registerIndex(instruction.operands[0]), () => {
          this.emitAddress(code, pool, instruction.operands[1], instruction.operands[2]);
          code.emitU1(OP_INVOKESTATIC);
          code.emitU2(pool.methodRef(this.internalClassName, this.memLoadByteHelper, DESC_INT_TO_INT));
        });
        return;
      case IrOp.STORE_BYTE:
        this.emitAddress(code, pool, instruction.operands[1], instruction.operands[2]);
        this.emitRegGet(code, pool, registerIndex(instruction.operands[0]));
        code.emitU1(OP_INVOKESTATIC);
        code.emitU2(pool.methodRef(this.internalClassName, this.memStoreByteHelper, DESC_INT_INT_TO_VOID));
        return;
      case IrOp.LOAD_WORD:
        this.emitRegSetFromStack(code, pool, registerIndex(instruction.operands[0]), () => {
          this.emitAddress(code, pool, instruction.operands[1], instruction.operands[2]);
          code.emitU1(OP_INVOKESTATIC);
          code.emitU2(pool.methodRef(this.internalClassName, this.loadWordHelper, DESC_INT_TO_INT));
        });
        return;
      case IrOp.STORE_WORD:
        this.emitAddress(code, pool, instruction.operands[1], instruction.operands[2]);
        this.emitRegGet(code, pool, registerIndex(instruction.operands[0]));
        code.emitU1(OP_INVOKESTATIC);
        code.emitU2(pool.methodRef(this.internalClassName, this.storeWordHelper, DESC_INT_INT_TO_VOID));
        return;
      default:
        throw new JvmBackendError(`Unsupported IR opcode ${IrOp[instruction.opcode]}`);
    }
  }

  private emitBinaryOp(
    code: BytecodeBuilder,
    pool: ConstantPoolBuilder,
    instruction: IrInstruction,
    opcode: number,
  ): void {
    this.emitRegSetFromStack(code, pool, registerIndex(instruction.operands[0]), () => {
      this.emitRegGet(code, pool, registerIndex(instruction.operands[1]));
      this.emitRegGet(code, pool, registerIndex(instruction.operands[2]));
      code.emitU1(opcode);
    });
  }

  private emitCompare(
    code: BytecodeBuilder,
    pool: ConstantPoolBuilder,
    instruction: IrInstruction,
    branchOpcode: number,
  ): void {
    const target = registerIndex(instruction.operands[0]);
    const yesLabel = this.freshLabel("cmp_yes");
    const doneLabel = this.freshLabel("cmp_done");
    this.emitRegGet(code, pool, registerIndex(instruction.operands[1]));
    this.emitRegGet(code, pool, registerIndex(instruction.operands[2]));
    code.emitBranch(branchOpcode, yesLabel);
    this.emitRegSet(code, pool, target, 0);
    code.emitBranch(OP_GOTO, doneLabel);
    code.mark(yesLabel);
    this.emitRegSet(code, pool, target, 1);
    code.mark(doneLabel);
  }

  private emitAddress(
    code: BytecodeBuilder,
    pool: ConstantPoolBuilder,
    base: IrOperand,
    offset: IrOperand,
  ): void {
    this.emitRegGet(code, pool, registerIndex(base));
    this.emitRegGet(code, pool, registerIndex(offset));
    code.emitU1(OP_IADD);
  }

  private emitRegGet(code: BytecodeBuilder, pool: ConstantPoolBuilder, register: number): void {
    code.emitPushInt(register, pool);
    code.emitU1(OP_INVOKESTATIC);
    code.emitU2(pool.methodRef(this.internalClassName, this.regGetHelper, DESC_INT_TO_INT));
  }

  private emitRegSet(code: BytecodeBuilder, pool: ConstantPoolBuilder, register: number, value: number): void {
    code.emitPushInt(register, pool);
    code.emitPushInt(value, pool);
    code.emitU1(OP_INVOKESTATIC);
    code.emitU2(pool.methodRef(this.internalClassName, this.regSetHelper, DESC_INT_INT_TO_VOID));
  }

  private emitRegSetFromStack(
    code: BytecodeBuilder,
    pool: ConstantPoolBuilder,
    register: number,
    buildValue: () => void,
  ): void {
    code.emitPushInt(register, pool);
    buildValue();
    code.emitU1(OP_INVOKESTATIC);
    code.emitU2(pool.methodRef(this.internalClassName, this.regSetHelper, DESC_INT_INT_TO_VOID));
  }

  private freshLabel(prefix: string): string {
    this.compareId += 1;
    return `__ca_${prefix}_${this.compareId}`;
  }
}

export function lowerIrToJvmClassFile(
  program: IrProgram,
  config: JvmBackendConfig,
): JVMClassArtifact {
  return new Lowerer(program, config).lower();
}

export function writeClassFile(artifact: JVMClassArtifact, outputDir: string): string {
  return writeClassFileWithStrategy(artifact, outputDir, "auto");
}

/**
 * How the secure writer walks from the classpath root down to the class file.
 *
 * - `"auto"`: descriptor-relative traversal (`openat`-style `dir_fd`) wherever
 *   the host Python supports it, i.e. every POSIX system; otherwise the
 *   path-check walk.  This is what {@link writeClassFile} uses.
 * - `"path-checks"`: always use the path-check walk — the only option on
 *   Windows, where `os.open()` cannot open a directory and `dir_fd` is
 *   unsupported.
 *
 * The strategy is an internal seam, deliberately not re-exported from the
 * package index.  It exists so the POSIX test runs can exercise the Windows
 * walk too; otherwise that code would only ever run on the Windows CI job.
 */
export type ClassFileWriteStrategy = "auto" | "path-checks";

export function writeClassFileWithStrategy(
  artifact: JVMClassArtifact,
  outputDir: string,
  strategy: ClassFileWriteStrategy,
): string {
  if (!JAVA_BINARY_NAME_RE.test(artifact.className)) {
    throw new JvmBackendError(`Class name "${artifact.className}" escapes the requested classpath root`);
  }
  const relativePath = validatedOutputRelativePath(artifact.classFilename);
  const root = resolve(outputDir);
  const payload = Buffer.from(artifact.classBytes).toString("base64");
  const result = spawnSync(
    "python3",
    ["-c", SECURE_CLASS_WRITER, root, relativePath, payload, strategy],
    { encoding: "utf8" },
  );
  if (result.error) {
    throw new JvmBackendError(`Failed to invoke secure class-file writer: ${result.error.message}`);
  }
  if (result.status !== 0) {
    const message = result.stderr.trim() || "secure class-file writer failed";
    throw new JvmBackendError(message);
  }
  return join(realpathSync.native(root), ...relativePath.split("/"));
}

function emitLoadIntLocal(code: BytecodeBuilder, index: number): void {
  code.emitU1(OP_ILOAD);
  code.emitU1(index);
}

function emitStoreIntLocal(code: BytecodeBuilder, index: number): void {
  code.emitU1(OP_ISTORE);
  code.emitU1(index);
}

function encodeClassFile(params: {
  readonly pool: ConstantPoolBuilder;
  readonly accessFlags: number;
  readonly internalClassName: string;
  readonly superClassName: string;
  readonly fields: readonly FieldSpec[];
  readonly methods: readonly MethodSpec[];
  readonly majorVersion: number;
  readonly minorVersion: number;
}): Uint8Array {
  const { pool } = params;
  const thisClassIndex = pool.classRef(params.internalClassName);
  const superClassIndex = pool.classRef(params.superClassName);
  const codeNameIndex = pool.utf8("Code");

  const fields = params.fields.map((field) =>
    concatBytes(
      u2(field.accessFlags),
      u2(pool.utf8(field.name)),
      u2(pool.utf8(field.descriptor)),
      u2(0),
    ));
  const methods = params.methods.map((method) => {
    const codeAttribute = concatBytes(
      u2(codeNameIndex),
      u4(12 + method.code.length),
      u2(method.maxStack),
      u2(method.maxLocals),
      u4(method.code.length),
      method.code,
      u2(0),
      u2(0),
    );
    return concatBytes(
      u2(method.accessFlags),
      u2(pool.utf8(method.name)),
      u2(pool.utf8(method.descriptor)),
      u2(1),
      codeAttribute,
    );
  });

  return concatBytes(
    u4(0xcafebabe),
    u2(params.minorVersion),
    u2(params.majorVersion),
    u2(pool.size()),
    pool.entriesBytes(),
    u2(params.accessFlags),
    u2(thisClassIndex),
    u2(superClassIndex),
    u2(0),
    u2(fields.length),
    ...fields,
    u2(methods.length),
    ...methods,
    u2(0),
  );
}

function collectCallableLabels(program: IrProgram): readonly string[] {
  const labels = new Set<string>([program.entryLabel]);
  for (const instruction of program.instructions) {
    if (instruction.opcode === IrOp.CALL) {
      labels.add(asLabel(instruction.operands[0]).name);
    }
  }
  return [...labels];
}

function splitRegions(program: IrProgram, callableLabels: readonly string[]): readonly CallableRegion[] {
  const callable = new Set(callableLabels);
  const regions: CallableRegion[] = [];
  let currentLabel: string | null = null;
  let currentInstructions: IrInstruction[] = [];
  for (const instruction of program.instructions) {
    if (instruction.opcode === IrOp.LABEL) {
      const label = asLabel(instruction.operands[0]).name;
      if (callable.has(label)) {
        if (currentLabel !== null) {
          regions.push({ label: currentLabel, instructions: Object.freeze(currentInstructions) });
        }
        currentLabel = label;
        currentInstructions = [instruction];
        continue;
      }
    }
    if (currentLabel !== null) {
      currentInstructions.push(instruction);
    }
  }
  if (currentLabel !== null) {
    regions.push({ label: currentLabel, instructions: Object.freeze(currentInstructions) });
  }
  return regions;
}

function layoutData(program: IrProgram): ReadonlyMap<string, number> {
  const offsets = new Map<string, number>();
  let offset = 0;
  for (const decl of program.data) {
    offsets.set(decl.label, offset);
    offset += decl.size;
  }
  return offsets;
}

function totalStaticData(program: IrProgram): number {
  return program.data.reduce((sum, decl) => sum + decl.size, 0);
}

function maxRegisterIndex(program: IrProgram): number {
  let max = 0;
  for (const instruction of program.instructions) {
    for (const operand of instruction.operands) {
      if (operand.kind === "register") {
        max = Math.max(max, operand.index);
      }
    }
  }
  return max;
}

function registerIndex(operand: IrOperand): number {
  if (operand.kind !== "register") {
    throw new JvmBackendError("Expected register operand");
  }
  return (operand as IrRegister).index;
}

function immediateValue(operand: IrOperand): number {
  if (operand.kind !== "immediate") {
    throw new JvmBackendError("Expected immediate operand");
  }
  return operand.value;
}

function asLabel(operand: IrOperand): IrLabel {
  if (operand.kind !== "label") {
    throw new JvmBackendError("Expected label operand");
  }
  return operand;
}

// The relative path handed to the Python writer is always "/"-separated,
// whatever the host OS.  The writer parses it as a PurePosixPath so that
// every directory component is visited (and checked) one at a time.
//
// An earlier version returned `join(...parts)`, which on Windows produced
// "demo\\Example.class".  PurePosixPath sees that as ONE component, so the
// writer would skip the checks on "demo" and hand the backslash straight to
// the Windows path parser.  For the same reason a component may not contain
// a backslash, a drive-letter colon, or a NUL: on Windows "..\\evil" or
// "C:evil" would escape the classpath root even though neither is literally
// "." or "..".
//
//   classFilename            result
//   ──────────────────────   ─────────────────────────────
//   "demo/Example.class"     "demo/Example.class"
//   "/demo//Example.class"   "demo/Example.class"   (empty parts dropped)
//   "../Example.class"       throws
//   "demo\\..\\x.class"      throws
//   "C:x.class"              throws
const UNSAFE_PATH_COMPONENT_RE = /[\\:\0]/;

function validatedOutputRelativePath(classFilename: string): string {
  const parts = classFilename.split("/").filter(Boolean);
  if (
    parts.length === 0
    || parts.some(
      (component) => component === "." || component === ".." || UNSAFE_PATH_COMPONENT_RE.test(component),
    )
  ) {
    throw new JvmBackendError("Resolved class-file path escapes the requested classpath root");
  }
  return parts.join("/");
}

// ── The secure class-file writer ──────────────────────────────────────────
//
// Node has no openat(2), so the actual write is delegated to a small Python
// program.  Its job is to place `<root>/<relative path>` on disk without ever
// being steered outside `root` by a symlink (or, on Windows, a junction —
// another kind of "reparse point") planted somewhere along the way.
//
// argv: root (absolute), relative path ("/"-separated), base64 payload,
// strategy ("auto" | "path-checks").
//
// Step 1, shared by both strategies: find the deepest component of `root`
// that already exists.  It must be a real directory — not a file, not a
// symlink, not a junction.  Everything above it is canonicalised with
// realpath (so e.g. macOS's /var -> /private/var is fine); everything below
// it is created by the writer, one component at a time.
//
//   root = /tmp/out/new/deeper     /tmp/out exists, new/ and deeper/ do not
//   canonical_root = /tmp/out      missing_parts = ["new", "deeper"]
//
// Step 2 depends on what the platform offers:
//
//   strategy          used on   how each directory is entered
//   ────────────────  ────────  ─────────────────────────────────────────────
//   descriptors       POSIX     os.open(child, O_DIRECTORY|O_NOFOLLOW,
//                               dir_fd=parent).  Race-free: once a directory
//                               is open, renaming or swapping it cannot
//                               redirect what happens inside it.
//   path checks       Windows   mkdir, then lstat; refuse a symlink, a
//                               junction, or a non-directory.  The file is
//                               written to an O_EXCL temp sibling and then
//                               os.replace()d over the target.
//
// Windows cannot do the descriptor walk from Python at all: os.open() on a
// directory fails with PermissionError [Errno 13] and os.supports_dir_fd
// does not contain os.open.  The path-check walk catches every link that is
// in place when the writer looks, but — unlike the descriptor walk — not a
// directory swapped for a junction between the lstat and the write.  Closing
// that window needs handle-relative NtCreateFile calls (see lessons.d:
// "A Windows directory handle does not make subsequent path-based writes
// safe from a directory swap"); the Rust and Python ports of this backend
// accept the same residual race on Windows.
//
// Why os.replace instead of opening the target with O_TRUNC?  The target
// may already exist (recompiling into the same classpath is normal).  If a
// symlink were swapped in at the target name after our lstat, opening it
// would follow the link and truncate whatever it points at.  os.replace
// (MoveFileExW on Windows, rename(2) on POSIX) replaces the directory entry
// itself and never follows a link at the destination.
const SECURE_CLASS_WRITER = `
import base64
import os
import pathlib
import secrets
import stat
import sys

root = pathlib.Path(sys.argv[1])
if not root.is_absolute():
    root = pathlib.Path.cwd() / root
relative_path = pathlib.PurePosixPath(sys.argv[2])
payload = base64.b64decode(sys.argv[3])
strategy = sys.argv[4] if len(sys.argv) > 4 else "auto"

# Windows marks symlinks AND junctions with this attribute; Python only
# reports symlinks through S_ISLNK, so a junction is caught by this bit alone.
# On POSIX st_file_attributes does not exist and the getattr yields 0.
FILE_ATTRIBUTE_REPARSE_POINT = 0x0400


def is_link_or_reparse_point(metadata):
    if stat.S_ISLNK(metadata.st_mode):
        return True
    return bool(getattr(metadata, "st_file_attributes", 0) & FILE_ATTRIBUTE_REPARSE_POINT)


def is_plain_directory(metadata):
    return stat.S_ISDIR(metadata.st_mode) and not is_link_or_reparse_point(metadata)


def is_plain_file(metadata):
    return stat.S_ISREG(metadata.st_mode) and not is_link_or_reparse_point(metadata)


def refuse_directory():
    raise RuntimeError("Refusing to write through symlinked or invalid directory")


def locate_existing_root():
    missing_parts = []
    current_root = root
    while True:
        try:
            metadata = os.lstat(current_root)
        except FileNotFoundError:
            parent = current_root.parent
            if parent == current_root:
                raise
            missing_parts.insert(0, current_root.name)
            current_root = parent
            continue
        if not is_plain_directory(metadata):
            refuse_directory()
        return pathlib.Path(os.path.realpath(current_root)), missing_parts


def write_via_directory_descriptors(canonical_root, directory_parts):
    open_directory_flags = os.O_RDONLY
    if hasattr(os, "O_DIRECTORY"):
        open_directory_flags |= os.O_DIRECTORY
    if hasattr(os, "O_NOFOLLOW"):
        open_directory_flags |= os.O_NOFOLLOW

    open_file_flags = os.O_WRONLY | os.O_CREAT | os.O_TRUNC
    if hasattr(os, "O_NOFOLLOW"):
        open_file_flags |= os.O_NOFOLLOW

    directory_fds = []
    try:
        current_fd = os.open(canonical_root, open_directory_flags)
        directory_fds.append(current_fd)
        for component in directory_parts:
            try:
                os.mkdir(component, dir_fd=current_fd)
            except FileExistsError:
                pass
            next_fd = os.open(component, open_directory_flags, dir_fd=current_fd)
            directory_fds.append(next_fd)
            current_fd = next_fd
        file_fd = os.open(relative_path.name, open_file_flags, 0o644, dir_fd=current_fd)
        with os.fdopen(file_fd, "wb", closefd=True) as handle:
            handle.write(payload)
    finally:
        for directory_fd in reversed(directory_fds):
            os.close(directory_fd)


def write_via_path_checks(canonical_root, directory_parts):
    current = canonical_root
    for component in directory_parts:
        current = current / component
        # mkdir first, then inspect: an existing symlink or junction makes
        # mkdir raise FileExistsError and is then rejected by the lstat.
        try:
            os.mkdir(current)
        except FileExistsError:
            pass
        if not is_plain_directory(os.lstat(current)):
            refuse_directory()

    target = current / relative_path.name
    try:
        existing = os.lstat(target)
    except FileNotFoundError:
        pass
    else:
        if not is_plain_file(existing):
            raise RuntimeError("Refusing to overwrite symlinked or invalid output file")

    temp_path = current / ("." + relative_path.name + "." + secrets.token_hex(8) + ".tmp")
    temp_flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    for optional_flag in ("O_BINARY", "O_NOINHERIT", "O_NOFOLLOW"):
        temp_flags |= getattr(os, optional_flag, 0)
    temp_fd = os.open(temp_path, temp_flags, 0o644)
    try:
        with os.fdopen(temp_fd, "wb", closefd=True) as handle:
            handle.write(payload)
        os.replace(temp_path, target)
    except BaseException:
        try:
            os.unlink(temp_path)
        except FileNotFoundError:
            pass
        raise


canonical_root, missing_parts = locate_existing_root()
directory_parts = missing_parts + list(relative_path.parts[:-1])
descriptors_supported = os.open in os.supports_dir_fd and os.mkdir in os.supports_dir_fd
if strategy == "auto" and descriptors_supported:
    write_via_directory_descriptors(canonical_root, directory_parts)
else:
    write_via_path_checks(canonical_root, directory_parts)
`;

function u1(value: number): Uint8Array {
  return Uint8Array.of(value & 0xff);
}

function u2(value: number): Uint8Array {
  return Uint8Array.of((value >>> 8) & 0xff, value & 0xff);
}

function u4(value: number): Uint8Array {
  return Uint8Array.of((value >>> 24) & 0xff, (value >>> 16) & 0xff, (value >>> 8) & 0xff, value & 0xff);
}

function concatBytes(...parts: readonly Uint8Array[]): Uint8Array {
  const size = parts.reduce((sum, part) => sum + part.length, 0);
  const result = new Uint8Array(size);
  let offset = 0;
  for (const part of parts) {
    result.set(part, offset);
    offset += part.length;
  }
  return result;
}
