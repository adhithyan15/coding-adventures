// lib.rs -- CommonMark Native Python Extension
// =============================================
//
// Native extension wrapping the Rust `commonmark` crate for Python via our
// zero-dependency python-bridge. Exposes two module-level functions:
//
//   markdown_to_html(markdown: str) -> str
//   markdown_to_html_safe(markdown: str) -> str
//
// # Architecture
//
// Python extension modules define a `PyInit_<name>` entry point that returns
// a new Python module object. We build the module with a `PyModuleDef` struct
// that carries a method table (`PyMethodDef[]`). When Python imports the
// extension, it calls `PyInit_commonmark_native`, which creates the module and
// attaches both functions.
//
// # Why two functions?
//
//   - `markdown_to_html`      — Full CommonMark 0.31.2 compliance, including raw
//                               HTML passthrough. Use for trusted author-controlled
//                               Markdown (documentation sites, blog posts, etc.).
//
//   - `markdown_to_html_safe` — Same parser, but strips all raw HTML blocks and
//                               inline HTML before rendering. Use for untrusted
//                               user-supplied Markdown (comments, forum posts,
//                               chat messages) to prevent XSS attacks.
//
// # Memory model
//
// `PyModuleDef` and its `PyMethodDef` table must outlive the module (i.e.,
// they need static lifetime). The cleanest Rust approach is `Box::leak`:
// allocate on the heap and intentionally "forget" the allocation. The OS
// will reclaim the memory when the process exits.
//
// # Calling convention
//
// Method functions for METH_VARARGS receive:
//   - `_module`: the module object (ignored)
//   - `args`:    a Python tuple containing the positional arguments
//
// We extract the first argument (index 0) as a UTF-8 string, call the
// corresponding Rust function, and return a new Python str.

use std::ffi::c_char;
use std::ptr;

use python_bridge::*;

// ---------------------------------------------------------------------------
// markdown_to_html(markdown: str) -> str
// ---------------------------------------------------------------------------
//
// Converts a CommonMark Markdown string to HTML. Raw HTML blocks are passed
// through unchanged — required for full CommonMark spec compliance.
//
// Raises TypeError if the argument is missing or not a string.
//
// Examples
// --------
//
//   >>> import commonmark_native
//   >>> commonmark_native.markdown_to_html("# Hello\n\nWorld\n")
//   '<h1>Hello</h1>\n<p>World</p>\n'
//
//   >>> commonmark_native.markdown_to_html("Hello **world**\n")
//   '<p>Hello <strong>world</strong></p>\n'
//
//   >>> commonmark_native.markdown_to_html("<div>raw</div>\n\nparagraph\n")
//   '<div>raw</div>\n<p>paragraph</p>\n'

unsafe extern "C" fn py_markdown_to_html(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    // Extract the first positional argument as a Rust String.
    // parse_arg_str returns None if:
    //   - the args tuple is empty (no argument given)
    //   - the argument is not a str (type mismatch)
    let md = match parse_arg_str(args, 0) {
        Some(s) => s,
        None => {
            // Set a Python ValueError — the caller will see this as an exception.
            set_error(
                type_error_class(),
                "markdown_to_html() requires a string argument",
            );
            // Returning null signals that an exception is active.
            return ptr::null_mut();
        }
    };

    // Call the Rust commonmark crate. This never panics — invalid Markdown
    // is not an error; the parser is lenient by design (CommonMark spec).
    let html = commonmark::markdown_to_html(&md);

    // Convert the Rust String to a Python str (new reference, UTF-8 encoded).
    str_to_py(&html)
}

// ---------------------------------------------------------------------------
// markdown_to_html_safe(markdown: str) -> str
// ---------------------------------------------------------------------------
//
// Like `markdown_to_html`, but strips all raw HTML from the output. Safe for
// rendering untrusted user-supplied Markdown in web applications.
//
// The parser still accepts and processes all CommonMark syntax; only the
// raw HTML nodes (RawBlockNode, RawInlineNode) are dropped before rendering.
// This prevents XSS attacks while preserving all Markdown formatting.
//
// Raises TypeError if the argument is missing or not a string.
//
// Examples
// --------
//
//   >>> import commonmark_native
//   >>> commonmark_native.markdown_to_html_safe("<script>alert(1)</script>\n\n**bold**\n")
//   '<p><strong>bold</strong></p>\n'
//
//   >>> commonmark_native.markdown_to_html_safe("# Hello\n\nWorld\n")
//   '<h1>Hello</h1>\n<p>World</p>\n'

unsafe extern "C" fn py_markdown_to_html_safe(
    _module: PyObjectPtr,
    args: PyObjectPtr,
) -> PyObjectPtr {
    let md = match parse_arg_str(args, 0) {
        Some(s) => s,
        None => {
            set_error(
                type_error_class(),
                "markdown_to_html_safe() requires a string argument",
            );
            return ptr::null_mut();
        }
    };

    let html = commonmark::markdown_to_html_safe(&md);
    str_to_py(&html)
}

// ---------------------------------------------------------------------------
// Module entry point: PyInit_commonmark_native
// ---------------------------------------------------------------------------
//
// Python calls this function when `import commonmark_native` is executed.
// The function name MUST match `PyInit_<lib_name>` where `<lib_name>` is
// the `name` field in `[lib]` in Cargo.toml.
//
// We build the PyModuleDef and PyMethodDef array on the heap and leak them
// (intentional: they must live for the entire process lifetime). Then we
// call PyModule_Create2 to construct the Python module object.
//
// The returned module has these attributes:
//
//   commonmark_native.markdown_to_html(s)       -- full CommonMark
//   commonmark_native.markdown_to_html_safe(s)  -- strips raw HTML

#[no_mangle]
pub unsafe extern "C" fn PyInit_commonmark_native() -> PyObjectPtr {
    // Build the method table.
    //
    // A PyMethodDef describes one Python-callable function:
    //   ml_name  : name visible in Python
    //   ml_meth  : function pointer (takes module + args tuple, returns PyObject)
    //   ml_flags : METH_VARARGS means args are passed as a positional tuple
    //   ml_doc   : docstring shown by help()
    //
    // The sentinel (all zeros / nulls) marks the end of the table.
    let methods: &'static mut [PyMethodDef; 3] = Box::leak(Box::new([
        PyMethodDef {
            ml_name: b"markdown_to_html\0".as_ptr() as *const c_char,
            ml_meth: Some(py_markdown_to_html),
            ml_flags: METH_VARARGS,
            ml_doc: b"Convert CommonMark Markdown to HTML.\n\n\
                      Raw HTML blocks are passed through (trusted input only).\n\
                      Use markdown_to_html_safe() for untrusted user content.\0"
                .as_ptr() as *const c_char,
        },
        PyMethodDef {
            ml_name: b"markdown_to_html_safe\0".as_ptr() as *const c_char,
            ml_meth: Some(py_markdown_to_html_safe),
            ml_flags: METH_VARARGS,
            ml_doc: b"Convert CommonMark Markdown to HTML, stripping raw HTML.\n\n\
                      Safe for untrusted user-supplied Markdown -- prevents XSS by\n\
                      dropping all raw HTML blocks and inline HTML.\0"
                .as_ptr() as *const c_char,
        },
        // Sentinel: the last entry must have all fields null/zero.
        method_def_sentinel(),
    ]));

    // Build the module definition.
    //
    // A PyModuleDef describes the module itself:
    //   m_base    : boilerplate (zeroed out, Python fills it in)
    //   m_name    : module name (must match the .so filename)
    //   m_doc     : module docstring
    //   m_size    : -1 means the module has no per-interpreter state
    //   m_methods : pointer to the method table above
    //   others    : null (no slots, traversal, clearing, or free functions)
    let def: &'static mut PyModuleDef = Box::leak(Box::new(PyModuleDef {
        m_base: PyModuleDef_Base {
            ob_base: [0u8; std::mem::size_of::<usize>() * 2],
            m_init: None,
            m_index: 0,
            m_copy: ptr::null_mut(),
        },
        m_name: b"commonmark_native\0".as_ptr() as *const c_char,
        m_doc: b"Rust-backed CommonMark Markdown to HTML converter.\n\n\
                 Wraps the Rust `commonmark` crate via python-bridge FFI.\n\
                 Zero third-party Python dependencies.\0"
            .as_ptr() as *const c_char,
        m_size: -1,
        m_methods: methods.as_mut_ptr(),
        m_slots: ptr::null_mut(),
        m_traverse: ptr::null_mut(),
        m_clear: ptr::null_mut(),
        m_free: ptr::null_mut(),
    }));

    // Create and return the Python module object.
    //
    // PYTHON_API_VERSION (1013) is the stable ABI version constant. Python
    // checks this matches what it expects; using the wrong value causes an
    // import error at runtime.
    PyModule_Create2(def as *mut PyModuleDef, PYTHON_API_VERSION)
}
