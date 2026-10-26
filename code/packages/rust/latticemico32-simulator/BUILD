cargo test -p latticemico32-simulator --release --no-fail-fast
cargo clippy -p latticemico32-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p latticemico32-simulator --no-deps
python3 latticemico32-simulator/tools/generate_vectors.py --check
ruff check latticemico32-simulator/tools/generate_vectors.py
