cd ../grammar-tools && cpanm --notest --quiet . && cd -
cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../json-lexer && pwd)/lib:$(cd ../json-parser && pwd)/lib:$(cd ../json-value && pwd)/lib:${PERL5LIB:-} prove -l -v t/
