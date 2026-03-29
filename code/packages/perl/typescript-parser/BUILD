cpanm --notest --quiet Test2::V0
cd ../grammar-tools && cpanm --notest --nodeps --quiet . && cd -
cd ../typescript-lexer && cpanm --notest --nodeps --quiet . && cd -
cpanm --notest --nodeps --quiet .
prove -l -v t/
