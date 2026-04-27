package com.codingadventures.wasmexecution;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.codingadventures.wasmtypes.WasmTypes;
import com.codingadventures.wasmtypes.WasmTypes.GlobalType;
import com.codingadventures.wasmtypes.WasmTypes.ValueType;
import java.io.ByteArrayOutputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import org.junit.jupiter.api.Test;

class WasmExecutionTest {
    @Test
    void exposesVersion() {
        assertEquals("0.1.0", WasmExecution.VERSION);
    }

    @Test
    void returnsConstant() {
        List<WasmExecution.WasmValue> result = run(List.of(), List.of(ValueType.I32), List.of(), bytes(i32Const(42)));
        assertEquals(42, result.get(0).value());
    }

    @Test
    void addsTwoArguments() {
        List<WasmExecution.WasmValue> result = run(
                List.of(ValueType.I32, ValueType.I32),
                List.of(ValueType.I32),
                List.of(),
                bytes(localGet(0), localGet(1), new byte[]{0x6A}),
                List.of(WasmExecution.i32(3), WasmExecution.i32(4))
        );
        assertEquals(7, result.get(0).value());
    }

    @Test
    void usesDeclaredLocalAndLocalTee() {
        List<WasmExecution.WasmValue> result = run(
                List.of(ValueType.I32),
                List.of(ValueType.I32),
                List.of(ValueType.I32),
                bytes(localGet(0), localTee(1), drop(), localGet(1))
                ,
                List.of(WasmExecution.i32(99))
        );
        assertEquals(99, result.get(0).value());
    }

    @Test
    void branchesThroughBlockLoopAndIf() {
        List<WasmExecution.WasmValue> result = run(
                List.of(),
                List.of(ValueType.I32),
                List.of(ValueType.I32),
                bytes(
                        i32Const(3),
                        localSet(0),
                        new byte[]{0x02, (byte) ValueType.I32.code()},
                        new byte[]{0x03, (byte) WasmTypes.BLOCK_TYPE_EMPTY},
                        localGet(0),
                        new byte[]{0x45},
                        new byte[]{0x04, (byte) WasmTypes.BLOCK_TYPE_EMPTY},
                        localGet(0),
                        br(2),
                        new byte[]{0x0B},
                        localGet(0),
                        i32Const(1),
                        new byte[]{0x6B},
                        localSet(0),
                        br(0),
                        new byte[]{0x0B},
                        i32Const(-1),
                        new byte[]{0x0B}
                )
        );
        assertEquals(0, result.get(0).value());
    }

    @Test
    void takesElseBranch() {
        List<WasmExecution.WasmValue> result = run(
                List.of(),
                List.of(ValueType.I32),
                List.of(),
                bytes(i32Const(0), new byte[]{0x04, (byte) ValueType.I32.code()}, i32Const(10), new byte[]{0x05}, i32Const(20), new byte[]{0x0B})
        );
        assertEquals(20, result.get(0).value());
    }

    @Test
    void returnExitsFunctionEarly() {
        List<WasmExecution.WasmValue> result = run(
                List.of(),
                List.of(ValueType.I32),
                List.of(),
                bytes(i32Const(42), new byte[]{0x0F}, i32Const(99))
        );
        assertEquals(42, result.get(0).value());
    }

    @Test
    void callsHostAndIndirectFunctions() {
        WasmTypes.FuncType funcType = WasmTypes.makeFuncType(List.of(ValueType.I32), List.of(ValueType.I32));
        WasmExecution.HostFunction hostFunction = new WasmExecution.HostFunction() {
            @Override
            public WasmTypes.FuncType type() {
                return funcType;
            }

            @Override
            public List<WasmExecution.WasmValue> call(List<WasmExecution.WasmValue> args) {
                return List.of(WasmExecution.i32(((Number) args.get(0).value()).intValue() * 2));
            }
        };

        WasmExecution.Table table = new WasmExecution.Table(1);
        table.set(0, 0);
        WasmExecution.WasmExecutionEngine engine = new WasmExecution.WasmExecutionEngine(
                null,
                List.of(table),
                new ArrayList<>(),
                new ArrayList<>(),
                List.of(funcType),
                Arrays.asList((WasmTypes.FunctionBody) null),
                List.of(hostFunction)
        );

        List<WasmExecution.WasmValue> direct = engine.callFunction(0, List.of(WasmExecution.i32(21)));
        assertEquals(42, direct.get(0).value());

        WasmExecution.WasmExecutionEngine caller = new WasmExecution.WasmExecutionEngine(
                null,
                List.of(table),
                new ArrayList<>(),
                new ArrayList<>(),
                List.of(funcType, WasmTypes.makeFuncType(List.of(), List.of(ValueType.I32))),
                Arrays.asList((WasmTypes.FunctionBody) null, body(List.of(), bytes(i32Const(21), i32Const(0), new byte[]{0x11, 0x00, 0x00}))),
                Arrays.asList(hostFunction, null)
        );

        List<WasmExecution.WasmValue> indirect = caller.callFunction(1, List.of());
        assertEquals(42, indirect.get(0).value());
    }

    @Test
    void updatesMutableGlobal() {
        ArrayList<WasmExecution.WasmValue> globals = new ArrayList<>(List.of(WasmExecution.i32(0)));
        ArrayList<GlobalType> globalTypes = new ArrayList<>(List.of(new GlobalType(ValueType.I32, true)));
        WasmExecution.WasmExecutionEngine engine = new WasmExecution.WasmExecutionEngine(
                null,
                List.of(),
                globals,
                globalTypes,
                List.of(WasmTypes.makeFuncType(List.of(), List.of())),
                List.of(body(List.of(), bytes(i32Const(99), globalSet(0)))),
                Arrays.asList((WasmExecution.HostFunction) null)
        );

        engine.callFunction(0, List.of());
        assertEquals(99, globals.get(0).value());
    }

    @Test
    void roundTripsLoadsStoresAndMemoryGrowth() {
        WasmExecution.LinearMemory memory = new WasmExecution.LinearMemory(1, 4);
        List<WasmExecution.WasmValue> result = runWithMemory(
                memory,
                List.of(),
                List.of(ValueType.I32),
                List.of(ValueType.I32, ValueType.I64, ValueType.F32, ValueType.F64, ValueType.I32, ValueType.I32),
                bytes(
                        i32Const(0),
                        i32Const(123),
                        memOp(0x36, 2, 8),
                        i32Const(0),
                        memOp(0x28, 2, 8),
                        localSet(0),
                        i32Const(16),
                        i64Const(0x1020_3040_5060_7080L),
                        memOp(0x37, 3, 0),
                        i32Const(16),
                        memOp(0x29, 3, 0),
                        localSet(1),
                        i32Const(32),
                        f32Const(3.25f),
                        memOp(0x38, 2, 0),
                        i32Const(32),
                        memOp(0x2A, 2, 0),
                        localSet(2),
                        i32Const(40),
                        f64Const(Math.PI),
                        memOp(0x39, 3, 0),
                        i32Const(40),
                        memOp(0x2B, 3, 0),
                        localSet(3),
                        i32Const(48),
                        i32Const(0xFF),
                        memOp(0x3A, 0, 0),
                        i32Const(48),
                        memOp(0x2C, 0, 0),
                        localSet(4),
                        i32Const(2),
                        new byte[]{0x40, 0x00},
                        localSet(5),
                        localGet(0),
                        localGet(1),
                        drop(),
                        localGet(2),
                        drop(),
                        localGet(3),
                        drop(),
                        localGet(4),
                        drop(),
                        localGet(5)
                )
        );

        assertEquals(1, result.get(0).value());
        assertEquals(3, memory.size());
        assertEquals(123, memory.loadI32(8));
        assertEquals(0x1020_3040_5060_7080L, memory.loadI64(16));
    }

    @Test
    void trapsWithoutMemoryAndOnUnknownFunction() {
        WasmExecution.WasmExecutionEngine noMemory = engine(List.of(), List.of(ValueType.I32), List.of(), bytes(i32Const(0), memOp(0x28, 2, 0)));
        assertThrows(WasmExecution.TrapError.class, () -> noMemory.callFunction(0, List.of()));

        WasmExecution.WasmExecutionEngine empty = new WasmExecution.WasmExecutionEngine(
                null,
                List.of(),
                List.of(),
                List.of(),
                List.of(),
                List.of(),
                List.of()
        );
        assertThrows(WasmExecution.TrapError.class, () -> empty.callFunction(99, List.of()));
    }

    @Test
    void trapsOnUnreachableAndIntegerDivideByZero() {
        WasmExecution.WasmExecutionEngine unreachable = engine(List.of(), List.of(), List.of(), new byte[]{0x00, 0x0B});
        assertThrows(WasmExecution.TrapError.class, () -> unreachable.callFunction(0, List.of()));

        WasmExecution.WasmExecutionEngine divideByZero = engine(
                List.of(),
                List.of(ValueType.I32),
                List.of(),
                bytes(i32Const(7), i32Const(0), new byte[]{0x6D})
        );
        assertThrows(WasmExecution.TrapError.class, () -> divideByZero.callFunction(0, List.of()));
    }

    @Test
    void comparesAndRotatesI32Values() {
        List<WasmExecution.WasmValue> result = run(
                List.of(),
                List.of(ValueType.I32, ValueType.I32, ValueType.I32),
                List.of(ValueType.I32, ValueType.I32, ValueType.I32),
                bytes(
                        i32Const(-1),
                        i32Const(0),
                        new byte[]{0x49},
                        localSet(0),
                        i32Const(0x80000001),
                        i32Const(1),
                        new byte[]{0x78},
                        localSet(1),
                        i32Const(0b10110011),
                        new byte[]{0x69},
                        localSet(2),
                        localGet(0),
                        localGet(1),
                        localGet(2)
                )
        );

        assertEquals(0, result.get(0).value());
        assertEquals(0xC0000000, result.get(1).value());
        assertEquals(5, result.get(2).value());
    }

    @Test
    void coversRemainingI32ArithmeticAndBitwiseOpcodes() {
        List<WasmExecution.WasmValue> result = run(
                List.of(),
                List.of(
                        ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32,
                        ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32,
                        ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32,
                        ValueType.I32, ValueType.I32
                ),
                List.of(
                        ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32,
                        ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32,
                        ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I32,
                        ValueType.I32, ValueType.I32
                ),
                bytes(
                        i32Const(7), i32Const(7), new byte[]{0x46}, localSet(0),
                        i32Const(1), i32Const(2), new byte[]{0x47}, localSet(1),
                        i32Const(-1), i32Const(0), new byte[]{0x48}, localSet(2),
                        i32Const(5), i32Const(3), new byte[]{0x4A}, localSet(3),
                        i32Const(-1), i32Const(0), new byte[]{0x4B}, localSet(4),
                        i32Const(3), i32Const(5), new byte[]{0x4C}, localSet(5),
                        i32Const(0), i32Const(-1), new byte[]{0x4D}, localSet(6),
                        i32Const(5), i32Const(3), new byte[]{0x4E}, localSet(7),
                        i32Const(-1), i32Const(0), new byte[]{0x4F}, localSet(8),
                        i32Const(1), new byte[]{0x67}, localSet(9),
                        i32Const(8), new byte[]{0x68}, localSet(10),
                        i32Const(6), i32Const(7), new byte[]{0x6C}, localSet(11),
                        i32Const(-1), i32Const(2), new byte[]{0x6E}, localSet(12),
                        i32Const(-7), i32Const(3), new byte[]{0x6F}, localSet(13),
                        i32Const(7), i32Const(3), new byte[]{0x70}, localSet(14),
                        i32Const(0xF0), i32Const(0x0F), new byte[]{0x72}, localSet(15),
                        i32Const(1), i32Const(4), new byte[]{0x74}, localSet(16),
                        localGet(0), localGet(1), localGet(2), localGet(3), localGet(4),
                        localGet(5), localGet(6), localGet(7), localGet(8), localGet(9),
                        localGet(10), localGet(11), localGet(12), localGet(13), localGet(14),
                        localGet(15), localGet(16)
                )
        );

        assertEquals(List.of(1, 1, 1, 1, 1, 1, 1, 1, 1, 31, 3, 42, 2147483647, -1, 1, 255, 16),
                result.stream().map(WasmExecution.WasmValue::value).toList());
    }

    @Test
    void coversRemainingMemoryOpcodesAndMemorySize() {
        WasmExecution.LinearMemory memory = new WasmExecution.LinearMemory(1, 4);
        List<WasmExecution.WasmValue> result = runWithMemory(
                memory,
                List.of(),
                List.of(
                        ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I64, ValueType.I64,
                        ValueType.I64, ValueType.I64, ValueType.I64, ValueType.I32
                ),
                List.of(
                        ValueType.I32, ValueType.I32, ValueType.I32, ValueType.I64, ValueType.I64,
                        ValueType.I64, ValueType.I64, ValueType.I64, ValueType.I32
                ),
                bytes(
                        i32Const(0), i32Const(0xFFFF), memOp(0x3B, 1, 0),
                        i32Const(0), memOp(0x2E, 1, 0), localSet(0),
                        i32Const(0), memOp(0x2F, 1, 0), localSet(1),
                        i32Const(4), i32Const(0xFF), memOp(0x3A, 0, 0),
                        i32Const(4), memOp(0x2D, 0, 0), localSet(2),
                        i32Const(8), i32Const(-1), memOp(0x36, 2, 0),
                        i32Const(8), memOp(0x34, 2, 0), localSet(3),
                        i32Const(8), memOp(0x35, 2, 0), localSet(4),
                        i32Const(4), memOp(0x30, 0, 0), localSet(5),
                        i32Const(0), memOp(0x33, 1, 0), localSet(6),
                        i32Const(12), i64Const(0x1FFFFFFFFL), memOp(0x3E, 2, 0),
                        i32Const(12), memOp(0x35, 2, 0), localSet(7),
                        new byte[]{0x3F, 0x00}, localSet(8),
                        localGet(0), localGet(1), localGet(2), localGet(3), localGet(4),
                        localGet(5), localGet(6), localGet(7), localGet(8)
                )
        );

        assertEquals(List.of(-1, 65535, 255, -1L, 4294967295L, -1L, 65535L, 4294967295L, 1),
                result.stream().map(WasmExecution.WasmValue::value).toList());
    }

    @Test
    void trapsOnEngineEdgeCases() {
        WasmExecution.WasmExecutionEngine immutableGlobal = new WasmExecution.WasmExecutionEngine(
                null,
                List.of(),
                new ArrayList<>(List.of(WasmExecution.i32(0))),
                new ArrayList<>(List.of(new GlobalType(ValueType.I32, false))),
                List.of(WasmTypes.makeFuncType(List.of(), List.of())),
                List.of(body(List.of(), bytes(i32Const(1), globalSet(0)))),
                Arrays.asList((WasmExecution.HostFunction) null)
        );
        assertThrows(WasmExecution.TrapError.class, () -> immutableGlobal.callFunction(0, List.of()));

        WasmExecution.WasmExecutionEngine badBranch = engine(List.of(), List.of(), List.of(), bytes(br(0)));
        assertThrows(WasmExecution.TrapError.class, () -> badBranch.callFunction(0, List.of()));

        WasmExecution.WasmExecutionEngine unexpectedElse = engine(List.of(), List.of(), List.of(), new byte[]{0x05, 0x0B});
        assertThrows(WasmExecution.TrapError.class, () -> unexpectedElse.callFunction(0, List.of()));

        WasmExecution.WasmExecutionEngine invalidMemoryImmediate = runMemoryEngine(bytes(new byte[]{0x3F, 0x01}));
        assertThrows(WasmExecution.TrapError.class, () -> invalidMemoryImmediate.callFunction(0, List.of()));

        WasmExecution.WasmExecutionEngine operandUnderflow = engine(List.of(), List.of(), List.of(), bytes(drop()));
        assertThrows(WasmExecution.TrapError.class, () -> operandUnderflow.callFunction(0, List.of()));
    }

    @Test
    void trapsOnIndirectCallErrors() {
        WasmTypes.FuncType unary = WasmTypes.makeFuncType(List.of(ValueType.I32), List.of(ValueType.I32));
        WasmTypes.FuncType nullary = WasmTypes.makeFuncType(List.of(), List.of(ValueType.I32));

        WasmExecution.HostFunction host = new WasmExecution.HostFunction() {
            @Override
            public WasmTypes.FuncType type() {
                return unary;
            }

            @Override
            public List<WasmExecution.WasmValue> call(List<WasmExecution.WasmValue> args) {
                return List.of(WasmExecution.i32(1));
            }
        };

        WasmExecution.Table table = new WasmExecution.Table(1);
        table.set(0, 0);

        WasmExecution.WasmExecutionEngine mismatch = new WasmExecution.WasmExecutionEngine(
                null,
                List.of(table),
                new ArrayList<>(),
                new ArrayList<>(),
                List.of(unary, nullary, WasmTypes.makeFuncType(List.of(), List.of(ValueType.I32))),
                Arrays.asList((WasmTypes.FunctionBody) null, null, body(List.of(), bytes(i32Const(0), new byte[]{0x11, 0x01, 0x00}))),
                Arrays.asList(host, null, null)
        );
        assertThrows(WasmExecution.TrapError.class, () -> mismatch.callFunction(2, List.of()));

        WasmExecution.Table emptyTable = new WasmExecution.Table(1);
        WasmExecution.WasmExecutionEngine uninitialized = new WasmExecution.WasmExecutionEngine(
                null,
                List.of(emptyTable),
                new ArrayList<>(),
                new ArrayList<>(),
                List.of(unary, WasmTypes.makeFuncType(List.of(), List.of(ValueType.I32))),
                Arrays.asList((WasmTypes.FunctionBody) null, body(List.of(), bytes(i32Const(0), new byte[]{0x11, 0x00, 0x00}))),
                Arrays.asList(host, null)
        );
        assertThrows(WasmExecution.TrapError.class, () -> uninitialized.callFunction(1, List.of()));
    }

    @Test
    void coversHostInterfaceDefaultsAndImportedGlobalRecord() {
        WasmExecution.HostInterface hostInterface = new WasmExecution.HostInterface() {
        };
        assertEquals(null, hostInterface.resolveFunction("env", "f"));
        assertEquals(null, hostInterface.resolveGlobal("env", "g"));
        assertEquals(null, hostInterface.resolveMemory("env", "m"));
        assertEquals(null, hostInterface.resolveTable("env", "t"));

        GlobalType globalType = new GlobalType(ValueType.I32, true);
        WasmExecution.ImportedGlobal imported = new WasmExecution.ImportedGlobal(globalType, WasmExecution.i32(7));
        assertEquals(globalType, imported.type());
        assertEquals(WasmExecution.i32(7), imported.value());
    }

    private static List<WasmExecution.WasmValue> run(
            List<ValueType> params,
            List<ValueType> results,
            List<ValueType> locals,
            byte[] code
    ) {
        return run(params, results, locals, code, List.of());
    }

    private static List<WasmExecution.WasmValue> run(
            List<ValueType> params,
            List<ValueType> results,
            List<ValueType> locals,
            byte[] code,
            List<WasmExecution.WasmValue> args
    ) {
        return engine(params, results, locals, code).callFunction(0, args);
    }

    private static List<WasmExecution.WasmValue> runWithMemory(
            WasmExecution.LinearMemory memory,
            List<ValueType> params,
            List<ValueType> results,
            List<ValueType> locals,
            byte[] code
    ) {
        return new WasmExecution.WasmExecutionEngine(
                memory,
                List.of(),
                new ArrayList<>(),
                new ArrayList<>(),
                List.of(WasmTypes.makeFuncType(params, results)),
                List.of(body(locals, code)),
                Arrays.asList((WasmExecution.HostFunction) null)
        ).callFunction(0, List.of());
    }

    private static WasmExecution.WasmExecutionEngine engine(
            List<ValueType> params,
            List<ValueType> results,
            List<ValueType> locals,
            byte[] code
    ) {
        return new WasmExecution.WasmExecutionEngine(
                null,
                List.of(),
                new ArrayList<>(),
                new ArrayList<>(),
                List.of(WasmTypes.makeFuncType(params, results)),
                List.of(body(locals, code)),
                Arrays.asList((WasmExecution.HostFunction) null)
        );
    }

    private static WasmExecution.WasmExecutionEngine runMemoryEngine(byte[] code) {
        return new WasmExecution.WasmExecutionEngine(
                new WasmExecution.LinearMemory(1, 1),
                List.of(),
                new ArrayList<>(),
                new ArrayList<>(),
                List.of(WasmTypes.makeFuncType(List.of(), List.of())),
                List.of(body(List.of(), code)),
                Arrays.asList((WasmExecution.HostFunction) null)
        );
    }

    private static WasmTypes.FunctionBody body(List<ValueType> locals, byte[] code) {
        return new WasmTypes.FunctionBody(locals, concat(code, new byte[]{0x0B}));
    }

    private static byte[] bytes(byte[]... parts) {
        return concat(parts);
    }

    private static byte[] i32Const(int value) {
        return concat(new byte[]{0x41}, WasmExecutionTestSupport.encodeSigned32(value));
    }

    private static byte[] i64Const(long value) {
        return concat(new byte[]{0x42}, WasmExecutionTestSupport.encodeSigned64(value));
    }

    private static byte[] f32Const(float value) {
        return concat(new byte[]{0x43}, ByteBuffer.allocate(4).order(ByteOrder.LITTLE_ENDIAN).putFloat(value).array());
    }

    private static byte[] f64Const(double value) {
        return concat(new byte[]{0x44}, ByteBuffer.allocate(8).order(ByteOrder.LITTLE_ENDIAN).putDouble(value).array());
    }

    private static byte[] localGet(int index) {
        return concat(new byte[]{0x20}, WasmExecutionTestSupport.encodeUnsigned(index));
    }

    private static byte[] localSet(int index) {
        return concat(new byte[]{0x21}, WasmExecutionTestSupport.encodeUnsigned(index));
    }

    private static byte[] localTee(int index) {
        return concat(new byte[]{0x22}, WasmExecutionTestSupport.encodeUnsigned(index));
    }

    private static byte[] globalSet(int index) {
        return concat(new byte[]{0x24}, WasmExecutionTestSupport.encodeUnsigned(index));
    }

    private static byte[] br(int depth) {
        return concat(new byte[]{0x0C}, WasmExecutionTestSupport.encodeUnsigned(depth));
    }

    private static byte[] memOp(int opcode, int align, int offset) {
        return concat(new byte[]{(byte) opcode}, WasmExecutionTestSupport.encodeUnsigned(align), WasmExecutionTestSupport.encodeUnsigned(offset));
    }

    private static byte[] drop() {
        return new byte[]{0x1A};
    }

    private static byte[] concat(byte[]... parts) {
        ByteArrayOutputStream output = new ByteArrayOutputStream();
        for (byte[] part : parts) {
            output.writeBytes(part);
        }
        return output.toByteArray();
    }
}
