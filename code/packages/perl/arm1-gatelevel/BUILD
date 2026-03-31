(cd ../transistors && perl Makefile.PL && make && make install)
(cd ../logic-gates && perl Makefile.PL && make && make install)
(cd ../arithmetic && perl Makefile.PL && make && make install)
(cd ../arm1-simulator && perl Makefile.PL && make && make install)
cpanm --installdeps --quiet .
prove -l -v t/
