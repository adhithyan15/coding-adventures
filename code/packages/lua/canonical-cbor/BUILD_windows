luac -p src/coding_adventures/canonical_cbor/constants.lua src/coding_adventures/canonical_cbor/messages.lua src/coding_adventures/canonical_cbor/types.lua src/coding_adventures/canonical_cbor/codec.lua src/coding_adventures/canonical_cbor/init.lua tests/corpus.lua tests/test_canonical_cbor.lua tests/check_coverage.lua
cd tests && busted . --verbose --pattern=test_ && lua check_coverage.lua
