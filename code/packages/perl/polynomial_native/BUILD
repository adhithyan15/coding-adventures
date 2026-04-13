if command -v mise >/dev/null 2>&1; then
  PERL_BIN=$(mise which perl)
else
  PERL_BIN=$(command -v perl)
fi

cargo build --release &&
soext=$("$PERL_BIN" -MConfig -e 'print $Config{so}') &&
dlext=$("$PERL_BIN" -MConfig -e 'print $Config{dlext}') &&
mkdir -p lib/auto/CodingAdventures/PolynomialNative &&
cp target/release/libPolynomialNative.$soext lib/auto/CodingAdventures/PolynomialNative/PolynomialNative.$dlext &&
"$PERL_BIN" -I"$PWD/lib" -MCodingAdventures::PolynomialNative -e 'print qq{loaded\n}'
