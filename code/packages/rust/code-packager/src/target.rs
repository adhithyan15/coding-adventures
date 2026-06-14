//! Compilation target descriptor.
//!
//! A `Target` is an immutable triple of `(arch, os, binary_format)` that tells
//! the packager how to wrap native bytes into the correct binary container.
//!
//! ## Analogy
//!
//! Think of the `Target` as the *address label* on a package:
//!
//! - `arch` says what CPU the code runs on (the language of the bytes inside).
//! - `os`   says what operating system loads the file (who opens the package).
//! - `binary_format` says the *container format* (ELF box, Mach-O box, PE box…).
//!
//! ## Supported triples
//!
//! ```text
//! arch      │ os      │ binary_format │ factory method
//! ──────────┼─────────┼───────────────┼───────────────────
//! x86_64    │ linux   │ elf64         │ linux_x64()
//! arm64     │ linux   │ elf64         │ linux_arm64()
//! x86_64    │ macos   │ macho64       │ macos_x64()
//! arm64     │ macos   │ macho64       │ macos_arm64()
//! x86_64    │ windows │ pe            │ windows_x64()
//! wasm32    │ none    │ wasm          │ wasm()
//! <arch>    │ none    │ raw           │ raw(arch)
//! i4004     │ none    │ intel_hex     │ intel_4004()
//! i8008     │ none    │ intel_hex     │ intel_8008()
//! ```

/// Immutable description of a compilation target.
///
/// All three fields are plain `String` values so that callers can construct
/// custom targets without needing an exhaustive enum.
///
/// # Examples
///
/// ```rust
/// use code_packager::Target;
///
/// let t = Target::linux_x64();
/// assert_eq!(t.arch, "x86_64");
/// assert_eq!(t.os, "linux");
/// assert_eq!(t.binary_format, "elf64");
/// println!("{t}"); // "x86_64-linux-elf64"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Target {
    /// The instruction-set architecture, e.g. `"x86_64"`, `"arm64"`, `"wasm32"`.
    pub arch: String,
    /// The operating system (or `"none"` for bare-metal / WASM targets).
    pub os: String,
    /// The binary container format: `"elf64"`, `"macho64"`, `"pe"`, `"wasm"`,
    /// `"raw"`, or `"intel_hex"`.
    pub binary_format: String,
}

impl Target {
    // ── Private constructor ────────────────────────────────────────────────────

    /// Internal helper: construct from string slices.
    fn new(arch: &str, os: &str, binary_format: &str) -> Self {
        Self {
            arch: arch.to_string(),
            os: os.to_string(),
            binary_format: binary_format.to_string(),
        }
    }

    // ── Linux targets (ELF64) ─────────────────────────────────────────────────

    /// Linux on AMD64 / x86-64. Produces an ELF64 executable.
    ///
    /// This is the most common server target (EC2, GCP, Azure).
    pub fn linux_x64() -> Self {
        Self::new("x86_64", "linux", "elf64")
    }

    /// Linux on AArch64 / ARM64. Produces an ELF64 executable.
    ///
    /// Covers Raspberry Pi 4 (64-bit), AWS Graviton, Apple M1 running Linux.
    pub fn linux_arm64() -> Self {
        Self::new("arm64", "linux", "elf64")
    }

    // ── macOS targets (Mach-O 64) ─────────────────────────────────────────────

    /// macOS on Intel x86-64. Produces a Mach-O 64-bit executable.
    pub fn macos_x64() -> Self {
        Self::new("x86_64", "macos", "macho64")
    }

    /// macOS on Apple Silicon (arm64). Produces a Mach-O 64-bit executable.
    ///
    /// Apple calls this architecture "arm64" in its toolchain even though the
    /// ISA is technically AArch64.
    pub fn macos_arm64() -> Self {
        Self::new("arm64", "macos", "macho64")
    }

    // ── Windows target (PE32+) ────────────────────────────────────────────────

    /// Windows on x86-64. Produces a PE32+ executable (`.exe`).
    ///
    /// PE32+ is the 64-bit variant of the Portable Executable format used by
    /// Windows NT and all its descendants (XP, 7, 10, 11, Server).
    pub fn windows_x64() -> Self {
        Self::new("x86_64", "windows", "pe")
    }

    // ── WebAssembly target ────────────────────────────────────────────────────

    /// WebAssembly (wasm32). Produces a `.wasm` module.
    ///
    /// The WASM target has no OS: it runs inside a host runtime (browser,
    /// Wasmtime, Wasmer, Node.js, etc.).
    pub fn wasm() -> Self {
        Self::new("wasm32", "none", "wasm")
    }

    // ── Raw binary target ─────────────────────────────────────────────────────

    /// Bare-metal raw binary for the given architecture.
    ///
    /// No container format is applied; the bytes are written verbatim.
    /// Useful for bootloaders, firmware images, and ROM dumps.
    ///
    /// # Example
    ///
    /// ```rust
    /// use code_packager::Target;
    /// let t = Target::raw("x86_64");
    /// assert_eq!(t.binary_format, "raw");
    /// ```
    pub fn raw(arch: &str) -> Self {
        Self::new(arch, "none", "raw")
    }

    // ── Intel HEX targets ─────────────────────────────────────────────────────

    /// Intel 4004 (4-bit CPU, 1971). Produces an Intel HEX file.
    ///
    /// The i4004 was Intel's first microprocessor. Its 4-bit ALU and 12-bit
    /// address space make it suitable for simple calculators and controllers.
    /// Intel HEX is a text format that encodes binary data as ASCII records,
    /// making it easy to transfer over serial links and program into EPROMs.
    pub fn intel_4004() -> Self {
        Self::new("i4004", "none", "intel_hex")
    }

    /// Intel 8008 (8-bit CPU, 1972). Produces an Intel HEX file.
    ///
    /// The i8008 was Intel's first 8-bit processor, a direct ancestor of the
    /// 8080 and the x86 family. It addressed 16 KiB of memory.
    pub fn intel_8008() -> Self {
        Self::new("i8008", "none", "intel_hex")
    }
}

// ── Display ───────────────────────────────────────────────────────────────────

impl std::fmt::Display for Target {
    /// Formats as `"arch-os-binary_format"`, e.g. `"x86_64-linux-elf64"`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}-{}", self.arch, self.os, self.binary_format)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: linux_x64 fields
    #[test]
    fn linux_x64_fields() {
        let t = Target::linux_x64();
        assert_eq!(t.arch, "x86_64");
        assert_eq!(t.os, "linux");
        assert_eq!(t.binary_format, "elf64");
    }

    // Test 2: linux_arm64 fields
    #[test]
    fn linux_arm64_fields() {
        let t = Target::linux_arm64();
        assert_eq!(t.arch, "arm64");
        assert_eq!(t.os, "linux");
        assert_eq!(t.binary_format, "elf64");
    }

    // Test 3: macos_x64 fields
    #[test]
    fn macos_x64_fields() {
        let t = Target::macos_x64();
        assert_eq!(t.arch, "x86_64");
        assert_eq!(t.os, "macos");
        assert_eq!(t.binary_format, "macho64");
    }

    // Test 4: macos_arm64 fields
    #[test]
    fn macos_arm64_fields() {
        let t = Target::macos_arm64();
        assert_eq!(t.arch, "arm64");
        assert_eq!(t.os, "macos");
        assert_eq!(t.binary_format, "macho64");
    }

    // Test 5: windows_x64 fields
    #[test]
    fn windows_x64_fields() {
        let t = Target::windows_x64();
        assert_eq!(t.arch, "x86_64");
        assert_eq!(t.os, "windows");
        assert_eq!(t.binary_format, "pe");
    }

    // Test 6: wasm fields
    #[test]
    fn wasm_fields() {
        let t = Target::wasm();
        assert_eq!(t.arch, "wasm32");
        assert_eq!(t.os, "none");
        assert_eq!(t.binary_format, "wasm");
    }

    // Test 7: raw(arch) sets binary_format to "raw"
    #[test]
    fn raw_target() {
        let t = Target::raw("avr");
        assert_eq!(t.arch, "avr");
        assert_eq!(t.os, "none");
        assert_eq!(t.binary_format, "raw");
    }

    // Test 8: intel_4004 fields
    #[test]
    fn intel_4004_fields() {
        let t = Target::intel_4004();
        assert_eq!(t.arch, "i4004");
        assert_eq!(t.os, "none");
        assert_eq!(t.binary_format, "intel_hex");
    }

    // Test 9: intel_8008 fields
    #[test]
    fn intel_8008_fields() {
        let t = Target::intel_8008();
        assert_eq!(t.arch, "i8008");
        assert_eq!(t.binary_format, "intel_hex");
    }

    // Test 10: Display format
    #[test]
    fn display_format() {
        assert_eq!(Target::linux_x64().to_string(), "x86_64-linux-elf64");
        assert_eq!(Target::wasm().to_string(), "wasm32-none-wasm");
        assert_eq!(Target::intel_4004().to_string(), "i4004-none-intel_hex");
        assert_eq!(Target::windows_x64().to_string(), "x86_64-windows-pe");
    }

    // Test 11: Clone and PartialEq
    #[test]
    fn clone_and_eq() {
        let a = Target::macos_arm64();
        let b = a.clone();
        assert_eq!(a, b);
        assert_ne!(a, Target::macos_x64());
    }
}
