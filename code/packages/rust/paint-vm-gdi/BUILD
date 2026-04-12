if [ "$(uname -o 2>/dev/null)" = "Msys" ] || [ "$OS" = "Windows_NT" ]; then
  cargo test -p paint-vm-gdi -- --nocapture
else
  echo "SKIP: paint-vm-gdi requires Windows"
fi
