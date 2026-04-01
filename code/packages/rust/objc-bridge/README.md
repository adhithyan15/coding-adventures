# objc-bridge

Zero-dependency Rust wrapper for Apple's Objective-C runtime, Metal,
CoreGraphics, CoreText, and CoreFoundation C APIs.

Like `python-bridge` and `ruby-bridge`, this crate uses raw `extern "C"`
declarations instead of heavyweight binding generators.  Every C function
call is visible and grep-able — no macros, no code generation.
