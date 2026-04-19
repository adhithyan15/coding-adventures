cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../compiler-ir && pwd)/lib:$(cd ../grammar-tools && pwd)/lib:$(cd ../lexer && pwd)/lib:$(cd ../nib-lexer && pwd)/lib:$(cd ../parser && pwd)/lib:$(cd ../nib-parser && pwd)/lib:$(cd ../type-checker-protocol && pwd)/lib:$(cd ../nib-type-checker && pwd)/lib:${PERL5LIB:-} prove -l -v t/
