package com.codingadventures.brainfuckwasmcompiler

import com.codingadventures.wasmruntime.WasmRuntime
import java.nio.file.Files
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class BrainfuckWasmCompilerTest {
    @Test
    fun compilesSourceIntoWasmBytes() {
        val result = BrainfuckWasmCompiler.compileSource("++>+")
        assertTrue(result.wasmBytes.size > 8)
        assertEquals(listOf('+', '+', '>', '+'), result.operations)
    }

    @Test
    fun lowersLoopsIntoAValidModule() {
        val result = BrainfuckWasmCompiler.compileSource("++[>+<-]")
        assertNotNull(WasmRuntime().load(result.wasmBytes))
    }

    @Test
    fun packSourceAliasesCompileSource() {
        assertEquals(
            BrainfuckWasmCompiler.compileSource("+").wasmBytes.size,
            BrainfuckWasmCompiler.packSource("+").wasmBytes.size,
        )
    }

    @Test
    fun writesWasmBytes() {
        val path = Files.createTempFile("kotlin-brainfuck", ".wasm")
        val result = BrainfuckWasmCompiler.writeWasmFile("+", path)
        assertEquals(path, result.wasmPath)
        assertTrue(Files.size(path) > 8)
    }

    @Test
    fun reportsMalformedLoops() {
        val error = assertFailsWith<PackageError> { BrainfuckWasmCompiler.compileSource("[") }
        assertEquals("parse", error.stage)
    }

    @Test
    fun rejectsExcessiveLoopNesting() {
        val source = "[".repeat(513) + "]".repeat(513)
        val error = assertFailsWith<PackageError> { BrainfuckWasmCompiler.compileSource(source) }
        assertEquals("parse", error.stage)
    }

    @Test
    fun runsTapeMutationThroughRuntime() {
        val result = BrainfuckWasmCompiler.compileSource("+")
        assertEquals(emptyList(), WasmRuntime().loadAndRun(result.wasmBytes, "_start", emptyList()))
    }
}
