cpanm --notest --quiet Test2::V0
cd ../md5 && cpanm --notest --nodeps --quiet . && cd -
cd ../sha1 && cpanm --notest --nodeps --quiet . && cd -
cpanm --notest --nodeps --quiet .
prove -l -v t/
