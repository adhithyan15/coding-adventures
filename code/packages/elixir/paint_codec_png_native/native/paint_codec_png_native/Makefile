CARGO ?= cargo
PRIV_DIR ?= ../../priv

UNAME := $(shell uname -s)
ifeq ($(UNAME), Darwin)
    LIB_NAME = libpaint_codec_png_native.dylib
else
    LIB_NAME = libpaint_codec_png_native.so
endif

TARGET_LIB = target/release/$(LIB_NAME)

.PHONY: all clean

all: $(PRIV_DIR)/paint_codec_png_native.so

$(PRIV_DIR)/paint_codec_png_native.so: $(TARGET_LIB)
	mkdir -p $(PRIV_DIR)
	cp $(TARGET_LIB) $(PRIV_DIR)/paint_codec_png_native.so

$(TARGET_LIB):
	$(CARGO) build --release

clean:
	$(CARGO) clean
	rm -f $(PRIV_DIR)/paint_codec_png_native.so
