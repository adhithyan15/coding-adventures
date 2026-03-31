cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../grammar-tools && pwd)/lib:$(cd ../json-lexer && pwd)/lib:$(cd ../json-parser && pwd)/lib:${PERL5LIB:-} prove -l -v t/
