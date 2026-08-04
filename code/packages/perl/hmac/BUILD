cpanm --with-test --installdeps --quiet .
PERL5LIB=$(cd ../md5 && pwd)/lib:$(cd ../sha1 && pwd)/lib:$(cd ../sha256 && pwd)/lib:$(cd ../sha512 && pwd)/lib:${PERL5LIB:-} prove -l -v t/
