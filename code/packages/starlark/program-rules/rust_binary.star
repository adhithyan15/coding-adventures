# ============================================================================
# rust_binary.star — Build rule for Rust executable programs
# ============================================================================
#
# Rust binaries compile to native machine code — fast, standalone executables
# with no runtime dependencies. Like Go, a Rust binary is a single file you
# can copy anywhere and run.
#
# The difference between a Rust library and binary is in Cargo.toml:
#   - Library: has [lib] section, entry point is src/lib.rs
#   - Binary:  has [[bin]] section, entry point is src/main.rs
#   - Both:    a crate can be BOTH a library and a binary
#
# Rust binaries don't need an entry_point parameter because the convention
# is rigid: src/main.rs contains fn main() — always. (You can have multiple
# binaries via [[bin]] sections, but each still has a fixed entry point.)
#
# EXAMPLE BUILD FILE
# ------------------
#   load("//rules:rust_binary.star", "rust_binary")
#
#   rust_binary(
#       name = "assembler-cli",
#       srcs = ["src/**/*.rs"],
#       deps = ["rust/assembler", "rust/parser"],
#   )
#
# ============================================================================

_targets = []


def rust_binary(name, srcs = [], deps = []):
    """Register a Rust binary (executable program) target.

    Rust binaries compile to native executables via Cargo. The build tool runs:
        cargo build -p <name>         — compile the binary
        cargo test -p <name>          — run tests
        cargo build -p <name> --release — optimized build (optional)

    The output binary is placed in target/debug/<name> (or target/release/
    for optimized builds).

    Args:
        name: The program name, matching the directory under
              code/programs/rust/ AND the package name in Cargo.toml.

              This is also the name of the output binary. Cargo converts
              hyphens to underscores for the actual binary filename:
              "assembler-cli" becomes target/debug/assembler-cli (hyphens
              preserved in binary name, unlike crate names in use statements).

        srcs: File paths or glob patterns for change detection.
              Typical: ["src/**/*.rs", "Cargo.toml"]

              Track Cargo.toml because dependency version changes should
              trigger a rebuild.

        deps: Dependencies as "language/package-name" strings.
              Examples:
                  ["rust/assembler"]
                  ["rust/parser", "rust/lexer"]

              Cargo handles transitive dependency resolution, but listing
              deps here helps the build tool's change propagation.

    Note: No entry_point parameter needed. In Rust, the entry point is always
    fn main() in src/main.rs. Cargo enforces this convention.
    """
    _targets.append({
        # "rust_binary" triggers Rust binary-specific build logic:
        #   - cargo build to produce the executable
        #   - cargo test for any tests in the binary crate
        #   - The output binary is in target/debug/ or target/release/
        "rule": "rust_binary",
        "name": name,
        "srcs": srcs,
        "deps": deps,
        "commands": [
            {"type": "cmd", "program": "cargo", "args": ["build"]},
            {"type": "cmd", "program": "cargo", "args": ["test"]},
        ],
    })
