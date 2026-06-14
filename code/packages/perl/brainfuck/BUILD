cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../virtual-machine && pwd)/lib:$(cd ../grammar-tools && pwd)/lib:$(cd ../lexer && pwd)/lib:$(cd ../interpreter-ir && pwd)/lib:$(cd ../vm-core && pwd)/lib:$(cd ../codegen-core && pwd)/lib:$(cd ../jit-core && pwd)/lib:${PERL5LIB:-} prove -l -v t/
