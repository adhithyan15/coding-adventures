cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../mosaic-lexer && pwd)/lib:$(cd ../mosaic-parser && pwd)/lib:$(cd ../mosaic-analyzer && pwd)/lib:$(cd ../mosaic-vm && pwd)/lib:${PERL5LIB:-} prove -l -v t/
