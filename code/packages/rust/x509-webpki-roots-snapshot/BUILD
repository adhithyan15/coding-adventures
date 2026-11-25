python3 tools/verify_snapshot.py
python3 -m unittest discover -s tests -p 'test_*.py'
cargo test --locked -p x509-webpki-roots-snapshot --test x509_webpki_roots_snapshot -- --nocapture
cargo test --locked -p x509-webpki-roots-snapshot --doc
