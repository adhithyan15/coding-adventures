package com.codingadventures.nibwasmcompiler

import com.codingadventures.wasmruntime.WasmRuntime
import java.nio.file.Files
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class NibWasmCompilerTest {
    @Test
    fun compilesSimpleFunction() {
        val result = NibWasmCompiler.compileSource("fn answer() -> u4 { return 7; }")
        assertTrue(result.wasmBytes.size > 8)
        assertEquals("answer", result.functions.first().name)
    }

    @Test
    fun aliasesPackSource() {
        assertEquals(
            NibWasmCompiler.compileSource("fn answer() -> u4 { return 7; }").wasmBytes.size,
            NibWasmCompiler.packSource("fn answer() -> u4 { return 7; }").wasmBytes.size,
        )
    }

    @Test
    fun runsAnswerThroughRuntime() {
        val result = NibWasmCompiler.compileSource("fn answer() -> u4 { return 7; }")
        assertEquals(listOf(7), WasmRuntime().loadAndRun(result.wasmBytes, "answer", emptyList()))
    }

    @Test
    fun runsWrappingAdditionThroughRuntime() {
        val source = "fn add(a: u4, b: u4) -> u4 { return a +% b; }"
        val result = NibWasmCompiler.compileSource(source)
        assertEquals(listOf(3), WasmRuntime().loadAndRun(result.wasmBytes, "add", listOf(14, 5)))
    }

    @Test
    fun writesWasmBytes() {
        val path = Files.createTempFile("kotlin-nib", ".wasm")
        val result = NibWasmCompiler.writeWasmFile("fn answer() -> u4 { return 7; }", path)
        assertEquals(path, result.wasmPath)
        assertTrue(Files.size(path) > 8)
    }

    @Test
    fun reportsInvalidNib() {
        val error = assertFailsWith<PackageError> { NibWasmCompiler.compileSource("fn bad() -> u4 { return 99; }") }
        assertEquals("validate", error.stage)
    }

    @Test
    fun rejectsExcessiveExpressionNesting() {
        var expression = "0"
        repeat(258) {
            expression = "id($expression)"
        }
        val source = "fn id(x: u4) -> u4 { return x; }\nfn main() -> u4 { return $expression; }"
        val error = assertFailsWith<PackageError> { NibWasmCompiler.compileSource(source) }
        assertEquals("validate", error.stage)
    }
}
