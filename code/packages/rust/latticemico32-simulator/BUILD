cargo test -p latticemico32-simulator --release --no-fail-fast
cargo fmt -p latticemico32-simulator -- --check
cargo clippy -p latticemico32-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p latticemico32-simulator --no-deps
python3 tools/generate_vectors.py --check
ruff check tools/generate_vectors.py
ruff format --check tools/generate_vectors.py
