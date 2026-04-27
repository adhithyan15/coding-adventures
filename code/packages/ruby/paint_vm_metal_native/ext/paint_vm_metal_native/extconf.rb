require "rbconfig"

dlext = RbConfig::CONFIG["DLEXT"]

case RbConfig::CONFIG["host_os"]
when /darwin/
  cargo_lib = "libpaint_vm_metal_native.dylib"
when /mingw|mswin|cygwin/
  cargo_lib = "paint_vm_metal_native.dll"
else
  cargo_lib = "libpaint_vm_metal_native.so"
end

target = "paint_vm_metal_native.#{dlext}"
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

  $(CARGO_LIB): src/lib.rs Cargo.toml
  \tcd $(CARGO_DIR) && cargo build --release

  clean:
  \tcd $(CARGO_DIR) && cargo clean
  \trm -f $(TARGET)

  install: $(TARGET)

  .PHONY: all clean install
MAKEFILE

puts "Makefile generated for paint_vm_metal_native"
