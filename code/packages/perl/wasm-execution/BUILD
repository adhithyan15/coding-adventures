cpanm --notest --quiet Test2::V0
cd ../virtual-machine && cpanm --installdeps --quiet .
cd ../wasm-leb128 && cpanm --installdeps --quiet .
cd ../wasm-types && cpanm --installdeps --quiet .
cd ../wasm-opcodes && cpanm --installdeps --quiet .
cd ../wasm-module-parser && cpanm --installdeps --quiet .
cpanm --installdeps --quiet .
PERL5LIB=$(cd ../virtual-machine && pwd)/lib:$(cd ../wasm-leb128 && pwd)/lib:$(cd ../wasm-types && pwd)/lib:$(cd ../wasm-opcodes && pwd)/lib:$(cd ../wasm-module-parser && pwd)/lib:${PERL5LIB:-} prove -l -v t/
