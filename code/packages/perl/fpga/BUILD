cpanm --installdeps --quiet .
PERL5LIB=$(cd ../logic-gates && pwd)/lib:$(cd ../block-ram && pwd)/lib:${PERL5LIB:-} prove -l -v t/
