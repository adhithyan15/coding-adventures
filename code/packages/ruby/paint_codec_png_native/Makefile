CARGO_DIR = /Users/adhithya/Downloads/Codex/coding-adventures/code/packages/ruby/paint_codec_png_native/ext/paint_codec_png_native
TARGET_DIR = $(CARGO_DIR)/target/release
CARGO_LIB = $(TARGET_DIR)/libpaint_codec_png_native.dylib
INSTALL_DIR = /Users/adhithya/Downloads/Codex/coding-adventures/code/packages/ruby/paint_codec_png_native/lib
TARGET = $(INSTALL_DIR)/paint_codec_png_native.bundle

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
