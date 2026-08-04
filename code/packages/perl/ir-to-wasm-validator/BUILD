cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../compiler-ir && pwd)/lib:$(cd ../ir-to-wasm-compiler && pwd)/lib:$(cd ../wasm-leb128 && pwd)/lib:$(cd ../wasm-opcodes && pwd)/lib:$(cd ../wasm-types && pwd)/lib:${PERL5LIB:-} prove -l -v t/
