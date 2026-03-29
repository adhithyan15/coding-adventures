cpanm --notest --quiet Test2::V0
cd ../matrix && cpanm --notest --nodeps --quiet . && cd -
cd ../loss-functions && cpanm --notest --nodeps --quiet . && cd -
cpanm --notest --nodeps --quiet .
prove -l -v t/
