cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../interpreter-ir && pwd)/lib:$(cd ../vm-core && pwd)/lib:$(cd ../codegen-core && pwd)/lib:$(cd ../jit-core && pwd)/lib:${PERL5LIB:-} prove -l -v t/
