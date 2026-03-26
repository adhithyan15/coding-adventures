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
    // `node_bridge::create_function` wraps `napi_create_function` (N-API v1).
    let markdown_to_html_fn =
        node_bridge::create_function(env, "markdownToHtml", Some(node_markdown_to_html));
    set_named_property(env, exports, "markdownToHtml", markdown_to_html_fn);

    // Create the markdownToHtmlSafe function and attach it to exports.
    let markdown_to_html_safe_fn =
        node_bridge::create_function(env, "markdownToHtmlSafe", Some(node_markdown_to_html_safe));
    set_named_property(env, exports, "markdownToHtmlSafe", markdown_to_html_safe_fn);

    exports
}
