cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../virtual-machine && pwd)/lib:$(cd ../wasm-leb128 && pwd)/lib:$(cd ../wasm-types && pwd)/lib:$(cd ../wasm-opcodes && pwd)/lib:$(cd ../wasm-module-parser && pwd)/lib:$(cd ../wasm-validator && pwd)/lib:${PERL5LIB:-} prove -l -v t/
