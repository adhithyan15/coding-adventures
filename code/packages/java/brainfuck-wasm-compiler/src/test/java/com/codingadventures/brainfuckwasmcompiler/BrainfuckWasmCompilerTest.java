package com.codingadventures.brainfuckwasmcompiler;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.codingadventures.wasmruntime.WasmRuntime;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;

class BrainfuckWasmCompilerTest {
    @Test
    void compilesSourceIntoWasmBytes() {
        BrainfuckWasmCompiler.PackageResult result = BrainfuckWasmCompiler.compileSource("++>+");
        assertTrue(result.wasmBytes().length > 8);
        assertEquals(List.of('+', '+', '>', '+'), result.operations());
    }

    @Test
    void lowersLoopsIntoAValidModule() {
        BrainfuckWasmCompiler.PackageResult result = BrainfuckWasmCompiler.compileSource("++[>+<-]");
        assertNotNull(new WasmRuntime().load(result.wasmBytes()));
    }

    @Test
    void packSourceAliasesCompileSource() {
        assertEquals(
                BrainfuckWasmCompiler.compileSource("+").wasmBytes().length,
                BrainfuckWasmCompiler.packSource("+").wasmBytes().length);
    }

    @Test
    void writesWasmBytes() throws Exception {
        Path path = Files.createTempFile("java-brainfuck", ".wasm");
        BrainfuckWasmCompiler.PackageResult result = BrainfuckWasmCompiler.writeWasmFile("+", path);
        assertEquals(path, result.wasmPath());
        assertTrue(Files.size(path) > 8);
    }

    @Test
    void reportsMalformedLoops() {
        BrainfuckWasmCompiler.PackageError error =
                assertThrows(BrainfuckWasmCompiler.PackageError.class, () -> BrainfuckWasmCompiler.compileSource("["));
        assertEquals("parse", error.stage());
    }

    @Test
    void rejectsExcessiveLoopNesting() {
        String source = "[".repeat(513) + "]".repeat(513);
        BrainfuckWasmCompiler.PackageError error =
                assertThrows(BrainfuckWasmCompiler.PackageError.class, () -> BrainfuckWasmCompiler.compileSource(source));
        assertEquals("parse", error.stage());
    }

    @Test
    void runsTapeMutationThroughRuntime() {
        BrainfuckWasmCompiler.PackageResult result = BrainfuckWasmCompiler.compileSource("+");
        assertEquals(List.of(), new WasmRuntime().loadAndRun(result.wasmBytes(), "_start", List.of()));
    }
}
