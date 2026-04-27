package com.codingadventures.wasmsimulator

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class WasmSimulatorTest {
    @Test
    fun encodesAndDecodesI32Const() {
        val encoded = encodeI32Const(42)
        val instruction = WasmDecoder().decode(encoded, 0)

        assertEquals("i32.const", instruction.mnemonic)
        assertEquals(42, instruction.operand)
        assertEquals(5, instruction.size)
    }

    @Test
    fun runsSimpleAdditionProgram() {
        val simulator = WasmSimulator(2)
        val program =
            assembleWasm(
                listOf(
                    encodeI32Const(1),
                    encodeI32Const(2),
                    encodeI32Add(),
                    encodeLocalSet(0),
                    encodeEnd(),
                ),
            )

        val traces = simulator.run(program)

        assertEquals(5, traces.size)
        assertEquals(listOf(3), traces[2].stackAfter)
        assertEquals(3, simulator.locals[0])
        assertEquals(emptyList(), simulator.stack)
        assertEquals(true, simulator.halted)
    }

    @Test
    fun localGetRestoresStoredValue() {
        val simulator = WasmSimulator(2)
        val program =
            assembleWasm(
                listOf(
                    encodeI32Const(42),
                    encodeLocalSet(1),
                    encodeLocalGet(1),
                    encodeEnd(),
                ),
            )

        simulator.load(program)
        simulator.step()
        simulator.step()
        val trace = simulator.step()

        assertEquals(listOf(42), trace.stackAfter)
        assertEquals(42, simulator.locals[1])
    }

    @Test
    fun throwsWhenSteppingAfterHalt() {
        val simulator = WasmSimulator(1)
        simulator.run(assembleWasm(listOf(encodeEnd())))

        assertFailsWith<IllegalStateException> {
            simulator.step()
        }
    }
}
