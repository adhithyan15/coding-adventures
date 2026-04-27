rootProject.name = "brainfuck-wasm-compiler"

includeBuild("../wasm-leb128")
includeBuild("../wasm-types")
includeBuild("../wasm-opcodes")
includeBuild("../wasm-module-parser")
includeBuild("../wasm-validator")
includeBuild("../wasm-execution")
includeBuild("../wasm-runtime")
