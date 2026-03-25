// lib.rs -- CommonMark Ruby native extension using ruby-bridge
// =============================================================
//
// This is a Ruby C extension written in Rust. It wraps the `commonmark`
// crate and exposes two module functions to Ruby:
//
//   CodingAdventures::CommonmarkNative.markdown_to_html(markdown)
//   CodingAdventures::CommonmarkNative.markdown_to_html_safe(markdown)
//
// # Architecture
//
// 1. `Init_commonmark_native()` is called by Ruby when the .so is loaded
// 2. We define the module hierarchy `CodingAdventures::CommonmarkNative`
// 3. Two module functions are attached via `rb_define_module_function`
// 4. Each function extracts the Ruby String argument, calls the Rust
//    `commonmark` crate, and returns a new Ruby String
//
// # The ruby-bridge approach
//
// Instead of Magnus or rb-sys, we use our own `ruby-bridge` crate that
// declares Ruby's C API functions via `extern "C"`. This gives us:
// - Zero dependencies beyond libruby (linked at load time)
// - Complete visibility into every C API call
// - No build-time header requirements
//
// # Module functions vs instance methods
//
// Unlike class-based extensions (like bitset_native), commonmark_native
// exposes module-level functions. In Ruby, `rb_define_module_function`
// defines both a module function (callable as `CommonmarkNative.markdown_to_html`)
// AND a private instance method (if the module is mixed in). We use it
// here because our functions are stateless utilities, not object methods.
//
// The function signature for argc=1 is:
//   extern "C" fn(self_val: VALUE, arg: VALUE) -> VALUE
//
// where `self_val` is the module itself (ignored) and `arg` is the
// Markdown string passed by the caller.
//
// # Error handling
//
// If the caller passes something that is not a String, `rb_string_value_cstr`
// will raise a TypeError. We don't need to handle this ourselves — Ruby's
// type coercion machinery does the right thing.

use std::ffi::c_void;

use commonmark;
use ruby_bridge::VALUE;

/// Raise an ArgumentError.
///
/// We use `ruby_bridge::path2class("ArgumentError")` instead of the
/// `rb_eArgError` extern static because the statics have linking issues
/// on Windows when MinGW Ruby meets the MSVC linker. `rb_path2class` is
/// a regular function and links cleanly on all platforms.
fn raise_arg_error(msg: &str) -> ! {
    ruby_bridge::raise_error(ruby_bridge::path2class("ArgumentError"), msg)
}

// ---------------------------------------------------------------------------
// markdown_to_html(_module, markdown) -> String
// ---------------------------------------------------------------------------
//
// Converts a CommonMark Markdown string to HTML. Raw HTML blocks in the
// Markdown are passed through unchanged (CommonMark spec compliance).
//
// # Arguments
//
//   _module   — the CodingAdventures::CommonmarkNative module (ignored)
//   markdown  — a Ruby String containing CommonMark Markdown
//
// # Returns
//
//   A Ruby String containing the rendered HTML.
//
// # Raises
//
//   TypeError if `markdown` is not a String (Ruby's type coercion raises this)
//
// # Examples
//
//   CodingAdventures::CommonmarkNative.markdown_to_html("# Hello\n\nWorld\n")
//   # => "<h1>Hello</h1>\n<p>World</p>\n"
//
//   CodingAdventures::CommonmarkNative.markdown_to_html("**bold** and *em*\n")
//   # => "<p><strong>bold</strong> and <em>em</em></p>\n"
//
//   # Raw HTML passes through in trusted mode
//   CodingAdventures::CommonmarkNative.markdown_to_html("<div>raw</div>\n\npara\n")
//   # => "<div>raw</div>\n<p>para</p>\n"

extern "C" fn commonmark_markdown_to_html(_module: VALUE, markdown_val: VALUE) -> VALUE {
    // Extract the Ruby String as a Rust &str.
    //
    // `str_from_rb` calls `rb_string_value_cstr` which:
    //   - accepts String arguments directly
    //   - calls `to_str` on objects that respond to it
    //   - raises TypeError for everything else
    let markdown = match ruby_bridge::str_from_rb(markdown_val) {
        Some(s) => s,
        None => raise_arg_error("markdown_to_html: argument must be a String"),
    };

    // Call the Rust commonmark crate. This is infallible — CommonMark's parser
    // accepts any UTF-8 string and never returns an error. Invalid or
    // malformed Markdown is rendered as best-effort (lenient parsing).
    let html = commonmark::markdown_to_html(&markdown);

    // Convert the Rust String back to a Ruby UTF-8 String (new object).
    ruby_bridge::str_to_rb(&html)
}

// ---------------------------------------------------------------------------
// markdown_to_html_safe(_module, markdown) -> String
// ---------------------------------------------------------------------------
//
// Like `markdown_to_html`, but strips all raw HTML blocks and inline HTML
// from the rendered output. Use this when rendering untrusted user-supplied
// Markdown (comments, forum posts, chat messages) to prevent XSS attacks.
//
// The parser still processes all CommonMark syntax; only `RawBlockNode` and
// `RawInlineNode` values are dropped before the HTML renderer runs.
//
// # Examples
//
//   # Attacker tries to inject a script tag through Markdown
//   CodingAdventures::CommonmarkNative.markdown_to_html_safe(
//     "<script>alert(1)</script>\n\n**bold**\n"
//   )
//   # => "<p><strong>bold</strong></p>\n"
//
//   # Regular Markdown is rendered normally
//   CodingAdventures::CommonmarkNative.markdown_to_html_safe("# Hello\n\nWorld\n")
//   # => "<h1>Hello</h1>\n<p>World</p>\n"

extern "C" fn commonmark_markdown_to_html_safe(_module: VALUE, markdown_val: VALUE) -> VALUE {
    let markdown = match ruby_bridge::str_from_rb(markdown_val) {
        Some(s) => s,
        None => raise_arg_error("markdown_to_html_safe: argument must be a String"),
    };

    let html = commonmark::markdown_to_html_safe(&markdown);
    ruby_bridge::str_to_rb(&html)
}

// ---------------------------------------------------------------------------
// Init_commonmark_native -- Ruby extension entry point
// ---------------------------------------------------------------------------
//
// This function MUST be named `Init_commonmark_native` because Ruby derives
// the init function name from the .so filename. When Ruby loads
// `commonmark_native.so`, it calls `Init_commonmark_native()`.
//
// We define the module hierarchy and attach the two module functions:
//
//   module CodingAdventures
//     module CommonmarkNative
//       def self.markdown_to_html(markdown)      → String
//       def self.markdown_to_html_safe(markdown) → String
//     end
//   end

#[no_mangle]
pub extern "C" fn Init_commonmark_native() {
    // -- Module hierarchy ---------------------------------------------------
    //
    // `CodingAdventures` is the top-level namespace shared across all packages
    // in this repo. `CommonmarkNative` is the sub-module specific to this gem.
    let coding_adventures = ruby_bridge::define_module("CodingAdventures");
    let commonmark_native = ruby_bridge::define_module_under(coding_adventures, "CommonmarkNative");

    // -- markdown_to_html ---------------------------------------------------
    //
    // `define_module_function_raw` with argc=1 means the function accepts
    // exactly one positional argument. The Ruby method signature is:
    //   def markdown_to_html(markdown) end
    //
    // The C function signature for argc=1 is:
    //   extern "C" fn(self_val: VALUE, arg: VALUE) -> VALUE
    ruby_bridge::define_module_function_raw(
        commonmark_native,
        "markdown_to_html",
        commonmark_markdown_to_html as *const c_void,
        1,
    );

    // -- markdown_to_html_safe ----------------------------------------------
    ruby_bridge::define_module_function_raw(
        commonmark_native,
        "markdown_to_html_safe",
        commonmark_markdown_to_html_safe as *const c_void,
        1,
    );
}
