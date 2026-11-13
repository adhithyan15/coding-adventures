# build-tool: deps=ruby/font_parser
# Own target dir: font-parser-node, font-parser-python and font-parser-ruby all
# produce a cdylib called font_parser_native, so sharing the workspace target
# directory would let parallel builds overwrite each other's library.
cargo build --release -p font-parser-ruby --target-dir target
cargo clippy -p font-parser-ruby --all-targets --target-dir target -- -D warnings
ruby tests/smoke_test.rb
