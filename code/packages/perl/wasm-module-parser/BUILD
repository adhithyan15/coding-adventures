cpanm --notest --quiet Test2::V0
cd ../wasm-leb128 && cpanm --notest --nodeps --quiet . && cd -
cd ../wasm-types && cpanm --notest --nodeps --quiet . && cd -
cpanm --notest --nodeps --quiet .
prove -l -v t/
