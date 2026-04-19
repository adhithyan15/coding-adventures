package com.codingadventures.brainfuckwasmcompiler;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public final class BrainfuckWasmCompiler {
    public static final String VERSION = "0.1.0";
    private static final int MAX_SOURCE_LENGTH = 1_000_000;
    private static final int MAX_LOOP_NESTING = 512;

    private BrainfuckWasmCompiler() {}

    private record ParsedProgram(List<Character> operations, Map<Integer, Integer> loopEnds) {
        private ParsedProgram {
            operations = List.copyOf(operations);
            loopEnds = Map.copyOf(loopEnds);
        }
    }

    public record PackageResult(String source, List<Character> operations, byte[] wasmBytes, Path wasmPath) {
        public PackageResult {
            operations = List.copyOf(operations);
            wasmBytes = wasmBytes.clone();
        }
    }

    public static final class PackageError extends RuntimeException {
        private final String stage;

        public PackageError(String stage, String message) {
            super(message);
            this.stage = stage;
        }

        public String stage() {
            return stage;
        }
    }

    public static PackageResult compileSource(String source) {
        ParsedProgram program = parse(source);
        return new PackageResult(source, program.operations(), emitModule(program), null);
    }

    public static PackageResult packSource(String source) {
        return compileSource(source);
    }

    public static PackageResult writeWasmFile(String source, Path path) {
        PackageResult result = compileSource(source);
        try {
            Files.write(path, result.wasmBytes());
        } catch (IOException error) {
            throw new PackageError("write", error.getMessage());
        }
        return new PackageResult(result.source(), result.operations(), result.wasmBytes(), path);
    }

    private static ParsedProgram parse(String source) {
        if (source.length() > MAX_SOURCE_LENGTH) {
            throw new PackageError("parse", "source exceeds " + MAX_SOURCE_LENGTH + " characters");
        }
        List<Character> ops = new ArrayList<>();
        ArrayDeque<Integer> stack = new ArrayDeque<>();
        Map<Integer, Integer> loopEnds = new HashMap<>();
        for (int index = 0; index < source.length(); index++) {
            char ch = source.charAt(index);
            if ("><+-.,[]".indexOf(ch) < 0) {
                continue;
            }
            int opIndex = ops.size();
            if (ch == '[') {
                stack.push(opIndex);
                if (stack.size() > MAX_LOOP_NESTING) {
                    throw new PackageError("parse", "loop nesting exceeds " + MAX_LOOP_NESTING);
                }
            } else if (ch == ']') {
                if (stack.isEmpty()) {
                    throw new PackageError("parse", "unmatched ] at byte " + index);
                }
                loopEnds.put(stack.pop(), opIndex);
            }
            ops.add(ch);
        }
        if (!stack.isEmpty()) {
            throw new PackageError("parse", "unmatched [");
        }
        return new ParsedProgram(ops, loopEnds);
    }

    private static byte[] emitModule(ParsedProgram program) {
        List<Character> operations = program.operations();
        boolean needsWrite = operations.contains('.');
        boolean needsRead = operations.contains(',');
        int importCount = (needsWrite ? 1 : 0) + (needsRead ? 1 : 0);
        int writeIndex = needsWrite ? 0 : -1;
        int readIndex = needsRead ? (needsWrite ? 1 : 0) : -1;
        int startTypeIndex = importCount;
        int startFunctionIndex = importCount;

        ByteArrayOutputStream module = new ByteArrayOutputStream();
        writeAll(module, new byte[] {0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00});

        Section types = new Section();
        types.u32(importCount + 1);
        if (needsWrite) {
            types.funcType(4, 1);
        }
        if (needsRead) {
            types.funcType(4, 1);
        }
        types.funcType(0, 0);
        module.writeBytes(section(1, types.bytes()));

        if (importCount > 0) {
            Section imports = new Section();
            imports.u32(importCount);
            if (needsWrite) {
                imports.importFunction("wasi_snapshot_preview1", "fd_write", writeIndex);
            }
            if (needsRead) {
                imports.importFunction("wasi_snapshot_preview1", "fd_read", readIndex);
            }
            module.writeBytes(section(2, imports.bytes()));
        }

        Section functions = new Section();
        functions.u32(1);
        functions.u32(startTypeIndex);
        module.writeBytes(section(3, functions.bytes()));

        Section memory = new Section();
        memory.u32(1);
        memory.write(0x00);
        memory.u32(1);
        module.writeBytes(section(5, memory.bytes()));

        Section exports = new Section();
        exports.u32(2);
        exports.export("_start", 0x00, startFunctionIndex);
        exports.export("memory", 0x02, 0);
        module.writeBytes(section(7, exports.bytes()));

        Section code = new Section();
        byte[] body = functionBody(program, writeIndex, readIndex);
        code.u32(1);
        code.u32(body.length);
        code.write(body);
        module.writeBytes(section(10, code.bytes()));
        return module.toByteArray();
    }

    private static byte[] functionBody(ParsedProgram program, int writeIndex, int readIndex) {
        Section body = new Section();
        body.u32(1);
        body.u32(3);
        body.write(0x7f);
        emitOps(body, program.operations(), program.loopEnds(), 0, program.operations().size(), writeIndex, readIndex);
        body.write(0x0b);
        return body.bytes();
    }

    private static int emitOps(
            Section out,
            List<Character> ops,
            Map<Integer, Integer> loopEnds,
            int start,
            int end,
            int writeIndex,
            int readIndex
    ) {
        int index = start;
        while (index < end) {
            char op = ops.get(index);
            switch (op) {
                case '>' -> addToLocal(out, 0, 1);
                case '<' -> addToLocal(out, 0, -1);
                case '+' -> mutateCell(out, 1);
                case '-' -> mutateCell(out, -1);
                case '.' -> emitWrite(out, writeIndex);
                case ',' -> emitRead(out, readIndex);
                case '[' -> {
                    int close = loopEnds.get(index);
                    out.write(0x02);
                    out.write(0x40);
                    out.write(0x03);
                    out.write(0x40);
                    loadCell(out);
                    out.write(0x45);
                    out.write(0x0d);
                    out.u32(1);
                    emitOps(out, ops, loopEnds, index + 1, close, writeIndex, readIndex);
                    out.write(0x0c);
                    out.u32(0);
                    out.write(0x0b);
                    out.write(0x0b);
                    index = close;
                }
                default -> {
                }
            }
            index++;
        }
        return index;
    }

    private static void loadCell(Section out) {
        out.write(0x20);
        out.u32(0);
        out.write(0x2d);
        out.u32(0);
        out.u32(0);
    }

    private static void mutateCell(Section out, int delta) {
        loadCell(out);
        out.write(0x41);
        out.s32(delta);
        out.write(0x6a);
        out.write(0x41);
        out.s32(255);
        out.write(0x71);
        out.write(0x21);
        out.u32(1);
        out.write(0x20);
        out.u32(0);
        out.write(0x20);
        out.u32(1);
        out.write(0x3a);
        out.u32(0);
        out.u32(0);
    }

    private static void addToLocal(Section out, int local, int delta) {
        out.write(0x20);
        out.u32(local);
        out.write(0x41);
        out.s32(delta);
        out.write(0x6a);
        out.write(0x21);
        out.u32(local);
    }

    private static void emitWrite(Section out, int writeIndex) {
        if (writeIndex < 0) {
            return;
        }
        loadCell(out);
        out.write(0x21);
        out.u32(1);
        storeByteConstAddress(out, 30012, 1);
        storeI32Const(out, 30000, 30012);
        storeI32Const(out, 30004, 1);
        out.i32(1);
        out.i32(30000);
        out.i32(1);
        out.i32(30008);
        out.write(0x10);
        out.u32(writeIndex);
        out.write(0x21);
        out.u32(2);
    }

    private static void emitRead(Section out, int readIndex) {
        if (readIndex < 0) {
            return;
        }
        storeByteConstAddress(out, 30012, 0);
        storeI32Const(out, 30000, 30012);
        storeI32Const(out, 30004, 1);
        out.i32(0);
        out.i32(30000);
        out.i32(1);
        out.i32(30008);
        out.write(0x10);
        out.u32(readIndex);
        out.write(0x21);
        out.u32(2);
        out.i32(30012);
        out.write(0x2d);
        out.u32(0);
        out.u32(0);
        out.write(0x21);
        out.u32(1);
        out.write(0x20);
        out.u32(0);
        out.write(0x20);
        out.u32(1);
        out.write(0x3a);
        out.u32(0);
        out.u32(0);
    }

    private static void storeByteConstAddress(Section out, int address, int local) {
        out.i32(address);
        out.write(0x20);
        out.u32(local);
        out.write(0x3a);
        out.u32(0);
        out.u32(0);
    }

    private static void storeI32Const(Section out, int address, int value) {
        out.i32(address);
        out.i32(value);
        out.write(0x36);
        out.u32(2);
        out.u32(0);
    }

    private static byte[] section(int id, byte[] payload) {
        Section out = new Section();
        out.write(id);
        out.u32(payload.length);
        out.write(payload);
        return out.bytes();
    }

    private static void writeAll(ByteArrayOutputStream out, byte[] bytes) {
        out.writeBytes(bytes);
    }

    private static final class Section {
        private final ByteArrayOutputStream out = new ByteArrayOutputStream();

        void write(int value) {
            out.write(value & 0xff);
        }

        void write(byte[] bytes) {
            out.writeBytes(bytes);
        }

        void i32(int value) {
            write(0x41);
            s32(value);
        }

        void u32(int value) {
            write(encodeUnsigned(value));
        }

        void s32(int value) {
            write(encodeSigned(value));
        }

        void funcType(int paramCount, int resultCount) {
            write(0x60);
            u32(paramCount);
            for (int index = 0; index < paramCount; index++) {
                write(0x7f);
            }
            u32(resultCount);
            for (int index = 0; index < resultCount; index++) {
                write(0x7f);
            }
        }

        void importFunction(String module, String name, int typeIndex) {
            name(module);
            name(name);
            write(0x00);
            u32(typeIndex);
        }

        void export(String name, int kind, int index) {
            name(name);
            write(kind);
            u32(index);
        }

        void name(String value) {
            byte[] bytes = value.getBytes(java.nio.charset.StandardCharsets.UTF_8);
            u32(bytes.length);
            write(bytes);
        }

        byte[] bytes() {
            return out.toByteArray();
        }
    }

    private static byte[] encodeUnsigned(int value) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        int remaining = value;
        do {
            int b = remaining & 0x7f;
            remaining >>>= 7;
            if (remaining != 0) {
                b |= 0x80;
            }
            out.write(b);
        } while (remaining != 0);
        return out.toByteArray();
    }

    private static byte[] encodeSigned(int value) {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        int remaining = value;
        boolean more;
        do {
            int b = remaining & 0x7f;
            remaining >>= 7;
            boolean signBit = (b & 0x40) != 0;
            more = !((remaining == 0 && !signBit) || (remaining == -1 && signBit));
            if (more) {
                b |= 0x80;
            }
            out.write(b);
        } while (more);
        return out.toByteArray();
    }
}
