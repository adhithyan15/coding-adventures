(cd ../transistors && cpanm --notest --quiet .)
(cd ../logic-gates && cpanm --notest --quiet .)
(cd ../arithmetic && cpanm --notest --quiet .)
(cd ../arm1-simulator && cpanm --notest --quiet .)
cpanm --quiet --notest Test2::V0
PERL5LIB="$HOME/perl5/lib/perl5${PERL5LIB:+:${PERL5LIB}}" prove -l -v t/
