cpanm --notest --quiet Test2::V0
PERL5LIB=$(cd ../md5 && pwd)/lib:$(cd ../sha1 && pwd)/lib:${PERL5LIB:-} prove -l -v t/
