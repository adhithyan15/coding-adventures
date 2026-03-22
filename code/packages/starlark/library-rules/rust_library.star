# ============================================================================
# rust_library.star — Build rule for Rust library packages
# ============================================================================
#
# Rust packages (called "crates") are managed by Cargo, Rust's built-in
# package manager and build system. Each crate has:
#
#   my-package/
#     src/
#       lib.rs          # library entry point (pub mod declarations)
#       module_a.rs     # implementation modules
#     tests/
#       integration.rs  # integration tests (optional)
#     Cargo.toml        # crate metadata, dependencies, features
#
# RUST WORKSPACE
# --------------
# All Rust crates in this monorepo belong to a single Cargo workspace, defined
# by the workspace Cargo.toml at code/packages/rust/Cargo.toml. The workspace
# lists every crate in its [workspace] members array.
#
# This means:
#   - cargo build --workspace compiles everything
#   - cargo test --workspace tests everything
#   - Dependencies between crates use path references:
#       [dependencies]
#       transistors = { path = "../transistors" }
#
# CRITICAL LESSONS LEARNED (see lessons.md):
#   - Workspace Cargo.toml must include ALL crates (except those with their
#     own [workspace] section, like FFI bridge crates)
#   - After creating/modifying a crate, run cargo build --workspace to catch
#     missing exports — don't just test the individual crate
#   - Crates that replace existing ones must export ALL types that downstream
#     crates import
#   - Coverage uses cargo-tarpaulin (not built into Rust like Go's -cover)
#
# WHY NO test_runner PARAMETER?
# ----------------------------
# Unlike Python (pytest vs unittest) or Ruby (minitest vs rspec), Rust has
# exactly one test framework: the built-in #[test] attribute system. There's
# no choice to make, so no test_runner parameter is needed.
#
# EXAMPLE BUILD FILE
# ------------------
#   load("//rules:rust_library.star", "rust_library")
#
#   rust_library(
#       name = "logic-gates",
#       srcs = ["src/**/*.rs"],
#       deps = ["rust/transistors"],
#   )
#
# ============================================================================

_targets = []


def rust_library(name, srcs = [], deps = []):
    """Register a Rust library crate target for the build system.

    Rust libraries use Cargo for building and testing. The build tool runs:
        cargo build -p <name>         — compile the crate
        cargo test -p <name> -- --nocapture — run tests with output
        cargo tarpaulin -p <name>     — measure coverage (if configured)

    The -p flag targets a specific package within the workspace, so only
    this crate is built/tested (though its dependencies are compiled too).

    Args:
        name: The crate name, matching both the directory under
              code/packages/rust/ AND the package name in Cargo.toml.

              Rust crate names use hyphens in the directory but Cargo
              converts them to underscores internally. So "logic-gates"
              becomes logic_gates in Rust code (use statements, etc.).

        srcs: File paths or glob patterns for change detection.
              Typical patterns:
                  ["src/**/*.rs"]                     — source only
                  ["src/**/*.rs", "Cargo.toml"]       — source and deps
                  ["src/**/*.rs", "tests/**/*.rs"]    — source and tests

              Tracking Cargo.toml is recommended because dependency version
              changes should trigger a rebuild even if source code hasn't
              changed.

        deps: Dependencies as "language/package-name" strings.
              These must match the path references in Cargo.toml.
              Examples:
                  ["rust/transistors"]
                  ["rust/logic-gates", "rust/arithmetic"]

              Unlike TypeScript's file: deps, Cargo handles transitive
              dependency resolution correctly — if A depends on B which
              depends on C, you only need to list B in A's deps. Cargo
              will automatically build C when building B. However, listing
              transitive deps explicitly helps the build tool make better
              change propagation decisions.
    """
    _targets.append({
        # "rust_library" triggers Rust-specific build logic:
        #   - cargo build for compilation
        #   - cargo test for testing (Rust's built-in test framework)
        #   - cargo tarpaulin for coverage measurement
        #   - No separate lint step — cargo build with warnings catches most
        #     issues, and clippy can be added as an extra check
        "rule": "rust_library",
        "name": name,
        "srcs": srcs,
        "deps": deps,
    })
