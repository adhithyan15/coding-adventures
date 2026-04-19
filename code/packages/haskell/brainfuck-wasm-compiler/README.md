# brainfuck-wasm-compiler

Haskell `brainfuck-wasm-compiler` connects the local Brainfuck frontend, the
local compiler IR, the Haskell IR optimizer, the Haskell IR-to-Wasm lowerer,
the Wasm validator, and the Wasm encoder into one source-to-bytes package.
