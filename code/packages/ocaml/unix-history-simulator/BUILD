# needs-toolchain: python
opam install . --deps-only --with-test --with-dev-setup --yes --require-checksums
opam exec -- python3 ../../../specs/fixtures/historical-unix-v1/build_ocaml_swift.py ocaml
