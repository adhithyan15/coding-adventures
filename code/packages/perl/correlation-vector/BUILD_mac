cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../sha256 && pwd)/lib:$(cd ../grammar-tools && pwd)/lib:$(cd ../json-lexer && pwd)/lib:$(cd ../json-parser && pwd)/lib:$(cd ../json-value && pwd)/lib:$(cd ../json-serializer && pwd)/lib:${PERL5LIB:-} prove -l -v t/
