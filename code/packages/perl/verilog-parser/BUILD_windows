cd ../grammar-tools && cpanm --notest --quiet . && cd -
cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../verilog-lexer && pwd)/lib:${PERL5LIB:-} prove -l -v t/
