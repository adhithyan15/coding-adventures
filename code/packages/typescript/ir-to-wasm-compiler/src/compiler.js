import { IrOp, } from "@coding-adventures/compiler-ir";
import { encodeSigned, encodeUnsigned } from "@coding-adventures/wasm-leb128";
import { getOpcodeByName } from "@coding-adventures/wasm-opcodes";
import { BlockType, ExternalKind, ValueType, WasmModule, makeFuncType, } from "@coding-adventures/wasm-types";
const LOOP_START_RE = /^loop_\d+_start$/;
const IF_ELSE_RE = /^if_\d+_else$/;
const FUNCTION_COMMENT_RE = /^function:\s*([A-Za-z_][A-Za-z0-9_]*)\((.*)\)$/;
const SYSCALL_WRITE = 1;
const SYSCALL_READ = 2;
const SYSCALL_EXIT = 10;
const SYSCALL_ARG0 = 4;
const WASI_MODULE = "wasi_snapshot_preview1";
const WASI_IOVEC_OFFSET = 0;
const WASI_COUNT_OFFSET = 8;
const WASI_BYTE_OFFSET = 12;
const WASI_SCRATCH_SIZE = 16;
const REG_SCRATCH = 1;
const REG_VAR_BASE = 2;
const MEMORY_OPS = new Set([
    IrOp.LOAD_ADDR,
    IrOp.LOAD_BYTE,
    IrOp.STORE_BYTE,
    IrOp.LOAD_WORD,
    IrOp.STORE_WORD,
]);
const OPCODE = {
    nop: getOpcodeByName("nop").opcode,
    block: getOpcodeByName("block").opcode,
    loop: getOpcodeByName("loop").opcode,
    if: getOpcodeByName("if").opcode,
    else: getOpcodeByName("else").opcode,
    end: getOpcodeByName("end").opcode,
    br: getOpcodeByName("br").opcode,
    br_if: getOpcodeByName("br_if").opcode,
    return: getOpcodeByName("return").opcode,
    call: getOpcodeByName("call").opcode,
    local_get: getOpcodeByName("local.get").opcode,
    local_set: getOpcodeByName("local.set").opcode,
    i32_load: getOpcodeByName("i32.load").opcode,
    i32_load8_u: getOpcodeByName("i32.load8_u").opcode,
    i32_store: getOpcodeByName("i32.store").opcode,
    i32_store8: getOpcodeByName("i32.store8").opcode,
    i32_const: getOpcodeByName("i32.const").opcode,
    i32_eqz: getOpcodeByName("i32.eqz").opcode,
    i32_eq: getOpcodeByName("i32.eq").opcode,
    i32_ne: getOpcodeByName("i32.ne").opcode,
    i32_lt_s: getOpcodeByName("i32.lt_s").opcode,
    i32_gt_s: getOpcodeByName("i32.gt_s").opcode,
    i32_add: getOpcodeByName("i32.add").opcode,
    i32_sub: getOpcodeByName("i32.sub").opcode,
    i32_and: getOpcodeByName("i32.and").opcode,
};
export class WasmLoweringError extends Error {
    constructor(message) {
        super(message);
        this.name = "WasmLoweringError";
    }
}
export class IrToWasmCompiler {
    compile(program, functionSignatures = []) {
        const signatures = inferFunctionSignaturesFromComments(program);
        for (const signature of functionSignatures) {
            signatures.set(signature.label, signature);
        }
        const functions = this.splitFunctions(program, signatures);
        const imports = this.collectWasiImports(program);
        const { typeIndices, types } = this.buildTypeTable(functions, imports);
        const dataOffsets = this.layoutData(program.data);
        const scratchBase = this.needsWasiScratch(program)
            ? alignUp(program.data.reduce((sum, decl) => sum + decl.size, 0), 4)
            : null;
        const module = new WasmModule();
        module.types.push(...types);
        module.imports.push(...imports.map((entry) => ({
            moduleName: WASI_MODULE,
            name: entry.name,
            kind: ExternalKind.FUNCTION,
            typeInfo: typeIndices.get(entry.typeKey),
        })));
        const functionIndexBase = imports.length;
        const functionIndices = new Map();
        functions.forEach((fn, index) => {
            functionIndices.set(fn.label, functionIndexBase + index);
            module.functions.push(typeIndices.get(fn.label));
        });
        let totalBytes = program.data.reduce((sum, decl) => sum + decl.size, 0);
        if (scratchBase !== null) {
            totalBytes = Math.max(totalBytes, scratchBase + WASI_SCRATCH_SIZE);
        }
        if (this.needsMemory(program) || scratchBase !== null) {
            const pageCount = totalBytes > 0 ? Math.max(1, Math.ceil(totalBytes / 65536)) : 1;
            const limits = { min: pageCount, max: null };
            const memoryType = { limits };
            module.memories.push(memoryType);
            module.exports.push({
                name: "memory",
                kind: ExternalKind.MEMORY,
                index: 0,
            });
            for (const decl of program.data) {
                module.data.push({
                    memoryIndex: 0,
                    offsetExpr: constExpr(dataOffsets.get(decl.label)),
                    data: new Uint8Array(decl.size).fill(decl.init & 0xff),
                });
            }
        }
        const wasiContext = {
            functionIndices: new Map(imports.map((entry, index) => [entry.syscallNumber, index])),
            scratchBase,
        };
        for (const fn of functions) {
            module.code.push(new FunctionLowerer({
                fn,
                signatures,
                functionIndices,
                dataOffsets,
                wasiContext,
            }).lower());
            if (fn.signature.exportName !== undefined) {
                module.exports.push({
                    name: fn.signature.exportName,
                    kind: ExternalKind.FUNCTION,
                    index: functionIndices.get(fn.label),
                });
            }
        }
        return module;
    }
    buildTypeTable(functions, imports) {
        const seenTypes = new Map();
        const typeIndices = new Map();
        const types = [];
        const rememberType = (key, funcType) => {
            const signatureKey = funcTypeKey(funcType);
            let index = seenTypes.get(signatureKey);
            if (index === undefined) {
                index = types.length;
                types.push(funcType);
                seenTypes.set(signatureKey, index);
            }
            typeIndices.set(key, index);
        };
        for (const entry of imports) {
            rememberType(entry.typeKey, entry.funcType);
        }
        for (const fn of functions) {
            rememberType(fn.label, makeFuncType(Array(fn.signature.paramCount).fill(ValueType.I32), [ValueType.I32]));
        }
        return { typeIndices, types };
    }
    layoutData(decls) {
        const offsets = new Map();
        let cursor = 0;
        for (const decl of decls) {
            offsets.set(decl.label, cursor);
            cursor += decl.size;
        }
        return offsets;
    }
    needsMemory(program) {
        if (program.data.length > 0)
            return true;
        return program.instructions.some((instruction) => MEMORY_OPS.has(instruction.opcode));
    }
    needsWasiScratch(program) {
        return program.instructions.some((instruction) => {
            if (instruction.opcode !== IrOp.SYSCALL || instruction.operands.length === 0) {
                return false;
            }
            const syscall = expectImmediate(instruction.operands[0], "SYSCALL number").value;
            return syscall === SYSCALL_WRITE || syscall === SYSCALL_READ;
        });
    }
    collectWasiImports(program) {
        const required = new Set();
        for (const instruction of program.instructions) {
            if (instruction.opcode !== IrOp.SYSCALL || instruction.operands.length === 0) {
                continue;
            }
            required.add(expectImmediate(instruction.operands[0], "SYSCALL number").value);
        }
        const ordered = [
            {
                syscallNumber: SYSCALL_WRITE,
                name: "fd_write",
                funcType: makeFuncType([ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32], [ValueType.I32]),
                typeKey: "wasi::fd_write",
            },
            {
                syscallNumber: SYSCALL_READ,
                name: "fd_read",
                funcType: makeFuncType([ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32], [ValueType.I32]),
                typeKey: "wasi::fd_read",
            },
            {
                syscallNumber: SYSCALL_EXIT,
                name: "proc_exit",
                funcType: makeFuncType([ValueType.I32], []),
                typeKey: "wasi::proc_exit",
            },
        ];
        const supported = new Set(ordered.map((entry) => entry.syscallNumber));
        const unsupported = Array.from(required).filter((value) => !supported.has(value)).sort((a, b) => a - b);
        if (unsupported.length > 0) {
            throw new WasmLoweringError(`unsupported SYSCALL number(s): ${unsupported.join(", ")}`);
        }
        return ordered.filter((entry) => required.has(entry.syscallNumber));
    }
    splitFunctions(program, signatures) {
        const functions = [];
        let startIndex = null;
        let startLabel = null;
        for (let index = 0; index < program.instructions.length; index++) {
            const labelName = functionLabelName(program.instructions[index]);
            if (labelName === null)
                continue;
            if (startLabel !== null && startIndex !== null) {
                functions.push(makeFunctionIr(startLabel, program.instructions.slice(startIndex, index), signatures));
            }
            startLabel = labelName;
            startIndex = index;
        }
        if (startLabel !== null && startIndex !== null) {
            functions.push(makeFunctionIr(startLabel, program.instructions.slice(startIndex), signatures));
        }
        return functions;
    }
}
class FunctionLowerer {
    options;
    paramCount;
    bytes = [];
    instructions;
    labelToIndex = new Map();
    constructor(options) {
        this.options = options;
        this.paramCount = options.fn.signature.paramCount;
        this.instructions = options.fn.instructions;
        for (let index = 0; index < this.instructions.length; index++) {
            const instruction = this.instructions[index];
            if (instruction.opcode !== IrOp.LABEL || instruction.operands.length === 0)
                continue;
            const operand = instruction.operands[0];
            if (operand.kind === "label") {
                this.labelToIndex.set(operand.name, index);
            }
        }
    }
    lower() {
        this.copyParamsIntoIrRegisters();
        this.emitRegion(1, this.instructions.length);
        this.emitOpcode(OPCODE.end);
        return {
            locals: Array(this.options.fn.maxReg + 1).fill(ValueType.I32),
            code: Uint8Array.from(this.bytes),
        };
    }
    copyParamsIntoIrRegisters() {
        for (let paramIndex = 0; paramIndex < this.paramCount; paramIndex++) {
            this.emitOpcode(OPCODE.local_get);
            this.emitU32(paramIndex);
            this.emitOpcode(OPCODE.local_set);
            this.emitU32(this.localIndex(REG_VAR_BASE + paramIndex));
        }
    }
    emitRegion(start, end) {
        let index = start;
        while (index < end) {
            const instruction = this.instructions[index];
            if (instruction.opcode === IrOp.COMMENT) {
                index += 1;
                continue;
            }
            const labelName = labelNameFromInstruction(instruction);
            if (labelName !== null && LOOP_START_RE.test(labelName)) {
                index = this.emitLoop(index);
                continue;
            }
            if ((instruction.opcode === IrOp.BRANCH_Z || instruction.opcode === IrOp.BRANCH_NZ) &&
                instruction.operands.length === 2 &&
                isLabel(instruction.operands[1]) &&
                IF_ELSE_RE.test(instruction.operands[1].name)) {
                index = this.emitIf(index);
                continue;
            }
            if (instruction.opcode === IrOp.LABEL) {
                index += 1;
                continue;
            }
            if (instruction.opcode === IrOp.JUMP ||
                instruction.opcode === IrOp.BRANCH_Z ||
                instruction.opcode === IrOp.BRANCH_NZ) {
                throw new WasmLoweringError(`unexpected unstructured control flow in ${this.options.fn.label}`);
            }
            this.emitSimple(instruction);
            index += 1;
        }
    }
    emitIf(branchIndex) {
        const branch = this.instructions[branchIndex];
        const condReg = expectRegister(branch.operands[0], "if condition");
        const elseLabel = expectLabel(branch.operands[1], "if else label").name;
        const endLabel = elseLabel.endsWith("_else")
            ? `${elseLabel.slice(0, -"_else".length)}_end`
            : `${elseLabel}_end`;
        const elseIndex = this.requireLabelIndex(elseLabel);
        const endIndex = this.requireLabelIndex(endLabel);
        const jumpIndex = this.findLastJumpToLabel(branchIndex + 1, elseIndex, endLabel);
        this.emitLocalGet(condReg.index);
        if (branch.opcode === IrOp.BRANCH_NZ) {
            this.emitOpcode(OPCODE.i32_eqz);
        }
        this.emitOpcode(OPCODE.if);
        this.emitByte(BlockType.EMPTY);
        this.emitRegion(branchIndex + 1, jumpIndex);
        if (elseIndex + 1 < endIndex) {
            this.emitOpcode(OPCODE.else);
            this.emitRegion(elseIndex + 1, endIndex);
        }
        this.emitOpcode(OPCODE.end);
        return endIndex + 1;
    }
    emitLoop(labelIndex) {
        const startLabel = labelNameFromInstruction(this.instructions[labelIndex]);
        if (startLabel === null) {
            throw new WasmLoweringError("loop lowering expected a start label");
        }
        const endLabel = startLabel.endsWith("_start")
            ? `${startLabel.slice(0, -"_start".length)}_end`
            : `${startLabel}_end`;
        const endIndex = this.requireLabelIndex(endLabel);
        const branchIndex = this.findFirstBranchToLabel(labelIndex + 1, endIndex, endLabel);
        const backedgeIndex = this.findLastJumpToLabel(branchIndex + 1, endIndex, startLabel);
        const branch = this.instructions[branchIndex];
        const condReg = expectRegister(branch.operands[0], "loop condition");
        this.emitOpcode(OPCODE.block);
        this.emitByte(BlockType.EMPTY);
        this.emitOpcode(OPCODE.loop);
        this.emitByte(BlockType.EMPTY);
        this.emitRegion(labelIndex + 1, branchIndex);
        this.emitLocalGet(condReg.index);
        if (branch.opcode === IrOp.BRANCH_Z) {
            this.emitOpcode(OPCODE.i32_eqz);
        }
        this.emitOpcode(OPCODE.br_if);
        this.emitU32(1);
        this.emitRegion(branchIndex + 1, backedgeIndex);
        this.emitOpcode(OPCODE.br);
        this.emitU32(0);
        this.emitOpcode(OPCODE.end);
        this.emitOpcode(OPCODE.end);
        return endIndex + 1;
    }
    emitSimple(instruction) {
        switch (instruction.opcode) {
            case IrOp.LOAD_IMM: {
                const dst = expectRegister(instruction.operands[0], "LOAD_IMM dst");
                const value = expectImmediate(instruction.operands[1], "LOAD_IMM imm");
                this.emitI32Const(value.value);
                this.emitLocalSet(dst.index);
                return;
            }
            case IrOp.LOAD_ADDR: {
                const dst = expectRegister(instruction.operands[0], "LOAD_ADDR dst");
                const label = expectLabel(instruction.operands[1], "LOAD_ADDR label");
                const offset = this.options.dataOffsets.get(label.name);
                if (offset === undefined) {
                    throw new WasmLoweringError(`unknown data label: ${label.name}`);
                }
                this.emitI32Const(offset);
                this.emitLocalSet(dst.index);
                return;
            }
            case IrOp.LOAD_BYTE: {
                const dst = expectRegister(instruction.operands[0], "LOAD_BYTE dst");
                const base = expectRegister(instruction.operands[1], "LOAD_BYTE base");
                const offset = expectRegister(instruction.operands[2], "LOAD_BYTE offset");
                this.emitAddress(base.index, offset.index);
                this.emitOpcode(OPCODE.i32_load8_u);
                this.emitMemarg(0, 0);
                this.emitLocalSet(dst.index);
                return;
            }
            case IrOp.STORE_BYTE: {
                const src = expectRegister(instruction.operands[0], "STORE_BYTE src");
                const base = expectRegister(instruction.operands[1], "STORE_BYTE base");
                const offset = expectRegister(instruction.operands[2], "STORE_BYTE offset");
                this.emitAddress(base.index, offset.index);
                this.emitLocalGet(src.index);
                this.emitOpcode(OPCODE.i32_store8);
                this.emitMemarg(0, 0);
                return;
            }
            case IrOp.LOAD_WORD: {
                const dst = expectRegister(instruction.operands[0], "LOAD_WORD dst");
                const base = expectRegister(instruction.operands[1], "LOAD_WORD base");
                const offset = expectRegister(instruction.operands[2], "LOAD_WORD offset");
                this.emitAddress(base.index, offset.index);
                this.emitOpcode(OPCODE.i32_load);
                this.emitMemarg(2, 0);
                this.emitLocalSet(dst.index);
                return;
            }
            case IrOp.STORE_WORD: {
                const src = expectRegister(instruction.operands[0], "STORE_WORD src");
                const base = expectRegister(instruction.operands[1], "STORE_WORD base");
                const offset = expectRegister(instruction.operands[2], "STORE_WORD offset");
                this.emitAddress(base.index, offset.index);
                this.emitLocalGet(src.index);
                this.emitOpcode(OPCODE.i32_store);
                this.emitMemarg(2, 0);
                return;
            }
            case IrOp.ADD:
                this.emitBinaryNumeric(OPCODE.i32_add, instruction);
                return;
            case IrOp.ADD_IMM: {
                const dst = expectRegister(instruction.operands[0], "ADD_IMM dst");
                const src = expectRegister(instruction.operands[1], "ADD_IMM src");
                const value = expectImmediate(instruction.operands[2], "ADD_IMM imm");
                this.emitLocalGet(src.index);
                this.emitI32Const(value.value);
                this.emitOpcode(OPCODE.i32_add);
                this.emitLocalSet(dst.index);
                return;
            }
            case IrOp.SUB:
                this.emitBinaryNumeric(OPCODE.i32_sub, instruction);
                return;
            case IrOp.AND:
                this.emitBinaryNumeric(OPCODE.i32_and, instruction);
                return;
            case IrOp.AND_IMM: {
                const dst = expectRegister(instruction.operands[0], "AND_IMM dst");
                const src = expectRegister(instruction.operands[1], "AND_IMM src");
                const value = expectImmediate(instruction.operands[2], "AND_IMM imm");
                this.emitLocalGet(src.index);
                this.emitI32Const(value.value);
                this.emitOpcode(OPCODE.i32_and);
                this.emitLocalSet(dst.index);
                return;
            }
            case IrOp.CMP_EQ:
                this.emitBinaryNumeric(OPCODE.i32_eq, instruction);
                return;
            case IrOp.CMP_NE:
                this.emitBinaryNumeric(OPCODE.i32_ne, instruction);
                return;
            case IrOp.CMP_LT:
                this.emitBinaryNumeric(OPCODE.i32_lt_s, instruction);
                return;
            case IrOp.CMP_GT:
                this.emitBinaryNumeric(OPCODE.i32_gt_s, instruction);
                return;
            case IrOp.CALL: {
                const label = expectLabel(instruction.operands[0], "CALL target");
                const signature = this.options.signatures.get(label.name);
                if (signature === undefined) {
                    throw new WasmLoweringError(`missing function signature for ${label.name}`);
                }
                const functionIndex = this.options.functionIndices.get(label.name);
                if (functionIndex === undefined) {
                    throw new WasmLoweringError(`unknown function label: ${label.name}`);
                }
                for (let paramIndex = 0; paramIndex < signature.paramCount; paramIndex++) {
                    this.emitLocalGet(REG_VAR_BASE + paramIndex);
                }
                this.emitOpcode(OPCODE.call);
                this.emitU32(functionIndex);
                this.emitLocalSet(REG_SCRATCH);
                return;
            }
            case IrOp.RET:
            case IrOp.HALT:
                this.emitLocalGet(REG_SCRATCH);
                this.emitOpcode(OPCODE.return);
                return;
            case IrOp.NOP:
                this.emitOpcode(OPCODE.nop);
                return;
            case IrOp.SYSCALL:
                this.emitSyscall(instruction);
                return;
            default:
                throw new WasmLoweringError(`unsupported opcode: ${IrOp[instruction.opcode]}`);
        }
    }
    emitSyscall(instruction) {
        const syscall = expectImmediate(instruction.operands[0], "SYSCALL number").value;
        switch (syscall) {
            case SYSCALL_WRITE:
                this.emitWasiWrite();
                return;
            case SYSCALL_READ:
                this.emitWasiRead();
                return;
            case SYSCALL_EXIT:
                this.emitWasiExit();
                return;
            default:
                throw new WasmLoweringError(`unsupported SYSCALL number: ${syscall}`);
        }
    }
    emitWasiWrite() {
        const scratchBase = this.requireWasiScratch();
        const iovecPtr = scratchBase + WASI_IOVEC_OFFSET;
        const nwrittenPtr = scratchBase + WASI_COUNT_OFFSET;
        const bytePtr = scratchBase + WASI_BYTE_OFFSET;
        this.emitI32Const(bytePtr);
        this.emitLocalGet(SYSCALL_ARG0);
        this.emitOpcode(OPCODE.i32_store8);
        this.emitMemarg(0, 0);
        this.emitStoreConstI32(iovecPtr, bytePtr);
        this.emitStoreConstI32(iovecPtr + 4, 1);
        this.emitI32Const(1);
        this.emitI32Const(iovecPtr);
        this.emitI32Const(1);
        this.emitI32Const(nwrittenPtr);
        this.emitWasiCall(SYSCALL_WRITE);
        this.emitLocalSet(REG_SCRATCH);
    }
    emitWasiRead() {
        const scratchBase = this.requireWasiScratch();
        const iovecPtr = scratchBase + WASI_IOVEC_OFFSET;
        const nreadPtr = scratchBase + WASI_COUNT_OFFSET;
        const bytePtr = scratchBase + WASI_BYTE_OFFSET;
        this.emitI32Const(bytePtr);
        this.emitI32Const(0);
        this.emitOpcode(OPCODE.i32_store8);
        this.emitMemarg(0, 0);
        this.emitStoreConstI32(iovecPtr, bytePtr);
        this.emitStoreConstI32(iovecPtr + 4, 1);
        this.emitI32Const(0);
        this.emitI32Const(iovecPtr);
        this.emitI32Const(1);
        this.emitI32Const(nreadPtr);
        this.emitWasiCall(SYSCALL_READ);
        this.emitLocalSet(REG_SCRATCH);
        this.emitI32Const(bytePtr);
        this.emitOpcode(OPCODE.i32_load8_u);
        this.emitMemarg(0, 0);
        this.emitLocalSet(SYSCALL_ARG0);
    }
    emitWasiExit() {
        this.emitLocalGet(SYSCALL_ARG0);
        this.emitWasiCall(SYSCALL_EXIT);
        this.emitI32Const(0);
        this.emitOpcode(OPCODE.return);
    }
    emitStoreConstI32(address, value) {
        this.emitI32Const(address);
        this.emitI32Const(value);
        this.emitOpcode(OPCODE.i32_store);
        this.emitMemarg(2, 0);
    }
    emitWasiCall(syscallNumber) {
        const functionIndex = this.options.wasiContext.functionIndices.get(syscallNumber);
        if (functionIndex === undefined) {
            throw new WasmLoweringError(`missing WASI import for SYSCALL ${syscallNumber}`);
        }
        this.emitOpcode(OPCODE.call);
        this.emitU32(functionIndex);
    }
    requireWasiScratch() {
        if (this.options.wasiContext.scratchBase === null) {
            throw new WasmLoweringError("SYSCALL lowering requires WASM scratch memory");
        }
        return this.options.wasiContext.scratchBase;
    }
    emitBinaryNumeric(opcode, instruction) {
        const name = IrOp[instruction.opcode];
        const dst = expectRegister(instruction.operands[0], `${name} dst`);
        const left = expectRegister(instruction.operands[1], `${name} lhs`);
        const right = expectRegister(instruction.operands[2], `${name} rhs`);
        this.emitLocalGet(left.index);
        this.emitLocalGet(right.index);
        this.emitOpcode(opcode);
        this.emitLocalSet(dst.index);
    }
    emitAddress(baseIndex, offsetIndex) {
        this.emitLocalGet(baseIndex);
        this.emitLocalGet(offsetIndex);
        this.emitOpcode(OPCODE.i32_add);
    }
    emitLocalGet(regIndex) {
        this.emitOpcode(OPCODE.local_get);
        this.emitU32(this.localIndex(regIndex));
    }
    emitLocalSet(regIndex) {
        this.emitOpcode(OPCODE.local_set);
        this.emitU32(this.localIndex(regIndex));
    }
    emitI32Const(value) {
        this.emitOpcode(OPCODE.i32_const);
        this.emitBytes(encodeSigned(value));
    }
    emitMemarg(align, offset) {
        this.emitU32(align);
        this.emitU32(offset);
    }
    emitOpcode(opcode) {
        this.bytes.push(opcode);
    }
    emitByte(value) {
        this.bytes.push(value);
    }
    emitU32(value) {
        this.emitBytes(encodeUnsigned(value));
    }
    emitBytes(bytes) {
        for (const byte of bytes) {
            this.bytes.push(byte);
        }
    }
    localIndex(regIndex) {
        return this.paramCount + regIndex;
    }
    requireLabelIndex(label) {
        const index = this.labelToIndex.get(label);
        if (index === undefined) {
            throw new WasmLoweringError(`missing label ${label} in ${this.options.fn.label}`);
        }
        return index;
    }
    findFirstBranchToLabel(start, end, label) {
        for (let index = start; index < end; index++) {
            const instruction = this.instructions[index];
            if (instruction.opcode !== IrOp.BRANCH_Z && instruction.opcode !== IrOp.BRANCH_NZ)
                continue;
            if (labelNameFromOperand(instruction.operands[1]) === label)
                return index;
        }
        throw new WasmLoweringError(`expected branch to ${label} in ${this.options.fn.label}`);
    }
    findLastJumpToLabel(start, end, label) {
        for (let index = end - 1; index >= start; index--) {
            const instruction = this.instructions[index];
            if (instruction.opcode !== IrOp.JUMP)
                continue;
            if (labelNameFromOperand(instruction.operands[0]) === label)
                return index;
        }
        throw new WasmLoweringError(`expected jump to ${label} in ${this.options.fn.label}`);
    }
}
export function inferFunctionSignaturesFromComments(program) {
    const signatures = new Map();
    let pendingComment = null;
    for (const instruction of program.instructions) {
        if (instruction.opcode === IrOp.COMMENT) {
            pendingComment = labelNameFromOperand(instruction.operands[0]);
            continue;
        }
        const labelName = functionLabelName(instruction);
        if (labelName !== null) {
            if (labelName === "_start") {
                signatures.set(labelName, {
                    label: labelName,
                    paramCount: 0,
                    exportName: "_start",
                });
            }
            else if (labelName.startsWith("_fn_") && pendingComment !== null) {
                const exportName = labelName.slice("_fn_".length);
                const match = FUNCTION_COMMENT_RE.exec(pendingComment);
                if (match && match[1] === exportName) {
                    const paramsBlob = match[2].trim();
                    const paramCount = paramsBlob === ""
                        ? 0
                        : paramsBlob.split(",").filter((piece) => piece.trim() !== "").length;
                    signatures.set(labelName, {
                        label: labelName,
                        paramCount,
                        exportName,
                    });
                }
            }
            pendingComment = null;
            continue;
        }
        pendingComment = null;
    }
    return signatures;
}
function makeFunctionIr(label, instructions, signatures) {
    let signature = signatures.get(label);
    if (label === "_start") {
        signature ??= { label, paramCount: 0, exportName: "_start" };
    }
    if (signature === undefined) {
        throw new WasmLoweringError(`missing function signature for ${label}`);
    }
    const registerIndices = instructions.flatMap((instruction) => instruction.operands.filter(isRegister).map((operand) => operand.index));
    const hasSyscall = instructions.some((instruction) => instruction.opcode === IrOp.SYSCALL);
    const maxReg = Math.max(1, REG_VAR_BASE + Math.max(signature.paramCount - 1, 0), ...(registerIndices.length > 0 ? registerIndices : [0]), hasSyscall ? SYSCALL_ARG0 : 0);
    return {
        label,
        instructions,
        signature,
        maxReg,
    };
}
function constExpr(value) {
    return new Uint8Array([OPCODE.i32_const, ...encodeSigned(value), OPCODE.end]);
}
function functionLabelName(instruction) {
    const label = labelNameFromInstruction(instruction);
    if (label === "_start" || (label !== null && label.startsWith("_fn_"))) {
        return label;
    }
    return null;
}
function labelNameFromInstruction(instruction) {
    if (instruction.opcode !== IrOp.LABEL || instruction.operands.length === 0)
        return null;
    const operand = instruction.operands[0];
    return operand.kind === "label" ? operand.name : null;
}
function labelNameFromOperand(operand) {
    if (operand.kind !== "label") {
        throw new WasmLoweringError(`expected label operand, got ${describeOperand(operand)}`);
    }
    return operand.name;
}
function expectRegister(operand, context) {
    if (operand.kind !== "register") {
        throw new WasmLoweringError(`${context}: expected register, got ${describeOperand(operand)}`);
    }
    return operand;
}
function expectImmediate(operand, context) {
    if (operand.kind !== "immediate") {
        throw new WasmLoweringError(`${context}: expected immediate, got ${describeOperand(operand)}`);
    }
    return operand;
}
function expectLabel(operand, context) {
    if (operand.kind !== "label") {
        throw new WasmLoweringError(`${context}: expected label, got ${describeOperand(operand)}`);
    }
    return operand;
}
function isRegister(operand) {
    return operand.kind === "register";
}
function isLabel(operand) {
    return operand.kind === "label";
}
function describeOperand(operand) {
    switch (operand.kind) {
        case "register":
            return `v${operand.index}`;
        case "immediate":
            return `${operand.value}`;
        case "label":
            return operand.name;
    }
}
function alignUp(value, alignment) {
    return Math.floor((value + alignment - 1) / alignment) * alignment;
}
function funcTypeKey(funcType) {
    return `${funcType.params.join(",")}=>${funcType.results.join(",")}`;
}
//# sourceMappingURL=compiler.js.map