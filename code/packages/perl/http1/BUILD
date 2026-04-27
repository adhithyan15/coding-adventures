command -v cpanm >/dev/null 2>&1 && cpanm --notest --quiet Test2::V0 || true
PERL5LIB=$(cd ../http-core && pwd)/lib:${PERL5LIB:-} prove -l -v t/
