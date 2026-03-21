# frozen_string_literal: true

# --------------------------------------------------------------------------
# extconf.rb -- Native extension build configuration
# --------------------------------------------------------------------------
#
# For Rust-based Ruby extensions using rb_sys, this file serves as the
# bridge between Ruby's extension build system and Cargo. The rb_sys gem
# provides `create_rust_makefile` which generates a Makefile that:
#
# 1. Invokes `cargo build` with the correct target and flags
# 2. Copies the resulting .so/.bundle/.dll to the right location
# 3. Sets up the correct linker flags for the current Ruby installation
#
# This is simpler than a traditional C extension's extconf.rb because
# Cargo handles all the Rust compilation details.
# --------------------------------------------------------------------------

require "mkmf"
require "rb_sys/mkmf"

create_rust_makefile("directed_graph_native/directed_graph_native")
