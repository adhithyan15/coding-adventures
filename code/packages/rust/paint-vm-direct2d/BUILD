if [ "$(uname -o 2>/dev/null)" = "Msys" ] || [ "$OS" = "Windows_NT" ]; then
  cargo test -p paint-vm-direct2d -- --nocapture
else
  echo "SKIP: paint-vm-direct2d requires Windows"
fi
