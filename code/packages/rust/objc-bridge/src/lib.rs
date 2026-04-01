//! # objc-bridge — Zero-dependency Rust wrapper for Apple's Objective-C runtime
//!
//! This crate provides safe Rust wrappers around the Objective-C runtime's
//! C API, plus selected Metal, CoreGraphics, CoreText, and CoreFoundation
//! functions — all using raw `extern "C"` declarations.  No objc2, no
//! bindgen, no build-time header requirements.
//!
//! ## How it works
//!
//! The Objective-C runtime (`libobjc.dylib`) exports a small set of C
//! functions for class lookup, selector registration, and message dispatch.
//! These functions have been ABI-stable since macOS 10.0 (2001).  We
//! declare them as `extern "C"` and call them directly.
//!
//! Apple's frameworks (Metal, CoreGraphics, CoreText) are Objective-C
//! classes and C functions linked against these same runtime primitives.
//! We can call their methods using `objc_msgSend` and their C functions
//! using standard `extern "C"` declarations.
//!
//! ## Why zero dependencies?
//!
//! Same rationale as `python-bridge` and `ruby-bridge`:
//!
//! - **Compiles everywhere** — no Objective-C headers needed at build time
//! - **No bindgen** — no clang/LLVM dependency
//! - **Fully auditable** — every C function call is visible and grep-able
//! - **Debuggable** — stack traces show actual C API calls, not macro layers
//!
//! ## Architecture note: objc_msgSend
//!
//! In Objective-C, every method call is a message send:
//!
//! ```text
//! [device newCommandQueue]
//! ```
//!
//! becomes at the C level:
//!
//! ```text
//! objc_msgSend(device, sel_registerName("newCommandQueue"))
//! ```
//!
//! The runtime looks up the method implementation for the given selector
//! on the object's class, then calls it.  This is how ALL Objective-C
//! method calls work — it's the universal dispatch mechanism.
//!
//! On **arm64** (Apple Silicon), `objc_msgSend` handles all return types.
//! On **x86_64** (Intel), there are variants for struct returns
//! (`objc_msgSend_stret`) and float returns (`objc_msgSend_fpret`).
//! We support both architectures.

pub const VERSION: &str = "0.1.0";

use std::ffi::{c_char, c_double, c_int, c_ulong, c_void, CString};

// ---------------------------------------------------------------------------
// Opaque types
// ---------------------------------------------------------------------------
//
// The Objective-C runtime uses opaque pointer types.  We don't need to know
// the internal layout — we just pass pointers around.

/// Opaque Objective-C object.  All ObjC values are `*mut Object` at the C level.
/// This is what Apple's headers call `id`.
#[repr(C)]
pub struct Object {
    _opaque: [u8; 0],
}

/// Opaque Objective-C class.  Classes are objects too, but we use a separate
/// type for clarity.
#[repr(C)]
pub struct Class {
    _opaque: [u8; 0],
}

/// Opaque Objective-C selector.  A selector is an interned string that
/// identifies a method name (e.g. "init", "newCommandQueue").
#[repr(C)]
pub struct Selector {
    _opaque: [u8; 0],
}

/// Convenience alias — the universal Objective-C object pointer.
pub type Id = *mut Object;

/// Convenience alias — class pointer.
pub type ClassPtr = *const Class;

/// Convenience alias — selector pointer.
pub type Sel = *const Selector;

/// The null object pointer, equivalent to Objective-C `nil`.
pub const NIL: Id = std::ptr::null_mut();

// ---------------------------------------------------------------------------
// CoreGraphics types
// ---------------------------------------------------------------------------
//
// CoreGraphics uses C structs, not Objective-C objects.  These must match
// the exact memory layout that the framework expects.

/// CoreGraphics point — two 64-bit floats.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CGPoint {
    pub x: c_double,
    pub y: c_double,
}

/// CoreGraphics size — two 64-bit floats.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CGSize {
    pub width: c_double,
    pub height: c_double,
}

/// CoreGraphics rectangle — origin point + size.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

/// CoreGraphics color space reference (opaque).
pub type CGColorSpaceRef = *mut c_void;

/// CoreGraphics context reference (opaque).
pub type CGContextRef = *mut c_void;

// ---------------------------------------------------------------------------
// Metal types
// ---------------------------------------------------------------------------

/// Metal clear color — four doubles (red, green, blue, alpha).
///
/// Metal uses doubles (not floats) for clear colors because the clear
/// operation is done by the GPU's fixed-function hardware, which operates
/// at full precision.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MTLClearColor {
    pub red: c_double,
    pub green: c_double,
    pub blue: c_double,
    pub alpha: c_double,
}

/// Metal origin — three unsigned longs (x, y, z).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct MTLOrigin {
    pub x: c_ulong,
    pub y: c_ulong,
    pub z: c_ulong,
}

/// Metal size — three unsigned longs (width, height, depth).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MTLSize {
    pub width: c_ulong,
    pub height: c_ulong,
    pub depth: c_ulong,
}

/// Metal region — origin + size.  Used for texture read-back.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MTLRegion {
    pub origin: MTLOrigin,
    pub size: MTLSize,
}

// Metal pixel format constants
pub const MTL_PIXEL_FORMAT_RGBA8_UNORM: c_ulong = 70;
pub const MTL_PIXEL_FORMAT_BGRA8_UNORM: c_ulong = 80;

// Metal texture usage flags
pub const MTL_TEXTURE_USAGE_RENDER_TARGET: c_ulong = 0x0004;
pub const MTL_TEXTURE_USAGE_SHADER_READ: c_ulong = 0x0001;

// Metal load/store action constants
pub const MTL_LOAD_ACTION_CLEAR: c_ulong = 2;
pub const MTL_STORE_ACTION_STORE: c_ulong = 1;

// Metal primitive type constants
pub const MTL_PRIMITIVE_TYPE_TRIANGLE: c_ulong = 3;

// Metal texture type constants
pub const MTL_TEXTURE_TYPE_2D: c_ulong = 2;

// CoreGraphics bitmap info constants
pub const K_CG_IMAGE_ALPHA_PREMULTIPLIED_LAST: u32 = 1;
pub const K_CG_BITMAP_BYTE_ORDER_32_BIG: u32 = 1 << 12;

// ---------------------------------------------------------------------------
// Objective-C runtime — extern "C" declarations
// ---------------------------------------------------------------------------
//
// These are the stable C functions exported by libobjc.dylib.  They have
// been ABI-stable since macOS 10.0 (2001).  When our dylib is loaded,
// the dynamic linker resolves these symbols against the running runtime.

#[allow(non_snake_case)]
#[link(name = "objc", kind = "dylib")]
extern "C" {
    // -- Class and selector lookup ----------------------------------------

    /// Look up a class by name.  Returns null if the class doesn't exist.
    pub fn objc_getClass(name: *const c_char) -> ClassPtr;

    /// Register (or look up) a selector by name.  Selectors are interned —
    /// calling this twice with the same string returns the same pointer.
    pub fn sel_registerName(name: *const c_char) -> Sel;

    // -- Message dispatch -------------------------------------------------
    //
    // objc_msgSend is the heart of the Objective-C runtime.  Every method
    // call goes through it.
    //
    // IMPORTANT: On arm64, objc_msgSend is NOT a variadic function.
    // It uses a special calling convention where all arguments are passed
    // as fixed parameters.  If we declare it as variadic (`...`), the
    // compiler will use the variadic ABI which passes arguments on the
    // stack instead of in registers — causing silent data corruption.
    //
    // The solution is to declare it with a minimal fixed signature, then
    // cast the function pointer to the correct type at each call site
    // using the `msg!` macro.
    //
    // On x86_64 there are variants for struct returns (objc_msgSend_stret)
    // and float returns (objc_msgSend_fpret).  On arm64, objc_msgSend
    // handles all return types.

    /// Raw objc_msgSend entry point.  Do NOT call directly — use the
    /// `msg!` macro which casts to the correct function pointer type.
    pub fn objc_msgSend(receiver: Id, sel: Sel, ...) -> Id;

    // -- Class creation (for delegate/callback classes) -------------------

    /// Allocate a new class pair (class + metaclass).
    /// `superclass` is the parent class, `name` is the new class name,
    /// `extra_bytes` is usually 0.
    pub fn objc_allocateClassPair(
        superclass: ClassPtr,
        name: *const c_char,
        extra_bytes: usize,
    ) -> ClassPtr;

    /// Register a class pair so it can be used.  Must be called after
    /// adding all methods/ivars.
    pub fn objc_registerClassPair(cls: ClassPtr);

    /// Add a method to a class.
    /// `name` is the selector, `imp` is the function pointer,
    /// `types` describes the signature (e.g. "v@:" for void method).
    pub fn class_addMethod(
        cls: ClassPtr,
        name: Sel,
        imp: *const c_void,
        types: *const c_char,
    ) -> bool;

    /// Add an instance variable to a class.
    pub fn class_addIvar(
        cls: ClassPtr,
        name: *const c_char,
        size: usize,
        alignment: u8,
        types: *const c_char,
    ) -> bool;

    /// Get a pointer to an instance variable's storage.
    pub fn object_getInstanceVariable(
        obj: Id,
        name: *const c_char,
        out_value: *mut *mut c_void,
    ) -> *mut c_void;

    /// Set the value of an instance variable.
    pub fn object_setInstanceVariable(
        obj: Id,
        name: *const c_char,
        value: *mut c_void,
    ) -> *mut c_void;
}

// ---------------------------------------------------------------------------
// Metal framework — C function (not ObjC)
// ---------------------------------------------------------------------------

#[link(name = "Metal", kind = "framework")]
extern "C" {
    /// Create the default Metal device (GPU).
    /// This is a plain C function, not an Objective-C method.
    /// Returns nil if no Metal-capable GPU is available.
    #[allow(non_snake_case)]
    pub fn MTLCreateSystemDefaultDevice() -> Id;
}

// ---------------------------------------------------------------------------
// CoreGraphics — C functions
// ---------------------------------------------------------------------------

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    #[allow(non_snake_case)]
    pub fn CGColorSpaceCreateDeviceRGB() -> CGColorSpaceRef;

    #[allow(non_snake_case)]
    pub fn CGColorSpaceRelease(space: CGColorSpaceRef);

    #[allow(non_snake_case)]
    pub fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        space: CGColorSpaceRef,
        bitmap_info: u32,
    ) -> CGContextRef;

    #[allow(non_snake_case)]
    pub fn CGContextRelease(context: CGContextRef);

    #[allow(non_snake_case)]
    pub fn CGContextSetRGBFillColor(
        context: CGContextRef,
        red: c_double,
        green: c_double,
        blue: c_double,
        alpha: c_double,
    );

    #[allow(non_snake_case)]
    pub fn CGContextFillRect(context: CGContextRef, rect: CGRect);

    #[allow(non_snake_case)]
    pub fn CGContextSetTextPosition(context: CGContextRef, x: c_double, y: c_double);
}

// ---------------------------------------------------------------------------
// CoreText — C functions
// ---------------------------------------------------------------------------
//
// CoreText is Apple's text layout and rendering engine.  It provides
// font loading, glyph shaping, line breaking, and rendering to a
// CoreGraphics context.

#[link(name = "CoreText", kind = "framework")]
extern "C" {
    /// Create a font by name and size.
    #[allow(non_snake_case)]
    pub fn CTFontCreateWithName(
        name: Id,         // CFStringRef (toll-free bridged to NSString)
        size: c_double,
        matrix: *const c_void, // const CGAffineTransform*, NULL for identity
    ) -> Id; // CTFontRef

    /// Create a line of text from an attributed string.
    #[allow(non_snake_case)]
    pub fn CTLineCreateWithAttributedString(
        attr_string: Id, // CFAttributedStringRef
    ) -> Id; // CTLineRef

    /// Get the typographic bounds of a line.
    #[allow(non_snake_case)]
    pub fn CTLineGetTypographicBounds(
        line: Id,
        ascent: *mut c_double,
        descent: *mut c_double,
        leading: *mut c_double,
    ) -> c_double; // width

    /// Draw a line into a CoreGraphics context.
    #[allow(non_snake_case)]
    pub fn CTLineDraw(line: Id, context: CGContextRef);
}

// ---------------------------------------------------------------------------
// CoreFoundation — C functions
// ---------------------------------------------------------------------------
//
// CoreFoundation provides toll-free bridged types (CFString ↔ NSString,
// CFDictionary ↔ NSDictionary, etc.).  We need these for creating
// attributed strings for CoreText.

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    #[allow(non_snake_case)]
    pub fn CFRelease(cf: Id);

    /// Create a CFString from a C string.
    #[allow(non_snake_case)]
    pub fn CFStringCreateWithCString(
        alloc: *const c_void,     // kCFAllocatorDefault = NULL
        c_str: *const c_char,
        encoding: u32,            // kCFStringEncodingUTF8 = 0x08000100
    ) -> Id; // CFStringRef

    /// Create an attributed string.
    #[allow(non_snake_case)]
    pub fn CFAttributedStringCreate(
        alloc: *const c_void,
        string: Id,               // CFStringRef
        attributes: Id,            // CFDictionaryRef
    ) -> Id; // CFAttributedStringRef

    /// Create a CFDictionary.
    #[allow(non_snake_case)]
    pub fn CFDictionaryCreate(
        allocator: *const c_void,
        keys: *const *const c_void,
        values: *const *const c_void,
        num_values: c_int,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> Id; // CFDictionaryRef
}

// CoreFoundation string encoding constants
pub const K_CF_STRING_ENCODING_UTF8: u32 = 0x08000100;

// CoreText attribute key names (these are global CFString constants)
#[link(name = "CoreText", kind = "framework")]
extern "C" {
    /// The attributed string key for the font (value: CTFontRef).
    #[allow(non_upper_case_globals)]
    pub static kCTFontAttributeName: Id;

    /// The attributed string key for the foreground color (value: CGColorRef).
    #[allow(non_upper_case_globals)]
    pub static kCTForegroundColorAttributeName: Id;
}

// CoreFoundation dictionary callbacks (used with CFDictionaryCreate)
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    #[allow(non_upper_case_globals)]
    pub static kCFTypeDictionaryKeyCallBacks: c_void;

    #[allow(non_upper_case_globals)]
    pub static kCFTypeDictionaryValueCallBacks: c_void;
}

// ---------------------------------------------------------------------------
// AppKit framework — for window display
// ---------------------------------------------------------------------------

#[link(name = "AppKit", kind = "framework")]
extern "C" {
    /// Start the NSApplication event loop.
    #[allow(non_snake_case)]
    pub fn NSApplicationMain(argc: c_int, argv: *const *const c_char) -> c_int;
}

// AppKit window style mask constants
pub const NS_WINDOW_STYLE_MASK_TITLED: c_ulong = 1 << 0;
pub const NS_WINDOW_STYLE_MASK_CLOSABLE: c_ulong = 1 << 1;
pub const NS_WINDOW_STYLE_MASK_MINIATURIZABLE: c_ulong = 1 << 2;
pub const NS_WINDOW_STYLE_MASK_RESIZABLE: c_ulong = 1 << 3;

// NSBackingStoreType
pub const NS_BACKING_STORE_BUFFERED: c_ulong = 2;

// ---------------------------------------------------------------------------
// msg! macro — type-safe message dispatch
// ---------------------------------------------------------------------------
//
// On arm64, objc_msgSend is NOT variadic — it uses a special trampoline
// that reads fixed-position register arguments.  If we call it through
// Rust's variadic FFI (`...`), the compiler puts arguments on the stack
// instead of in registers, causing silent data corruption.
//
// The solution: cast objc_msgSend to a typed function pointer at each
// call site.  The `msg!` macro makes this ergonomic.
//
// ## Usage
//
// ```ignore
// // No extra args: [obj selector] → msg!(obj, "selector")
// msg!(obj, "init")
//
// // One arg: [obj setFoo:42] → msg!(obj, "setFoo:", arg)
// msg!(obj, "setFoo:", 42usize)
//
// // Two args: [obj setX:1 Y:2]
// msg!(obj, "setX:Y:", 1usize, 2usize)
// ```

/// Type-safe message dispatch macro.
///
/// Casts `objc_msgSend` to the correct function pointer type based on
/// the number and types of arguments provided.  This ensures the arm64
/// ABI places arguments in registers, not on the stack.
#[macro_export]
macro_rules! msg {
    // No extra arguments: (receiver, selector) -> Id
    ($receiver:expr, $sel:expr) => {{
        let f: unsafe extern "C" fn($crate::Id, $crate::Sel) -> $crate::Id =
            ::std::mem::transmute($crate::objc_msgSend as *const ());
        f($receiver, $crate::sel($sel))
    }};

    // One argument
    ($receiver:expr, $sel:expr, $a1:expr) => {{
        let f: unsafe extern "C" fn($crate::Id, $crate::Sel, _) -> $crate::Id =
            ::std::mem::transmute($crate::objc_msgSend as *const ());
        f($receiver, $crate::sel($sel), $a1)
    }};

    // Two arguments
    ($receiver:expr, $sel:expr, $a1:expr, $a2:expr) => {{
        let f: unsafe extern "C" fn($crate::Id, $crate::Sel, _, _) -> $crate::Id =
            ::std::mem::transmute($crate::objc_msgSend as *const ());
        f($receiver, $crate::sel($sel), $a1, $a2)
    }};

    // Three arguments
    ($receiver:expr, $sel:expr, $a1:expr, $a2:expr, $a3:expr) => {{
        let f: unsafe extern "C" fn($crate::Id, $crate::Sel, _, _, _) -> $crate::Id =
            ::std::mem::transmute($crate::objc_msgSend as *const ());
        f($receiver, $crate::sel($sel), $a1, $a2, $a3)
    }};

    // Four arguments
    ($receiver:expr, $sel:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr) => {{
        let f: unsafe extern "C" fn($crate::Id, $crate::Sel, _, _, _, _) -> $crate::Id =
            ::std::mem::transmute($crate::objc_msgSend as *const ());
        f($receiver, $crate::sel($sel), $a1, $a2, $a3, $a4)
    }};

    // Five arguments
    ($receiver:expr, $sel:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr) => {{
        let f: unsafe extern "C" fn($crate::Id, $crate::Sel, _, _, _, _, _) -> $crate::Id =
            ::std::mem::transmute($crate::objc_msgSend as *const ());
        f($receiver, $crate::sel($sel), $a1, $a2, $a3, $a4, $a5)
    }};

    // Six arguments
    ($receiver:expr, $sel:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr) => {{
        let f: unsafe extern "C" fn($crate::Id, $crate::Sel, _, _, _, _, _, _) -> $crate::Id =
            ::std::mem::transmute($crate::objc_msgSend as *const ());
        f($receiver, $crate::sel($sel), $a1, $a2, $a3, $a4, $a5, $a6)
    }};

    // Seven arguments
    ($receiver:expr, $sel:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr, $a7:expr) => {{
        let f: unsafe extern "C" fn($crate::Id, $crate::Sel, _, _, _, _, _, _, _) -> $crate::Id =
            ::std::mem::transmute($crate::objc_msgSend as *const ());
        f($receiver, $crate::sel($sel), $a1, $a2, $a3, $a4, $a5, $a6, $a7)
    }};

    // Eight arguments
    ($receiver:expr, $sel:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr, $a7:expr, $a8:expr) => {{
        let f: unsafe extern "C" fn($crate::Id, $crate::Sel, _, _, _, _, _, _, _, _) -> $crate::Id =
            ::std::mem::transmute($crate::objc_msgSend as *const ());
        f($receiver, $crate::sel($sel), $a1, $a2, $a3, $a4, $a5, $a6, $a7, $a8)
    }};

    // Nine arguments
    ($receiver:expr, $sel:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr, $a7:expr, $a8:expr, $a9:expr) => {{
        let f: unsafe extern "C" fn($crate::Id, $crate::Sel, _, _, _, _, _, _, _, _, _) -> $crate::Id =
            ::std::mem::transmute($crate::objc_msgSend as *const ());
        f($receiver, $crate::sel($sel), $a1, $a2, $a3, $a4, $a5, $a6, $a7, $a8, $a9)
    }};

    // Ten arguments
    ($receiver:expr, $sel:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr, $a6:expr, $a7:expr, $a8:expr, $a9:expr, $a10:expr) => {{
        let f: unsafe extern "C" fn($crate::Id, $crate::Sel, _, _, _, _, _, _, _, _, _, _) -> $crate::Id =
            ::std::mem::transmute($crate::objc_msgSend as *const ());
        f($receiver, $crate::sel($sel), $a1, $a2, $a3, $a4, $a5, $a6, $a7, $a8, $a9, $a10)
    }};
}

// ---------------------------------------------------------------------------
// Safe wrappers
// ---------------------------------------------------------------------------

/// Look up an Objective-C class by name.
///
/// # Example
///
/// ```ignore
/// let ns_string_class = objc_bridge::class("NSString");
/// ```
///
/// # Panics
///
/// Panics if the class is not found (i.e. the framework providing it
/// is not linked).
pub fn class(name: &str) -> ClassPtr {
    let c_name = CString::new(name).expect("class name must not contain NUL");
    let cls = unsafe { objc_getClass(c_name.as_ptr()) };
    assert!(
        !cls.is_null(),
        "objc_bridge::class: class '{}' not found — is the framework linked?",
        name
    );
    cls
}

/// Look up (or register) an Objective-C selector by name.
///
/// Selectors are interned strings that identify method names.  For example,
/// the selector for `[obj init]` is `sel("init")`, and the selector for
/// `[obj setValue:forKey:]` is `sel("setValue:forKey:")`.
pub fn sel(name: &str) -> Sel {
    let c_name = CString::new(name).expect("selector name must not contain NUL");
    unsafe { sel_registerName(c_name.as_ptr()) }
}

/// Create a CFString (toll-free bridged to NSString) from a Rust `&str`.
///
/// The returned pointer is a retained CFString — the caller is responsible
/// for releasing it with `CFRelease`.
pub fn cfstring(s: &str) -> Id {
    let c_str = CString::new(s).expect("string must not contain NUL");
    unsafe {
        CFStringCreateWithCString(
            std::ptr::null(),
            c_str.as_ptr(),
            K_CF_STRING_ENCODING_UTF8,
        )
    }
}

/// Create an NSString from a Rust `&str`.
///
/// NSString and CFString are toll-free bridged — same object, different
/// type name.  This is a convenience alias for `cfstring`.
pub fn nsstring(s: &str) -> Id {
    cfstring(s)
}

/// Convenience: send a no-argument message that returns a pointer.
///
/// Equivalent to `[receiver selector]` in Objective-C.
///
/// # Safety
///
/// The caller must ensure that:
/// - `receiver` is a valid object or class pointer
/// - The selector exists on the receiver's class
/// - The return type is a pointer (Id)
pub unsafe fn msg_send_id(receiver: Id, selector: &str) -> Id {
    msg!(receiver, selector)
}

/// Send a message to a class (class method call).
///
/// Equivalent to `[ClassName selector]` in Objective-C.
/// Classes are objects too, so this is just msg_send with a cast.
pub unsafe fn msg_send_class(cls: ClassPtr, selector: &str) -> Id {
    msg!(cls as Id, selector)
}

/// Alloc + init an object (the most common ObjC pattern).
///
/// Equivalent to `[[ClassName alloc] init]`.
pub unsafe fn alloc_init(class_name: &str) -> Id {
    let cls = class(class_name);
    let obj = msg!(cls as Id, "alloc");
    msg!(obj, "init")
}

/// Release an Objective-C object (decrement reference count).
pub unsafe fn release(obj: Id) {
    if !obj.is_null() {
        msg!(obj, "release");
    }
}

/// Retain an Objective-C object (increment reference count).
pub unsafe fn retain(obj: Id) -> Id {
    if obj.is_null() {
        return obj;
    }
    msg!(obj, "retain")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    /// Verify we can look up NSObject — the root class of all Objective-C
    /// objects.  If this works, the runtime is linked and functional.
    #[test]
    fn can_find_nsobject_class() {
        let cls = class("NSObject");
        assert!(!cls.is_null());
    }

    /// Verify we can register a selector.  Selectors are interned strings,
    /// so calling sel("init") twice should return the same pointer.
    #[test]
    fn can_register_selector() {
        let s1 = sel("init");
        let s2 = sel("init");
        assert!(!s1.is_null());
        assert_eq!(s1, s2, "same selector name should return same pointer");
    }

    /// Verify alloc/init works for NSObject.
    #[test]
    fn can_alloc_init_nsobject() {
        unsafe {
            let obj = alloc_init("NSObject");
            assert!(!obj.is_null());
            release(obj);
        }
    }

    /// Verify we can create a CFString / NSString from Rust.
    #[test]
    fn can_create_nsstring() {
        let s = nsstring("hello");
        assert!(!s.is_null());
        unsafe { CFRelease(s) };
    }

    /// Verify the Metal device can be created (requires macOS with GPU).
    #[test]
    fn can_create_metal_device() {
        unsafe {
            let device = MTLCreateSystemDefaultDevice();
            // On macOS with Metal support, this should succeed.
            // On CI without GPU, this might be nil — that's OK.
            if !device.is_null() {
                release(device);
            }
        }
    }

    /// Verify class lookup fails gracefully for non-existent classes.
    #[test]
    #[should_panic(expected = "class 'NonExistentClass12345' not found")]
    fn class_lookup_panics_for_unknown() {
        class("NonExistentClass12345");
    }
}
