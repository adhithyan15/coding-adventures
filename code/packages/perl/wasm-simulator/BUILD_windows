cd ../wasm-leb128 && cpanm --notest --quiet . && cd -
cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../wasm-types && pwd)/lib:$(cd ../wasm-module-parser && pwd)/lib:${PERL5LIB:-} prove -l -v t/
