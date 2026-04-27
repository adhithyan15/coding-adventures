cpanm --notest --quiet Test2::V0
(cd ../transistors && cpanm --notest --quiet .)
(cd ../logic-gates && cpanm --notest --quiet .)
(cd ../arithmetic && cpanm --notest --quiet .)
(cd ../intel8008-simulator && cpanm --notest --quiet .)
PERL5LIB=../transistors/lib:../logic-gates/lib:../arithmetic/lib:../intel8008-simulator/lib prove -l -v t/
