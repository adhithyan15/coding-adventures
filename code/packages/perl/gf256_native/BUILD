if command -v mise >/dev/null 2>&1; then
  PERL_BIN=$(mise which perl)
else
  PERL_BIN=$(command -v perl)
fi

cargo build --release &&
soext=$("$PERL_BIN" -MConfig -e 'print $Config{so}') &&
dlext=$("$PERL_BIN" -MConfig -e 'print $Config{dlext}') &&
mkdir -p lib/auto/CodingAdventures/GF256Native &&
cp target/release/libGF256Native.$soext lib/auto/CodingAdventures/GF256Native/GF256Native.$dlext &&
"$PERL_BIN" -I"$PWD/lib" -MCodingAdventures::GF256Native -e 'print CodingAdventures::GF256Native::add(1,2), qq{\n}'
