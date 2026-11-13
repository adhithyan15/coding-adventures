# build-tool: deps=python/font-parser
# Own target dir: font-parser-node, font-parser-python and font-parser-ruby all
# produce a cdylib called font_parser_native, so sharing the workspace target
# directory would let parallel builds overwrite each other's library.
cargo build --release -p font-parser-python --target-dir target
cargo clippy -p font-parser-python --all-targets --target-dir target -- -D warnings
python3 tests/smoke_test.py
