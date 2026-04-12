cd ../json-rpc && cpanm --notest --quiet Test2::V0 2>/dev/null; cd ../ls00 && prove -I../json-rpc/lib -l -v t/ || for f in t/*.t; do perl -I../json-rpc/lib -Ilib "$f"; done
