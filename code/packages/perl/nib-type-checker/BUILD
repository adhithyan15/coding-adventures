cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../grammar-tools && pwd)/lib:$(cd ../lexer && pwd)/lib:$(cd ../nib-lexer && pwd)/lib:$(cd ../parser && pwd)/lib:$(cd ../nib-parser && pwd)/lib:$(cd ../type-checker-protocol && pwd)/lib:${PERL5LIB:-} prove -l -v t/
