package com.codingadventures.wasmsimulator;

import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;

public final class WasmSimulator {
    public static final String VERSION = "0.1.0";

    public static final int OP_END = 0x0B;
    public static final int OP_LOCAL_GET = 0x20;
    public static final int OP_LOCAL_SET = 0x21;
    public static final int OP_I32_CONST = 0x41;
    public static final int OP_I32_ADD = 0x6A;
    public static final int OP_I32_SUB = 0x6B;

    private final int[] locals;
    private final ArrayDeque<Integer> stack = new ArrayDeque<>();
    private byte[] bytecode = new byte[0];
    private int pc = 0;
    private int cycle = 0;
    private boolean halted = false;

    public WasmSimulator(int localCount) {
        this.locals = new int[localCount];
    }

    public record WasmInstruction(int opcode, String mnemonic, Integer operand, int size) {}

    public record WasmStepTrace(
            int pc,
            WasmInstruction instruction,
            List<Integer> stackBefore,
            List<Integer> stackAfter,
            List<Integer> localsSnapshot,
            String description,
            boolean halted
    ) {}

    public static final class WasmDecoder {
        public WasmInstruction decode(byte[] bytecode, int pc) {
            int opcode = Byte.toUnsignedInt(bytecode[pc]);
            return switch (opcode) {
                case OP_I32_CONST -> new WasmInstruction(
                        opcode,
                        "i32.const",
                        ByteBuffer.wrap(bytecode, pc + 1, 4).order(ByteOrder.LITTLE_ENDIAN).getInt(),
                        5
                );
                case OP_I32_ADD -> new WasmInstruction(opcode, "i32.add", null, 1);
                case OP_I32_SUB -> new WasmInstruction(opcode, "i32.sub", null, 1);
                case OP_LOCAL_GET -> new WasmInstruction(opcode, "local.get", Byte.toUnsignedInt(bytecode[pc + 1]), 2);
                case OP_LOCAL_SET -> new WasmInstruction(opcode, "local.set", Byte.toUnsignedInt(bytecode[pc + 1]), 2);
                case OP_END -> new WasmInstruction(opcode, "end", null, 1);
                default -> throw new IllegalArgumentException(
                        "Unknown WASM opcode: 0x" + Integer.toHexString(opcode).toUpperCase() + " at PC=" + pc
                );
            };
        }
    }

    public void load(byte[] program) {
        this.bytecode = program.clone();
        this.pc = 0;
        this.cycle = 0;
        this.halted = false;
        this.stack.clear();
        java.util.Arrays.fill(this.locals, 0);
    }

    public WasmStepTrace step() {
        if (halted) {
            throw new IllegalStateException("simulator is halted");
        }

        WasmInstruction instruction = new WasmDecoder().decode(bytecode, pc);
        List<Integer> stackBefore = snapshotStack();
        String description;

        switch (instruction.opcode()) {
            case OP_I32_CONST -> {
                stack.push(instruction.operand());
                description = "push " + instruction.operand();
            }
            case OP_I32_ADD -> {
                int right = pop();
                int left = pop();
                int result = left + right;
                stack.push(result);
                description = "pop " + right + " and " + left + ", push " + result;
            }
            case OP_I32_SUB -> {
                int right = pop();
                int left = pop();
                int result = left - right;
                stack.push(result);
                description = "pop " + right + " and " + left + ", push " + result;
            }
            case OP_LOCAL_GET -> {
                int index = instruction.operand();
                stack.push(locals[index]);
                description = "push local[" + index + "]";
            }
            case OP_LOCAL_SET -> {
                int index = instruction.operand();
                locals[index] = pop();
                description = "store into local[" + index + "]";
            }
            case OP_END -> {
                halted = true;
                description = "halt";
            }
            default -> throw new IllegalStateException("Unsupported opcode " + instruction.opcode());
        }

        int currentPc = pc;
        pc += instruction.size();
        cycle += 1;
        return new WasmStepTrace(
                currentPc,
                instruction,
                stackBefore,
                snapshotStack(),
                snapshotLocals(),
                description,
                halted
        );
    }

    public List<WasmStepTrace> run(byte[] program) {
        load(program);
        List<WasmStepTrace> traces = new ArrayList<>();
        while (!halted) {
            traces.add(step());
        }
        return traces;
    }

    public List<Integer> stack() {
        return List.copyOf(stack);
    }

    public List<Integer> locals() {
        return snapshotLocals();
    }

    public boolean halted() {
        return halted;
    }

    public int pc() {
        return pc;
    }

    public int cycle() {
        return cycle;
    }

    public void reset() {
        this.bytecode = new byte[0];
        this.pc = 0;
        this.cycle = 0;
        this.halted = false;
        this.stack.clear();
        java.util.Arrays.fill(this.locals, 0);
    }

    public static byte[] encodeI32Const(int value) {
        ByteBuffer buffer = ByteBuffer.allocate(5).order(ByteOrder.LITTLE_ENDIAN);
        buffer.put((byte) OP_I32_CONST);
        buffer.putInt(value);
        return buffer.array();
    }

    public static byte[] encodeI32Add() {
        return new byte[]{(byte) OP_I32_ADD};
    }

    public static byte[] encodeI32Sub() {
        return new byte[]{(byte) OP_I32_SUB};
    }

    public static byte[] encodeLocalGet(int index) {
        return new byte[]{(byte) OP_LOCAL_GET, (byte) index};
    }

    public static byte[] encodeLocalSet(int index) {
        return new byte[]{(byte) OP_LOCAL_SET, (byte) index};
    }

    public static byte[] encodeEnd() {
        return new byte[]{(byte) OP_END};
    }

    public static byte[] assembleWasm(byte[]... instructions) {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        for (byte[] instruction : instructions) {
            output.writeBytes(instruction);
        }
        return output.toByteArray();
    }

    private int pop() {
        Integer value = stack.pollFirst();
        if (value == null) {
            throw new IllegalStateException("stack underflow");
        }
        return value;
    }

    private List<Integer> snapshotLocals() {
        List<Integer> snapshot = new ArrayList<>(locals.length);
        for (int value : locals) {
            snapshot.add(value);
        }
        return snapshot;
    }

    private List<Integer> snapshotStack() {
        List<Integer> snapshot = new ArrayList<>(stack.size());
        var iterator = stack.descendingIterator();
        while (iterator.hasNext()) {
            Integer value = iterator.next();
            snapshot.add(value);
        }
        return snapshot;
    }
}
