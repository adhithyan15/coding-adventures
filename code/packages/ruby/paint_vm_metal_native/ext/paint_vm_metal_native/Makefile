CARGO_DIR = /Users/adhithya/Downloads/Codex/coding-adventures-barcode-all-languages/code/packages/ruby/paint_vm_metal_native/ext/paint_vm_metal_native
TARGET_DIR = $(CARGO_DIR)/target/release
CARGO_LIB = $(TARGET_DIR)/libpaint_vm_metal_native.dylib
INSTALL_DIR = /Users/adhithya/Downloads/Codex/coding-adventures-barcode-all-languages/code/packages/ruby/paint_vm_metal_native/lib
TARGET = $(INSTALL_DIR)/paint_vm_metal_native.bundle

all: $(TARGET)

$(TARGET): $(CARGO_LIB)
	@mkdir -p $(INSTALL_DIR)
	cp $(CARGO_LIB) $(TARGET)

$(CARGO_LIB): src/lib.rs Cargo.toml
	cd $(CARGO_DIR) && cargo build --release

clean:
	cd $(CARGO_DIR) && cargo clean
	rm -f $(TARGET)

install: $(TARGET)

.PHONY: all clean install
