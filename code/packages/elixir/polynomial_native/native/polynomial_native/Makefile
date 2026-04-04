# Makefile for building the Rust NIF shared library.
#
# Mix's :make compiler calls `make all` in this directory, then Mix copies
# the resulting shared library to the application's priv/ directory.
#
# The PRIV_DIR variable is set by elixir_make to the path of the priv dir.

CARGO ?= cargo
PRIV_DIR ?= ../../priv

# Detect the OS to determine the shared library extension.
UNAME := $(shell uname -s)
ifeq ($(UNAME), Darwin)
    LIB_EXT = dylib
    # Erlang on macOS expects .so, so we rename the dylib.
    LIB_NAME = libpolynomial_native.dylib
else
    LIB_EXT = so
    LIB_NAME = libpolynomial_native.so
endif

TARGET_LIB = target/release/$(LIB_NAME)

.PHONY: all clean

all: $(PRIV_DIR)/polynomial_native.so

$(PRIV_DIR)/polynomial_native.so: $(TARGET_LIB)
	mkdir -p $(PRIV_DIR)
	cp $(TARGET_LIB) $(PRIV_DIR)/polynomial_native.so

$(TARGET_LIB):
	$(CARGO) build --release

clean:
	$(CARGO) clean
	rm -f $(PRIV_DIR)/polynomial_native.so
