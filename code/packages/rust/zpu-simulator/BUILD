python3 zpu-simulator/tools/verify_oracle_vectors.py
ruff check zpu-simulator/tools/regenerate_oracle_vectors.py zpu-simulator/tools/verify_oracle_vectors.py
ruff format --check zpu-simulator/tools/regenerate_oracle_vectors.py zpu-simulator/tools/verify_oracle_vectors.py
cargo test -p zpu-simulator --release --no-fail-fast
cargo clippy -p zpu-simulator --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p zpu-simulator --no-deps
