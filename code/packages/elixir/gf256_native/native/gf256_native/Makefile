CARGO ?= cargo
PRIV_DIR ?= ../../priv

UNAME := $(shell uname -s)
ifeq ($(UNAME), Darwin)
    LIB_NAME = libgf256_native.dylib
else
    LIB_NAME = libgf256_native.so
endif

TARGET_LIB = target/release/$(LIB_NAME)

.PHONY: all clean

all: $(PRIV_DIR)/gf256_native.so

$(PRIV_DIR)/gf256_native.so: $(TARGET_LIB)
	mkdir -p $(PRIV_DIR)
	cp $(TARGET_LIB) $(PRIV_DIR)/gf256_native.so

$(TARGET_LIB):
	$(CARGO) build --release

clean:
	$(CARGO) clean
	rm -f $(PRIV_DIR)/gf256_native.so
