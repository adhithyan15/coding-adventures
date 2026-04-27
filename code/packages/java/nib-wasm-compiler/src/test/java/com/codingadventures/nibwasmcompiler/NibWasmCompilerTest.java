package com.codingadventures.nibwasmcompiler;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.codingadventures.wasmruntime.WasmRuntime;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import org.junit.jupiter.api.Test;

class NibWasmCompilerTest {
    @Test
    void compilesSimpleFunction() {
        NibWasmCompiler.PackageResult result = NibWasmCompiler.compileSource("fn answer() -> u4 { return 7; }");
        assertTrue(result.wasmBytes().length > 8);
        assertEquals("answer", result.functions().get(0).name());
    }

    @Test
    void aliasesPackSource() {
        assertEquals(
                NibWasmCompiler.compileSource("fn answer() -> u4 { return 7; }").wasmBytes().length,
                NibWasmCompiler.packSource("fn answer() -> u4 { return 7; }").wasmBytes().length);
    }

    @Test
    void runsAnswerThroughRuntime() {
        NibWasmCompiler.PackageResult result = NibWasmCompiler.compileSource("fn answer() -> u4 { return 7; }");
        assertEquals(List.of(7), new WasmRuntime().loadAndRun(result.wasmBytes(), "answer", List.of()));
    }

    @Test
    void runsWrappingAdditionThroughRuntime() {
        String source = "fn add(a: u4, b: u4) -> u4 { return a +% b; }";
        NibWasmCompiler.PackageResult result = NibWasmCompiler.compileSource(source);
        assertEquals(List.of(3), new WasmRuntime().loadAndRun(result.wasmBytes(), "add", List.of(14, 5)));
    }

    @Test
    void writesWasmBytes() throws Exception {
        Path path = Files.createTempFile("java-nib", ".wasm");
        NibWasmCompiler.PackageResult result = NibWasmCompiler.writeWasmFile("fn answer() -> u4 { return 7; }", path);
        assertEquals(path, result.wasmPath());
        assertTrue(Files.size(path) > 8);
    }

    @Test
    void reportsInvalidNib() {
        NibWasmCompiler.PackageError error =
                assertThrows(NibWasmCompiler.PackageError.class, () -> NibWasmCompiler.compileSource("fn bad() -> u4 { return 99; }"));
        assertEquals("validate", error.stage());
    }

    @Test
    void rejectsExcessiveExpressionNesting() {
        String expression = "0";
        for (int index = 0; index < 258; index++) {
            expression = "id(" + expression + ")";
        }
        String source = "fn id(x: u4) -> u4 { return x; }\nfn main() -> u4 { return " + expression + "; }";
        NibWasmCompiler.PackageError error =
                assertThrows(NibWasmCompiler.PackageError.class, () -> NibWasmCompiler.compileSource(source));
        assertEquals("validate", error.stage());
    }
}
