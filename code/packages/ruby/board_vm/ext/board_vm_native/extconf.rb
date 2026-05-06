# frozen_string_literal: true

require "rbconfig"

dlext = RbConfig::CONFIG["DLEXT"]
cargo_lib = case RbConfig::CONFIG["host_os"]
            when /darwin/
              "libboard_vm_native.dylib"
            when /mingw|mswin|cygwin/
              "board_vm_native.dll"
            else
              "libboard_vm_native.so"
            end

target = "board_vm_native.#{dlext}"
lib_dir = File.expand_path("../../lib", __dir__)

File.write("Makefile", <<~MAKEFILE)
  CARGO_DIR = #{File.expand_path(__dir__)}
  TARGET_DIR = $(CARGO_DIR)/target/release
  CARGO_LIB = $(TARGET_DIR)/#{cargo_lib}
  INSTALL_DIR = #{lib_dir}
  TARGET = $(INSTALL_DIR)/#{target}

  all: $(TARGET)

  $(TARGET): $(CARGO_LIB)
  \t@mkdir -p $(INSTALL_DIR)
  \tcp $(CARGO_LIB) $(TARGET)

  $(CARGO_LIB): src/lib.rs Cargo.toml build.rs
  \tcd $(CARGO_DIR) && cargo build --release

  clean:
  \tcd $(CARGO_DIR) && cargo clean
  \trm -f $(TARGET)

  install: $(TARGET)

  .PHONY: all clean install
MAKEFILE

puts "Makefile generated; will build board_vm_native via cargo"
