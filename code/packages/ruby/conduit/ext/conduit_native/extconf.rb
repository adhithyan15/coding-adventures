# frozen_string_literal: true

require "rbconfig"
require_relative "build_config"

dlext = RbConfig::CONFIG["DLEXT"]
cargo_lib = case RbConfig::CONFIG["host_os"]
            when /darwin/
              "libconduit_native.dylib"
            when /mingw|mswin|cygwin/
              "conduit_native.dll"
            else
              "libconduit_native.so"
            end

target = "conduit_native.#{dlext}"
lib_dir = File.expand_path("../../lib", __dir__)
target_dir = ConduitNativeBuildConfig.target_release_dir(File.expand_path(__dir__))
cargo_command = ConduitNativeBuildConfig.cargo_make_command

File.write("Makefile", <<~MAKEFILE)
  CARGO_DIR = #{File.expand_path(__dir__)}
  TARGET_DIR = #{target_dir}
  CARGO = #{cargo_command}
  CARGO_LIB = $(TARGET_DIR)/#{cargo_lib}
  INSTALL_DIR = #{lib_dir}
  TARGET = $(INSTALL_DIR)/#{target}

  all: $(TARGET)

  $(TARGET): $(CARGO_LIB)
  \t@mkdir -p $(INSTALL_DIR)
  \tcp $(CARGO_LIB) $(TARGET)

  $(CARGO_LIB): src/lib.rs Cargo.toml
  \tcd $(CARGO_DIR) && $(CARGO)

  clean:
  \tcd $(CARGO_DIR) && #{ConduitNativeBuildConfig.cargo_clean_command.join(" ")}
  \trm -f $(TARGET)

  install: $(TARGET)

  .PHONY: all clean install
MAKEFILE

puts "Makefile generated; will build via `cargo build --release`"
