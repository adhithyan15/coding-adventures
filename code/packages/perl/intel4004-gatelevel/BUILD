cpanm --notest --quiet Test2::V0
(cd ../transistors && perl Makefile.PL && make && make install)
(cd ../logic-gates && perl Makefile.PL && make && make install)
(cd ../arithmetic && perl Makefile.PL && make && make install)
prove -l -v t/
