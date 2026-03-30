cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../grammar-tools && pwd)/lib:$(cd ../lattice-lexer && pwd)/lib:$(cd ../lattice-parser && pwd)/lib:${PERL5LIB:-} prove -l -v t/
