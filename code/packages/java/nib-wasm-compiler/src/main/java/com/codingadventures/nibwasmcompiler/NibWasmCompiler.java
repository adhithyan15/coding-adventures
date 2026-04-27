package com.codingadventures.nibwasmcompiler;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

public final class NibWasmCompiler {
    public static final String VERSION = "0.1.0";
    private static final int MAX_SOURCE_LENGTH = 1_000_000;
    private static final int MAX_EXPR_NESTING = 256;
    private static final Pattern FUNCTION =
            Pattern.compile("fn\\s+([A-Za-z_][A-Za-z0-9_]*)\\s*\\(([^)]*)\\)\\s*->\\s*u4\\s*\\{\\s*return\\s+([^;]+);\\s*\\}", Pattern.DOTALL);

    private NibWasmCompiler() {}

    public record NibFunction(String name, List<String> params, String expression) {
        public NibFunction {
            params = List.copyOf(params);
            expression = expression.trim();
        }
    }

    public record PackageResult(String source, List<NibFunction> functions, byte[] wasmBytes, Path wasmPath) {
        public PackageResult {
            functions = List.copyOf(functions);
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
        List<NibFunction> functions = parse(source);
        validate(functions);
        return new PackageResult(source, functions, emitModule(functions), null);
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
        return new PackageResult(result.source(), result.functions(), result.wasmBytes(), path);
    }

    private static List<NibFunction> parse(String source) {
        if (source.length() > MAX_SOURCE_LENGTH) {
            throw new PackageError("parse", "source exceeds " + MAX_SOURCE_LENGTH + " characters");
        }
        List<NibFunction> functions = new ArrayList<>();
        Matcher matcher = FUNCTION.matcher(source);
        int cursor = 0;
        while (matcher.find()) {
            if (!source.substring(cursor, matcher.start()).trim().isEmpty()) {
                throw new PackageError("parse", "unexpected text before function");
            }
            functions.add(new NibFunction(matcher.group(1), parseParams(matcher.group(2)), matcher.group(3)));
            cursor = matcher.end();
        }
        if (!source.substring(cursor).trim().isEmpty() || functions.isEmpty()) {
            throw new PackageError("parse", "expected one or more Nib functions");
        }
        return functions;
    }

    private static List<String> parseParams(String text) {
        if (text.trim().isEmpty()) {
            return List.of();
        }
        List<String> params = new ArrayList<>();
        for (String piece : text.split(",")) {
            String[] parts = piece.trim().split("\\s*:\\s*");
            if (parts.length != 2 || !"u4".equals(parts[1]) || !parts[0].matches("[A-Za-z_][A-Za-z0-9_]*")) {
                throw new PackageError("parse", "parameters must be `name: u4`");
            }
            params.add(parts[0]);
        }
        return params;
    }

    private static void validate(List<NibFunction> functions) {
        Map<String, NibFunction> byName = new LinkedHashMap<>();
        for (NibFunction function : functions) {
            if (byName.put(function.name(), function) != null) {
                throw new PackageError("validate", "duplicate function `" + function.name() + "`");
            }
        }
        for (NibFunction function : functions) {
            emitExpr(new Section(), function.expression(), byName, paramMap(function), false, 0);
        }
    }

    private static byte[] emitModule(List<NibFunction> functions) {
        Section module = new Section();
        module.write(new byte[] {0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00});

        Section types = new Section();
        types.u32(functions.size());
        for (NibFunction function : functions) {
            types.funcType(function.params().size(), 1);
        }
        module.write(section(1, types.bytes()));

        Section functionSection = new Section();
        functionSection.u32(functions.size());
        for (int index = 0; index < functions.size(); index++) {
            functionSection.u32(index);
        }
        module.write(section(3, functionSection.bytes()));

        Section exports = new Section();
        exports.u32(functions.size());
        for (int index = 0; index < functions.size(); index++) {
            exports.export(functions.get(index).name(), 0x00, index);
        }
        module.write(section(7, exports.bytes()));

        Map<String, NibFunction> byName = new LinkedHashMap<>();
        for (NibFunction function : functions) {
            byName.put(function.name(), function);
        }
        Section code = new Section();
        code.u32(functions.size());
        for (NibFunction function : functions) {
            Section body = new Section();
            body.u32(0);
            emitExpr(body, function.expression(), byName, paramMap(function), true, 0);
            body.write(0x0b);
            byte[] bytes = body.bytes();
            code.u32(bytes.length);
            code.write(bytes);
        }
        module.write(section(10, code.bytes()));
        return module.bytes();
    }

    private static void emitExpr(
            Section out,
            String expression,
            Map<String, NibFunction> functions,
            Map<String, Integer> params,
            boolean emit,
            int depth
    ) {
        if (depth > MAX_EXPR_NESTING) {
            throw new PackageError("validate", "expression nesting exceeds " + MAX_EXPR_NESTING);
        }
        List<String> addParts = splitTopLevel(expression, "+%");
        if (addParts.size() > 1) {
            emitExpr(out, addParts.get(0), functions, params, emit, depth + 1);
            for (int index = 1; index < addParts.size(); index++) {
                emitExpr(out, addParts.get(index), functions, params, emit, depth + 1);
                if (emit) {
                    out.write(0x6a);
                    out.i32(15);
                    out.write(0x71);
                }
            }
            return;
        }
        String trimmed = expression.trim();
        if (trimmed.matches("\\d+")) {
            int value = Integer.parseInt(trimmed);
            if (value < 0 || value > 15) {
                throw new PackageError("validate", "u4 literal out of range: " + value);
            }
            if (emit) {
                out.i32(value);
            }
            return;
        }
        Matcher call = Pattern.compile("([A-Za-z_][A-Za-z0-9_]*)\\s*\\((.*)\\)").matcher(trimmed);
        if (call.matches()) {
            NibFunction target = functions.get(call.group(1));
            if (target == null) {
                throw new PackageError("validate", "unknown function `" + call.group(1) + "`");
            }
            List<String> args = splitArgs(call.group(2));
            if (args.size() != target.params().size()) {
                throw new PackageError("validate", "wrong arity for `" + target.name() + "`");
            }
            for (String arg : args) {
                emitExpr(out, arg, functions, params, emit, depth + 1);
            }
            if (emit) {
                out.write(0x10);
                out.u32(new ArrayList<>(functions.keySet()).indexOf(target.name()));
            }
            return;
        }
        Integer paramIndex = params.get(trimmed);
        if (paramIndex != null) {
            if (emit) {
                out.write(0x20);
                out.u32(paramIndex);
            }
            return;
        }
        throw new PackageError("validate", "unsupported expression `" + expression + "`");
    }

    private static Map<String, Integer> paramMap(NibFunction function) {
        Map<String, Integer> params = new LinkedHashMap<>();
        for (int index = 0; index < function.params().size(); index++) {
            params.put(function.params().get(index), index);
        }
        return params;
    }

    private static List<String> splitArgs(String text) {
        if (text.trim().isEmpty()) {
            return List.of();
        }
        return splitTopLevel(text, ",");
    }

    private static List<String> splitTopLevel(String text, String delimiter) {
        List<String> parts = new ArrayList<>();
        int depth = 0;
        int start = 0;
        for (int index = 0; index < text.length(); index++) {
            char ch = text.charAt(index);
            if (ch == '(') {
                depth++;
            } else if (ch == ')') {
                depth--;
            } else if (depth == 0 && text.startsWith(delimiter, index)) {
                parts.add(text.substring(start, index).trim());
                start = index + delimiter.length();
                index += delimiter.length() - 1;
            }
        }
        parts.add(text.substring(start).trim());
        return parts;
    }

    private static byte[] section(int id, byte[] payload) {
        Section out = new Section();
        out.write(id);
        out.u32(payload.length);
        out.write(payload);
        return out.bytes();
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
