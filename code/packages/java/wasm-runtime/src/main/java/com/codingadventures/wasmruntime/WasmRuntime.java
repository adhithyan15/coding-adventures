package com.codingadventures.wasmruntime;

import com.codingadventures.wasmexecution.WasmExecution;
import com.codingadventures.wasmexecution.WasmExecution.HostFunction;
import com.codingadventures.wasmexecution.WasmExecution.HostInterface;
import com.codingadventures.wasmexecution.WasmExecution.ImportedGlobal;
import com.codingadventures.wasmexecution.WasmExecution.LinearMemory;
import com.codingadventures.wasmexecution.WasmExecution.Table;
import com.codingadventures.wasmexecution.WasmExecution.TrapError;
import com.codingadventures.wasmexecution.WasmExecution.WasmExecutionEngine;
import com.codingadventures.wasmexecution.WasmExecution.WasmValue;
import com.codingadventures.wasmmoduleparser.WasmModuleParser;
import com.codingadventures.wasmtypes.WasmModule;
import com.codingadventures.wasmtypes.WasmTypes;
import com.codingadventures.wasmtypes.WasmTypes.Export;
import com.codingadventures.wasmtypes.WasmTypes.ExternalKind;
import com.codingadventures.wasmtypes.WasmTypes.FuncType;
import com.codingadventures.wasmtypes.WasmTypes.FunctionBody;
import com.codingadventures.wasmtypes.WasmTypes.GlobalType;
import com.codingadventures.wasmtypes.WasmTypes.MemoryType;
import com.codingadventures.wasmtypes.WasmTypes.ValueType;
import com.codingadventures.wasmvalidator.WasmValidator;
import com.codingadventures.wasmvalidator.WasmValidator.ValidatedModule;
import java.nio.charset.StandardCharsets;
import java.security.SecureRandom;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Consumer;

public final class WasmRuntime {
    public static final String VERSION = "0.1.0";

    private final WasmModuleParser parser = new WasmModuleParser();
    private final HostInterface host;

    public WasmRuntime() {
        this(null);
    }

    public WasmRuntime(HostInterface host) {
        this.host = host;
    }

    public WasmModule load(byte[] wasmBytes) {
        return parser.parse(wasmBytes);
    }

    public ValidatedModule validate(WasmModule module) {
        return WasmValidator.validate(module);
    }

    public WasmInstance instantiate(WasmModule module) {
        List<FuncType> funcTypes = new ArrayList<>();
        List<FunctionBody> funcBodies = new ArrayList<>();
        List<HostFunction> hostFunctions = new ArrayList<>();
        List<GlobalType> globalTypes = new ArrayList<>();
        List<WasmValue> globals = new ArrayList<>();
        List<Table> tables = new ArrayList<>();
        LinearMemory memory = null;

        for (WasmTypes.Import importEntry : module.imports) {
            switch (importEntry.kind()) {
                case FUNCTION -> {
                    int typeIndex = (Integer) importEntry.typeInfo();
                    funcTypes.add(module.types.get(typeIndex));
                    funcBodies.add(null);
                    hostFunctions.add(host == null ? null : host.resolveFunction(importEntry.moduleName(), importEntry.name()));
                }
                case MEMORY -> {
                    if (host != null) {
                        LinearMemory importedMemory = host.resolveMemory(importEntry.moduleName(), importEntry.name());
                        if (importedMemory != null) {
                            memory = importedMemory;
                        }
                    }
                }
                case TABLE -> {
                    if (host != null) {
                        Table importedTable = host.resolveTable(importEntry.moduleName(), importEntry.name());
                        if (importedTable != null) {
                            tables.add(importedTable);
                        }
                    }
                }
                case GLOBAL -> {
                    if (host != null) {
                        ImportedGlobal importedGlobal = host.resolveGlobal(importEntry.moduleName(), importEntry.name());
                        if (importedGlobal != null) {
                            globalTypes.add(importedGlobal.type());
                            globals.add(importedGlobal.value());
                        }
                    }
                }
            }
        }

        for (int index = 0; index < module.functions.size(); index++) {
            int typeIndex = module.functions.get(index);
            funcTypes.add(module.types.get(typeIndex));
            funcBodies.add(module.code.get(index));
            hostFunctions.add(null);
        }

        if (memory == null && !module.memories.isEmpty()) {
            MemoryType memoryType = module.memories.get(0);
            memory = new LinearMemory(memoryType.limits().min(), memoryType.limits().max());
        }

        for (WasmTypes.TableType tableType : module.tables) {
            tables.add(new Table(tableType.limits().min(), tableType.limits().max()));
        }

        for (WasmTypes.Global global : module.globals) {
            WasmValue value = WasmExecution.evaluateConstExpr(global.initExpr(), globals);
            globalTypes.add(global.globalType());
            globals.add(value);
        }

        if (memory != null) {
            for (WasmTypes.DataSegment segment : module.data) {
                int offset = ((Number) WasmExecution.evaluateConstExpr(segment.offsetExpr(), globals).value()).intValue();
                memory.storeBytes(offset, segment.data());
            }
        }

        for (WasmTypes.Element element : module.elements) {
            int offset = ((Number) WasmExecution.evaluateConstExpr(element.offsetExpr(), globals).value()).intValue();
            Table table = tables.get(element.tableIndex());
            for (int index = 0; index < element.functionIndices().size(); index++) {
                table.set(offset + index, element.functionIndices().get(index));
            }
        }

        Map<String, Export> exports = new LinkedHashMap<>();
        for (Export export : module.exports) {
            exports.put(export.name(), export);
        }

        WasmExecutionEngine engine = new WasmExecutionEngine(memory, tables, globals, globalTypes, funcTypes, funcBodies, hostFunctions);
        WasmInstance instance = new WasmInstance(memory, tables, globals, globalTypes, funcTypes, funcBodies, hostFunctions, exports, host, engine);

        if (host instanceof WasiStub wasiStub && memory != null) {
            wasiStub.setMemory(memory);
        }

        if (module.start != null) {
            engine.callFunction(module.start, List.of());
        }

        return instance;
    }

    public List<Object> call(WasmInstance instance, String exportName, List<Object> args) {
        Export export = instance.exports.get(exportName);
        if (export == null) {
            throw new TrapError("export \"" + exportName + "\" not found");
        }
        if (export.kind() != ExternalKind.FUNCTION) {
            throw new TrapError("export \"" + exportName + "\" is not a function");
        }

        FuncType funcType = instance.funcTypes.get(export.index());
        List<WasmValue> typedArgs = new ArrayList<>(args.size());
        for (int index = 0; index < args.size(); index++) {
            typedArgs.add(WasmExecution.coerceValue(args.get(index), funcType.params().get(index)));
        }

        List<WasmValue> results = instance.engine.callFunction(export.index(), typedArgs);
        List<Object> rawValues = new ArrayList<>(results.size());
        for (WasmValue result : results) {
            rawValues.add(WasmExecution.unwrapValue(result));
        }
        return rawValues;
    }

    public List<Object> loadAndRun(byte[] wasmBytes, String exportName, List<Object> args) {
        WasmModule module = load(wasmBytes);
        validate(module);
        WasmInstance instance = instantiate(module);
        return call(instance, exportName, args);
    }

    public static final class WasmInstance {
        public final LinearMemory memory;
        public final List<Table> tables;
        public final List<WasmValue> globals;
        public final List<GlobalType> globalTypes;
        public final List<FuncType> funcTypes;
        public final List<FunctionBody> funcBodies;
        public final List<HostFunction> hostFunctions;
        public final Map<String, Export> exports;
        public final HostInterface host;
        private final WasmExecutionEngine engine;

        private WasmInstance(
                LinearMemory memory,
                List<Table> tables,
                List<WasmValue> globals,
                List<GlobalType> globalTypes,
                List<FuncType> funcTypes,
                List<FunctionBody> funcBodies,
                List<HostFunction> hostFunctions,
                Map<String, Export> exports,
                HostInterface host,
                WasmExecutionEngine engine
        ) {
            this.memory = memory;
            this.tables = tables;
            this.globals = globals;
            this.globalTypes = globalTypes;
            this.funcTypes = funcTypes;
            this.funcBodies = funcBodies;
            this.hostFunctions = hostFunctions;
            this.exports = exports;
            this.host = host;
            this.engine = engine;
        }
    }

    public static final class ProcExitError extends RuntimeException {
        public final int exitCode;

        public ProcExitError(int exitCode) {
            super("proc_exit(" + exitCode + ")");
            this.exitCode = exitCode;
        }
    }

    public interface WasiStdin {
        Object read(int count);
    }

    public interface WasiClock {
        long realtimeNs();

        long monotonicNs();

        long resolutionNs(int clockId);
    }

    public interface WasiRandom {
        void fillBytes(byte[] buffer);
    }

    public static final class SystemClock implements WasiClock {
        @Override
        public long realtimeNs() {
            return System.currentTimeMillis() * 1_000_000L;
        }

        @Override
        public long monotonicNs() {
            return System.nanoTime();
        }

        @Override
        public long resolutionNs(int clockId) {
            return 1_000_000L;
        }
    }

    public static final class SystemRandom implements WasiRandom {
        private final SecureRandom random = new SecureRandom();

        @Override
        public void fillBytes(byte[] buffer) {
            random.nextBytes(buffer);
        }
    }

    public static final class WasiConfig {
        private final WasiStdin stdin;
        private final List<String> args;
        private final Map<String, String> env;
        private final Consumer<String> stdout;
        private final Consumer<String> stderr;
        private final WasiClock clock;
        private final WasiRandom random;

        public WasiConfig() {
            this(count -> new byte[0], List.of(), Map.of(), text -> {}, text -> {}, new SystemClock(), new SystemRandom());
        }

        public WasiConfig(
                WasiStdin stdin,
                List<String> args,
                Map<String, String> env,
                Consumer<String> stdout,
                Consumer<String> stderr,
                WasiClock clock,
                WasiRandom random
        ) {
            this.stdin = stdin;
            this.args = List.copyOf(args);
            this.env = Collections.unmodifiableMap(new LinkedHashMap<>(env));
            this.stdout = stdout;
            this.stderr = stderr;
            this.clock = clock;
            this.random = random;
        }
    }

    public static class WasiStub implements HostInterface {
        private static final int ENOSYS = 52;
        private static final int ESUCCESS = 0;
        private static final int EBADF = 8;
        private static final int EINVAL = 28;

        private final WasiStdin stdinCallback;
        private final Consumer<String> stdoutCallback;
        private final Consumer<String> stderrCallback;
        private final List<String> args;
        private final Map<String, String> env;
        private final WasiClock clock;
        private final WasiRandom random;
        private LinearMemory instanceMemory;

        public WasiStub() {
            this(new WasiConfig());
        }

        public WasiStub(WasiConfig config) {
            this.stdinCallback = config.stdin;
            this.stdoutCallback = config.stdout;
            this.stderrCallback = config.stderr;
            this.args = config.args;
            this.env = config.env;
            this.clock = config.clock;
            this.random = config.random;
        }

        public void setMemory(LinearMemory memory) {
            this.instanceMemory = memory;
        }

        @Override
        public HostFunction resolveFunction(String moduleName, String name) {
            if (!"wasi_snapshot_preview1".equals(moduleName)) {
                return null;
            }
            return switch (name) {
                case "fd_write" -> makeFdWrite();
                case "fd_read" -> makeFdRead();
                case "proc_exit" -> makeProcExit();
                case "args_sizes_get" -> makeArgsSizesGet();
                case "args_get" -> makeArgsGet();
                case "environ_sizes_get" -> makeEnvironSizesGet();
                case "environ_get" -> makeEnvironGet();
                case "clock_res_get" -> makeClockResGet();
                case "clock_time_get" -> makeClockTimeGet();
                case "random_get" -> makeRandomGet();
                case "sched_yield" -> makeSchedYield();
                default -> makeStub(name);
            };
        }

        private HostFunction makeFdWrite() {
            return new HostFunction() {
                @Override
                public FuncType type() {
                    return WasmTypes.makeFuncType(
                            List.of(ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32),
                            List.of(ValueType.I32)
                    );
                }

                @Override
                public List<WasmValue> call(List<WasmValue> callArgs) {
                    if (instanceMemory == null) {
                        return List.of(WasmExecution.i32(ENOSYS));
                    }

                    int fd = ((Number) callArgs.get(0).value()).intValue();
                    int iovsPtr = ((Number) callArgs.get(1).value()).intValue();
                    int iovsLen = ((Number) callArgs.get(2).value()).intValue();
                    int nwrittenPtr = ((Number) callArgs.get(3).value()).intValue();
                    int totalWritten = 0;

                    for (int i = 0; i < iovsLen; i++) {
                        int bufPtr = instanceMemory.loadI32(iovsPtr + i * 8);
                        int bufLen = instanceMemory.loadI32(iovsPtr + i * 8 + 4);
                        byte[] bytes = new byte[bufLen];
                        for (int j = 0; j < bufLen; j++) {
                            bytes[j] = (byte) instanceMemory.loadI32_8u(bufPtr + j);
                        }
                        String text = new String(bytes, StandardCharsets.UTF_8);
                        totalWritten += bufLen;
                        if (fd == 1) {
                            stdoutCallback.accept(text);
                        } else if (fd == 2) {
                            stderrCallback.accept(text);
                        }
                    }

                    instanceMemory.storeI32(nwrittenPtr, totalWritten);
                    return List.of(WasmExecution.i32(ESUCCESS));
                }
            };
        }

        private HostFunction makeFdRead() {
            return new HostFunction() {
                @Override
                public FuncType type() {
                    return WasmTypes.makeFuncType(
                            List.of(ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32),
                            List.of(ValueType.I32)
                    );
                }

                @Override
                public List<WasmValue> call(List<WasmValue> callArgs) {
                    if (instanceMemory == null) {
                        return List.of(WasmExecution.i32(ENOSYS));
                    }

                    int fd = ((Number) callArgs.get(0).value()).intValue();
                    if (fd != 0) {
                        return List.of(WasmExecution.i32(EBADF));
                    }

                    int iovsPtr = ((Number) callArgs.get(1).value()).intValue();
                    int iovsLen = ((Number) callArgs.get(2).value()).intValue();
                    int nreadPtr = ((Number) callArgs.get(3).value()).intValue();
                    int totalRead = 0;

                    for (int i = 0; i < iovsLen; i++) {
                        int bufPtr = instanceMemory.loadI32(iovsPtr + i * 8);
                        int bufLen = instanceMemory.loadI32(iovsPtr + i * 8 + 4);
                        byte[] chunk = normalizeInputChunk(stdinCallback.read(bufLen), bufLen);
                        for (int j = 0; j < chunk.length; j++) {
                            instanceMemory.storeI32_8(bufPtr + j, chunk[j]);
                        }
                        totalRead += chunk.length;
                        if (chunk.length < bufLen) {
                            break;
                        }
                    }

                    instanceMemory.storeI32(nreadPtr, totalRead);
                    return List.of(WasmExecution.i32(ESUCCESS));
                }
            };
        }

        private HostFunction makeProcExit() {
            return new HostFunction() {
                @Override
                public FuncType type() {
                    return WasmTypes.makeFuncType(List.of(ValueType.I32), List.of());
                }

                @Override
                public List<WasmValue> call(List<WasmValue> callArgs) {
                    throw new ProcExitError(((Number) callArgs.get(0).value()).intValue());
                }
            };
        }

        private HostFunction makeArgsSizesGet() {
            return new HostFunction() {
                @Override
                public FuncType type() {
                    return WasmTypes.makeFuncType(List.of(ValueType.I32, ValueType.I32), List.of(ValueType.I32));
                }

                @Override
                public List<WasmValue> call(List<WasmValue> callArgs) {
                    if (instanceMemory == null) {
                        return List.of(WasmExecution.i32(ENOSYS));
                    }
                    int argcPtr = ((Number) callArgs.get(0).value()).intValue();
                    int argvBufSizePtr = ((Number) callArgs.get(1).value()).intValue();
                    instanceMemory.storeI32(argcPtr, args.size());
                    int bufSize = 0;
                    for (String arg : args) {
                        bufSize += arg.getBytes(StandardCharsets.UTF_8).length + 1;
                    }
                    instanceMemory.storeI32(argvBufSizePtr, bufSize);
                    return List.of(WasmExecution.i32(ESUCCESS));
                }
            };
        }

        private HostFunction makeArgsGet() {
            return new HostFunction() {
                @Override
                public FuncType type() {
                    return WasmTypes.makeFuncType(List.of(ValueType.I32, ValueType.I32), List.of(ValueType.I32));
                }

                @Override
                public List<WasmValue> call(List<WasmValue> callArgs) {
                    if (instanceMemory == null) {
                        return List.of(WasmExecution.i32(ENOSYS));
                    }
                    int argvPtr = ((Number) callArgs.get(0).value()).intValue();
                    int argvBufPtr = ((Number) callArgs.get(1).value()).intValue();
                    int offset = argvBufPtr;
                    for (int i = 0; i < args.size(); i++) {
                        instanceMemory.storeI32(argvPtr + i * 4, offset);
                        byte[] encoded = args.get(i).getBytes(StandardCharsets.UTF_8);
                        for (byte b : encoded) {
                            instanceMemory.storeI32_8(offset++, b);
                        }
                        instanceMemory.storeI32_8(offset++, 0);
                    }
                    return List.of(WasmExecution.i32(ESUCCESS));
                }
            };
        }

        private HostFunction makeEnvironSizesGet() {
            return new HostFunction() {
                @Override
                public FuncType type() {
                    return WasmTypes.makeFuncType(List.of(ValueType.I32, ValueType.I32), List.of(ValueType.I32));
                }

                @Override
                public List<WasmValue> call(List<WasmValue> callArgs) {
                    if (instanceMemory == null) {
                        return List.of(WasmExecution.i32(ENOSYS));
                    }
                    int countPtr = ((Number) callArgs.get(0).value()).intValue();
                    int bufSizePtr = ((Number) callArgs.get(1).value()).intValue();
                    instanceMemory.storeI32(countPtr, env.size());
                    int bufSize = 0;
                    for (Map.Entry<String, String> entry : env.entrySet()) {
                        bufSize += (entry.getKey() + "=" + entry.getValue()).getBytes(StandardCharsets.UTF_8).length + 1;
                    }
                    instanceMemory.storeI32(bufSizePtr, bufSize);
                    return List.of(WasmExecution.i32(ESUCCESS));
                }
            };
        }

        private HostFunction makeEnvironGet() {
            return new HostFunction() {
                @Override
                public FuncType type() {
                    return WasmTypes.makeFuncType(List.of(ValueType.I32, ValueType.I32), List.of(ValueType.I32));
                }

                @Override
                public List<WasmValue> call(List<WasmValue> callArgs) {
                    if (instanceMemory == null) {
                        return List.of(WasmExecution.i32(ENOSYS));
                    }
                    int environPtr = ((Number) callArgs.get(0).value()).intValue();
                    int environBufPtr = ((Number) callArgs.get(1).value()).intValue();
                    int offset = environBufPtr;
                    int index = 0;
                    for (Map.Entry<String, String> entry : env.entrySet()) {
                        instanceMemory.storeI32(environPtr + index * 4, offset);
                        byte[] encoded = (entry.getKey() + "=" + entry.getValue()).getBytes(StandardCharsets.UTF_8);
                        for (byte b : encoded) {
                            instanceMemory.storeI32_8(offset++, b);
                        }
                        instanceMemory.storeI32_8(offset++, 0);
                        index++;
                    }
                    return List.of(WasmExecution.i32(ESUCCESS));
                }
            };
        }

        private HostFunction makeClockResGet() {
            return new HostFunction() {
                @Override
                public FuncType type() {
                    return WasmTypes.makeFuncType(List.of(ValueType.I32, ValueType.I32), List.of(ValueType.I32));
                }

                @Override
                public List<WasmValue> call(List<WasmValue> callArgs) {
                    if (instanceMemory == null) {
                        return List.of(WasmExecution.i32(ENOSYS));
                    }
                    int clockId = ((Number) callArgs.get(0).value()).intValue();
                    int resolutionPtr = ((Number) callArgs.get(1).value()).intValue();
                    instanceMemory.storeI64(resolutionPtr, clock.resolutionNs(clockId));
                    return List.of(WasmExecution.i32(ESUCCESS));
                }
            };
        }

        private HostFunction makeClockTimeGet() {
            return new HostFunction() {
                @Override
                public FuncType type() {
                    return WasmTypes.makeFuncType(List.of(ValueType.I32, ValueType.I64, ValueType.I32), List.of(ValueType.I32));
                }

                @Override
                public List<WasmValue> call(List<WasmValue> callArgs) {
                    if (instanceMemory == null) {
                        return List.of(WasmExecution.i32(ENOSYS));
                    }
                    int clockId = ((Number) callArgs.get(0).value()).intValue();
                    int timePtr = ((Number) callArgs.get(2).value()).intValue();
                    long timeNs;
                    switch (clockId) {
                        case 0 -> timeNs = clock.realtimeNs();
                        case 1, 2, 3 -> timeNs = clock.monotonicNs();
                        default -> {
                            return List.of(WasmExecution.i32(EINVAL));
                        }
                    }
                    instanceMemory.storeI64(timePtr, timeNs);
                    return List.of(WasmExecution.i32(ESUCCESS));
                }
            };
        }

        private HostFunction makeRandomGet() {
            return new HostFunction() {
                @Override
                public FuncType type() {
                    return WasmTypes.makeFuncType(List.of(ValueType.I32, ValueType.I32), List.of(ValueType.I32));
                }

                @Override
                public List<WasmValue> call(List<WasmValue> callArgs) {
                    if (instanceMemory == null) {
                        return List.of(WasmExecution.i32(ENOSYS));
                    }
                    int bufPtr = ((Number) callArgs.get(0).value()).intValue();
                    int bufLen = ((Number) callArgs.get(1).value()).intValue();
                    byte[] bytes = new byte[bufLen];
                    random.fillBytes(bytes);
                    instanceMemory.writeBytes(bufPtr, bytes);
                    return List.of(WasmExecution.i32(ESUCCESS));
                }
            };
        }

        private HostFunction makeSchedYield() {
            return new HostFunction() {
                @Override
                public FuncType type() {
                    return WasmTypes.makeFuncType(List.of(), List.of(ValueType.I32));
                }

                @Override
                public List<WasmValue> call(List<WasmValue> callArgs) {
                    return List.of(WasmExecution.i32(ESUCCESS));
                }
            };
        }

        private HostFunction makeStub(String name) {
            return new HostFunction() {
                @Override
                public FuncType type() {
                    return WasmTypes.makeFuncType(List.of(), List.of(ValueType.I32));
                }

                @Override
                public List<WasmValue> call(List<WasmValue> callArgs) {
                    return List.of(WasmExecution.i32(ENOSYS));
                }
            };
        }

        private static byte[] normalizeInputChunk(Object value, int maxLen) {
            if (value == null) {
                return new byte[0];
            }

            byte[] bytes;
            if (value instanceof byte[] rawBytes) {
                bytes = rawBytes;
            } else if (value instanceof String text) {
                bytes = text.getBytes(StandardCharsets.UTF_8);
            } else if (value instanceof List<?> list) {
                bytes = new byte[list.size()];
                for (int index = 0; index < list.size(); index++) {
                    Object item = list.get(index);
                    if (!(item instanceof Number number)) {
                        throw new IllegalArgumentException("stdin callback list must contain only numbers");
                    }
                    bytes[index] = (byte) number.intValue();
                }
            } else {
                throw new IllegalArgumentException("unsupported stdin callback value: " + value.getClass().getName());
            }

            return bytes.length <= maxLen ? bytes : Arrays.copyOf(bytes, maxLen);
        }
    }
}
