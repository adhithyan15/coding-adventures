package com.codingadventures.wasmsimulator;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.List;
import org.junit.jupiter.api.Test;

class WasmSimulatorTest {
    @Test
    void encodesAndDecodesI32Const() {
        byte[] encoded = WasmSimulator.encodeI32Const(42);
        WasmSimulator.WasmInstruction instruction = new WasmSimulator.WasmDecoder().decode(encoded, 0);

        assertEquals("i32.const", instruction.mnemonic());
        assertEquals(42, instruction.operand());
        assertEquals(5, instruction.size());
    }

    @Test
    void runsSimpleAdditionProgram() {
        WasmSimulator simulator = new WasmSimulator(2);
        byte[] program = WasmSimulator.assembleWasm(
                WasmSimulator.encodeI32Const(1),
                WasmSimulator.encodeI32Const(2),
                WasmSimulator.encodeI32Add(),
                WasmSimulator.encodeLocalSet(0),
                WasmSimulator.encodeEnd()
        );

        List<WasmSimulator.WasmStepTrace> traces = simulator.run(program);

        assertEquals(5, traces.size());
        assertEquals(List.of(3), traces.get(2).stackAfter());
        assertEquals(3, simulator.locals().get(0));
        assertEquals(List.of(), simulator.stack());
        assertEquals(true, simulator.halted());
    }

    @Test
    void localGetRestoresStoredValue() {
        WasmSimulator simulator = new WasmSimulator(2);
        byte[] program = WasmSimulator.assembleWasm(
                WasmSimulator.encodeI32Const(42),
                WasmSimulator.encodeLocalSet(1),
                WasmSimulator.encodeLocalGet(1),
                WasmSimulator.encodeEnd()
        );

        simulator.load(program);
        simulator.step();
        simulator.step();
        WasmSimulator.WasmStepTrace trace = simulator.step();

        assertEquals(List.of(42), trace.stackAfter());
        assertEquals(42, simulator.locals().get(1));
    }

    @Test
    void throwsWhenSteppingAfterHalt() {
        WasmSimulator simulator = new WasmSimulator(1);
        simulator.run(WasmSimulator.assembleWasm(WasmSimulator.encodeEnd()));

        assertThrows(IllegalStateException.class, simulator::step);
    }
}
