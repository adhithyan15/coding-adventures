library;

// This monorepo program intentionally exposes no callable library API. The
// checked-in bin entry point (bin/cowsay.dart) owns process wiring; tests
// import lib/src/cowsay.dart directly, matching the scaffold-generator
// program's convention.
