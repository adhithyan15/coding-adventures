// lib.rs -- CommonMark Node.js native addon using node-bridge
// ============================================================
//
// This crate exposes the Rust `commonmark` crate to Node.js via N-API,
// using our zero-dependency `node-bridge` crate. No napi-rs, no napi-sys,
// no build-time header requirements -- just raw N-API calls through
// node-bridge's safe wrappers.
//
// # Architecture
//
// 1. `napi_register_module_v1()` is the entry point called by Node.js when
//    the addon is loaded via `require()`. It attaches two functions to the
//    exports object.
//
// 2. `node_markdown_to_html` and `node_markdown_to_html_safe` are N-API
//    callback functions that:
//    a. Extract the single string argument from the JS call
//    b. Call the corresponding Rust function from the `commonmark` crate
//    c. Convert the result back to a JS string and return it
//
// # Why two functions?
//
//   markdownToHtml(s)      -- Full CommonMark spec compliance including raw
//                             HTML passthrough. Use for trusted content.
//
//   markdownToHtmlSafe(s)  -- Strips raw HTML before rendering. Use for
//                             untrusted user-supplied content (XSS prevention).
//
// # Function naming
//
// JavaScript convention is camelCase:
//   markdownToHtml, markdownToHtmlSafe
//
// These match the TypeScript definitions in index.d.ts and the exports
// in index.js.
//
// # Error handling
//
// If the caller passes a non-string argument, we throw a JavaScript TypeError
// and return undefined. The caller will see an exception.

use commonmark;
use node_bridge::*;
use std::ffi::{c_void, CString};
use std::ptr;

// ---------------------------------------------------------------------------
// Extra N-API extern: napi_create_function
// ---------------------------------------------------------------------------
//
// `napi_create_function` creates a JS function value that we can attach to
// the exports object. node-bridge provides `define_class` for class-based
// APIs, but for module-level functions we need to create standalone function
// objects.
//
// N-API v1 (stable since Node.js 8.0.0):
//   napi_status napi_create_function(
//     napi_env env,
//     const char* utf8name,     // function name (shown in stack traces)
//     size_t length,            // byte length of name (or NAPI_AUTO_LENGTH)
//     napi_callback cb,         // the function implementation
//     void* data,               // optional user data passed to cb
//     napi_value* result        // OUT: the new function value
//   )

extern "C" {
    fn napi_create_function(
        env: napi_env,
        utf8name: *const std::ffi::c_char,
        length: usize,
        cb: napi_callback,
        data: *const c_void,
        result: *mut napi_value,
    ) -> napi_status;
}

/// Create a JS function with the given name and callback.
///
/// The function is a standalone value — call `set_named_property` to attach
/// it to an object (e.g., the module exports).
unsafe fn create_function(env: napi_env, name: &str, cb: napi_callback) -> napi_value {
    let c_name = CString::new(name).expect("function name must not contain NUL");
    let mut result: napi_value = ptr::null_mut();
    // NAPI_AUTO_LENGTH (usize::MAX) tells N-API to use strlen to find the end
    // of the name string.
    napi_create_function(env, c_name.as_ptr(), usize::MAX, cb, ptr::null(), &mut result);
    result
}

// ---------------------------------------------------------------------------
// markdownToHtml(markdown: string) -> string
// ---------------------------------------------------------------------------
//
// Converts a CommonMark Markdown string to HTML. Raw HTML blocks in the
// Markdown are passed through unchanged — required for full CommonMark
// 0.31.2 spec compliance.
//
// Throws a TypeError if the argument is not a string.
//
// Example (JS):
//   import { markdownToHtml } from "@coding-adventures/commonmark-native";
//   markdownToHtml("# Hello\n\nWorld\n");
//   // → "<h1>Hello</h1>\n<p>World</p>\n"
//
//   markdownToHtml("<div>raw</div>\n\nparagraph\n");
//   // → "<div>raw</div>\n<p>paragraph</p>\n"

unsafe extern "C" fn node_markdown_to_html(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    // Extract callback arguments. We expect exactly 1: the Markdown string.
    let (_this, args) = get_cb_info(env, info, 1);

    if args.is_empty() {
        throw_error(env, "markdownToHtml() requires a string argument");
        return undefined(env);
    }

    // Convert the JS string to a Rust String.
    // str_from_js returns None if the argument is not a string type.
    let markdown = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "markdownToHtml(): argument must be a string");
            return undefined(env);
        }
    };

    // Call the Rust commonmark crate. This is infallible -- CommonMark's
    // parser accepts any string and never errors. Malformed Markdown is
    // rendered leniently per the spec.
    let html = commonmark::markdown_to_html(&markdown);

    // Convert the result back to a JS string (new value, UTF-8 encoded).
    str_to_js(env, &html)
}

// ---------------------------------------------------------------------------
// markdownToHtmlSafe(markdown: string) -> string
// ---------------------------------------------------------------------------
//
// Like `markdownToHtml`, but strips all raw HTML blocks and inline HTML from
// the rendered output. Prevents XSS attacks when rendering untrusted
// user-supplied Markdown in web applications.
//
// The parser still processes all CommonMark syntax; only the raw HTML nodes
// (RawBlockNode, RawInlineNode) are dropped before the HTML renderer runs.
//
// Throws a TypeError if the argument is not a string.
//
// Example (JS):
//   import { markdownToHtmlSafe } from "@coding-adventures/commonmark-native";
//
//   // Attacker tries to inject a script tag via raw HTML in Markdown:
//   markdownToHtmlSafe("<script>alert(1)</script>\n\n**bold**\n");
//   // → "<p><strong>bold</strong></p>\n"
//
//   // Regular Markdown is rendered normally:
//   markdownToHtmlSafe("# Hello\n\nWorld\n");
//   // → "<h1>Hello</h1>\n<p>World</p>\n"

unsafe extern "C" fn node_markdown_to_html_safe(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 1);

    if args.is_empty() {
        throw_error(env, "markdownToHtmlSafe() requires a string argument");
        return undefined(env);
    }

    let markdown = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "markdownToHtmlSafe(): argument must be a string");
            return undefined(env);
        }
    };

    let html = commonmark::markdown_to_html_safe(&markdown);
    str_to_js(env, &html)
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------
//
// N-API calls this function when the addon is loaded via `require()`.
// We create two standalone function values and attach them to the exports
// object under camelCase names.
//
// The resulting module exports:
//   exports.markdownToHtml      = function(markdown) { ... }
//   exports.markdownToHtmlSafe  = function(markdown) { ... }

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    // Create the markdownToHtml function and attach it to exports.
    //
    // The name argument is what appears in stack traces -- we use the
    // camelCase JavaScript name for consistency with JS tooling.
    let markdown_to_html_fn = create_function(env, "markdownToHtml", Some(node_markdown_to_html));
    set_named_property(env, exports, "markdownToHtml", markdown_to_html_fn);

    // Create the markdownToHtmlSafe function and attach it to exports.
    let markdown_to_html_safe_fn =
        create_function(env, "markdownToHtmlSafe", Some(node_markdown_to_html_safe));
    set_named_property(env, exports, "markdownToHtmlSafe", markdown_to_html_safe_fn);

    exports
}
