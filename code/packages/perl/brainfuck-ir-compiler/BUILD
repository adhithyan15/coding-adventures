cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../compiler-ir && pwd)/lib:$(cd ../compiler-source-map && pwd)/lib:$(cd ../virtual-machine && pwd)/lib:$(cd ../brainfuck && pwd)/lib:$(cd ../grammar-tools && pwd)/lib:$(cd ../lexer && pwd)/lib:${PERL5LIB:-} prove -l -v t/
