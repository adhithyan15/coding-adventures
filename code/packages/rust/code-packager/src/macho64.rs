//! Mach-O 64-bit executable packager.
//!
//! Produces a minimal, single-section Mach-O 64-bit binary that macOS can
//! execute directly.  The output uses `LC_UNIXTHREAD` to specify the
//! entry point as an initial thread-state PC value, which lets the
//! kernel start the program **without** invoking `dyld`.
//!
//! ## Why `LC_UNIXTHREAD` and not `LC_MAIN`?
//!
//! `LC_MAIN` is the modern load command and is what real toolchains
//! (clang, ld) emit.  But `LC_MAIN` requires `dyld` to set up the
//! C-runtime entry frame (argc/argv/envp) and route the program's
//! return value through `exit()`.  Without `LC_LOAD_DYLINKER` pointing
//! at `/usr/lib/dyld`, modern macOS rejects the binary at exec time
//! with `ENOEXEC` ("Bad executable").
//!
//! For self-contained binaries that don't link any shared libraries —
//! the typical AOT-compiled output of this stack — `LC_UNIXTHREAD` is
//! the right choice: the kernel sets a fresh thread state, jumps to
//! the supplied PC, and lets the program manage itself (typically
//! ending with a direct `SVC` to the `exit` syscall).
//!
//! Trade-off: programs emitted this way must terminate with an
//! explicit `exit` syscall — they cannot just `ret` because there is
//! no caller to return to.
//!
//! ## Mach-O file layout (arm64)
//!
//! ```text
//! Offset │ Size │ Content
//! ───────┼──────┼────────────────────────────────────────────────────────────
//!      0 │  32  │ mach_header_64
//!     32 │  72  │ LC_SEGMENT_64 header (for the __TEXT segment)
//!    104 │  80  │ section_64 (for the __text section)
//!    184 │ 288  │ LC_UNIXTHREAD (16-byte header + 272-byte ARM64 thread state)
//!    472 │   N  │ native_bytes (the machine code)
//! ```
//!
//! For x86-64, the thread state is 168 bytes instead of 272, so the
//! LC_UNIXTHREAD command is 184 bytes and the header total is 368.
//!
//! ## mach_header_64 (32 bytes, at offset 0)
//!
//! ```text
//! Field      │ Size │ Value
//! ───────────┼──────┼───────────────────────────────────────────────────────
//! magic      │  4   │ 0xCFFAEDFE (MH_MAGIC_64, reversed for LE)
//! cputype    │  4   │ 0x01000007 (x86_64) | 0x0100000C (arm64)
//! cpusubtype │  4   │ 3 (x86_64 all) | 0 (arm64 all)
//! filetype   │  4   │ 2 = MH_EXECUTE
//! ncmds      │  4   │ 2 (two load commands: LC_SEGMENT_64 + LC_MAIN)
//! sizeofcmds │  4   │ 256 (72 + 80 + 24 = 176 … but we compute dynamically)
//! flags      │  4   │ 0
//! reserved   │  4   │ 0 (only in 64-bit header)
//! ```
//!
//! ## LC_SEGMENT_64 (72 bytes, at offset 32)
//!
//! ```text
//! Field    │ Size │ Value
//! ─────────┼──────┼─────────────────────────────────────────────────────────
//! cmd      │  4   │ 0x19 = LC_SEGMENT_64
//! cmdsize  │  4   │ 152 (72 header + 80 section_64)
//! segname  │ 16   │ "__TEXT\0\0\0\0\0\0\0\0\0\0"
//! vmaddr   │  8   │ load_address (default 0x100000000)
//! vmsize   │  8   │ HEADER_TOTAL + len(code)
//! fileoff  │  8   │ 0 (segment starts at file beginning)
//! filesize │  8   │ HEADER_TOTAL + len(code)
//! maxprot  │  4   │ 7 (rwx — maximum allowed protection)
//! initprot │  4   │ 5 (r+x — initial protection at load time)
//! nsects   │  4   │ 1 (one section: __text)
//! flags    │  4   │ 0
//! ```
//!
//! ## section_64 (80 bytes, at offset 104)
//!
//! ```text
//! Field     │ Size │ Value
//! ──────────┼──────┼────────────────────────────────────────────────────────
//! sectname  │ 16   │ "__text\0\0\0\0\0\0\0\0\0\0"
//! segname   │ 16   │ "__TEXT\0\0\0\0\0\0\0\0\0\0"
//! addr      │  8   │ load_address + HEADER_TOTAL
//! size      │  8   │ len(code)
//! offset    │  4   │ HEADER_TOTAL (file offset of the code)
//! align     │  4   │ 4 (2^4 = 16-byte alignment)
//! reloff    │  4   │ 0 (no relocations)
//! nreloc    │  4   │ 0
//! flags     │  4   │ 0x80000400 (S_ATTR_PURE_INSTRUCTIONS | S_ATTR_SOME_INSTRUCTIONS)
//! reserved1 │  4   │ 0
//! reserved2 │  4   │ 0
//! reserved3 │  4   │ 0
//! ```
//!
//! ## LC_UNIXTHREAD (variable size, at offset 184)
//!
//! ```text
//! Field    │ Size │ Value
//! ─────────┼──────┼─────────────────────────────────────────────────────────
//! cmd      │  4   │ 0x05 = LC_UNIXTHREAD
//! cmdsize  │  4   │ 288 (arm64) or 184 (x86_64)
//! flavor   │  4   │ 6 (ARM_THREAD_STATE64) or 4 (X86_THREAD_STATE64)
//! count    │  4   │ 68 (arm64) or 42 (x86_64) — number of u32s in state
//! state    │  N   │ 272 bytes (arm64) or 168 bytes (x86_64), all zero except PC
//! ```
//!
//! ### ARM64 thread state (272 bytes)
//!
//! ```text
//! Offset │ Size │ Field
//! ───────┼──────┼─────────────────────────────────────────────────────────
//!      0 │ 232  │ x[0..29] — 29 GPRs
//!    232 │   8  │ fp (x29)
//!    240 │   8  │ lr (x30)
//!    248 │   8  │ sp
//!    256 │   8  │ pc        ← absolute load address of the entry point
//!    264 │   4  │ cpsr
//!    268 │   4  │ pad
//! ```
//!
//! ### x86-64 thread state (168 bytes)
//!
//! ```text
//! Offset │ Size │ Field (selected)
//! ───────┼──────┼─────────────────────────────────────────────────────────
//!    128 │   8  │ rip       ← absolute load address of the entry point
//! ```

use crate::artifact::CodeArtifact;
use crate::errors::PackagerError;
use crate::target::Target;

// ── Layout constants ──────────────────────────────────────────────────────────

// mach_header_64 size.
const MACH_HEADER_SIZE: u64 = 32;
// LC_SEGMENT_64 header (without sections) = 72 bytes.
const LC_SEGMENT_SIZE: u64 = 72;
// section_64 struct = 80 bytes.
const SECTION_SIZE: u64 = 80;

// LC_UNIXTHREAD has a 16-byte header (cmd, cmdsize, flavor, count) plus
// the architecture-specific thread state.  Total command size depends on
// the arch:
//   arm64:   16 + 272 = 288
//   x86-64:  16 + 168 = 184
const LC_UNIXTHREAD_HEADER_SIZE: u64 = 16;
const ARM_THREAD_STATE64_SIZE:   u64 = 272;
const X86_THREAD_STATE64_SIZE:   u64 = 168;

// cmdsize of the LC_SEGMENT_64 command = header + one section_64.
const LC_SEGMENT_CMDSIZE: u32 = (LC_SEGMENT_SIZE + SECTION_SIZE) as u32; // 152

// ── CPU type/subtype constants ────────────────────────────────────────────────

// CPU_TYPE_X86_64 = CPU_TYPE_I386 | CPU_ARCH_ABI64 = 7 | 0x01000000.
const CPU_TYPE_X86_64: u32 = 0x01000007;
// CPU_TYPE_ARM64 = CPU_TYPE_ARM | CPU_ARCH_ABI64 = 12 | 0x01000000.
const CPU_TYPE_ARM64: u32 = 0x0100000C;

// CPU_SUBTYPE_X86_ALL = 3.
const CPU_SUBTYPE_X86_ALL: u32 = 3;
// CPU_SUBTYPE_ARM_ALL = 0.
const CPU_SUBTYPE_ARM_ALL: u32 = 0;

// ── Load command identifiers ──────────────────────────────────────────────────

// LC_SEGMENT_64 = 0x19.
const LC_SEGMENT_64: u32 = 0x19;
// LC_UNIXTHREAD = 0x05.  Specifies an initial thread state (no dyld).
const LC_UNIXTHREAD: u32 = 0x05;
// LC_CODE_SIGNATURE = 0x1D.  Points to a code-signature blob in __LINKEDIT.
const LC_CODE_SIGNATURE: u32 = 0x1D;
// LC_CODE_SIGNATURE command size = 16 bytes (cmd, cmdsize, dataoff, datasize).
const LC_CODE_SIGNATURE_SIZE: u64 = 16;

// LC_BUILD_VERSION = 0x32.  Records the min OS version + SDK; modern
// macOS / Apple Silicon will SIGKILL binaries that omit it.
const LC_BUILD_VERSION: u32 = 0x32;
// LC_BUILD_VERSION command size = 24 bytes when there are no embedded tool versions.
const LC_BUILD_VERSION_SIZE: u64 = 24;
// PLATFORM_MACOS in <mach-o/loader.h>.
const PLATFORM_MACOS: u32 = 1;
/// Min OS version we declare.
///
/// Sequoia (15.x) and Tahoe (26.x) reject binaries that declare a min OS
/// older than the current major version.  We declare a recent macOS
/// (15.0 / Sequoia) which has been stable since 2024 and is a
/// reasonable lower bound — newer machines accept it; older machines
/// won't be running these binaries anyway.
///
/// Encoded as `(major << 16) | (minor << 8) | patch`.
const MIN_OS_VERSION: u32 = 0x000F_0000; // 15.0.0
/// SDK version: same encoding.  Match minOS for simplicity.
const SDK_VERSION: u32 = 0x000F_0000;

// ── Code-signing constants (`<security/cs_blobs.h>`) ─────────────────────────
//
// All multi-byte fields in code-signature blobs are *big-endian*, a
// holdover from the format's PowerPC origins.

/// `CSMAGIC_EMBEDDED_SIGNATURE` — the SuperBlob wrapping all signature blobs.
const CSMAGIC_EMBEDDED_SIGNATURE: u32 = 0xfade_0cc0;
/// `CSMAGIC_CODEDIRECTORY` — a CodeDirectory blob (page hashes + metadata).
const CSMAGIC_CODEDIRECTORY:      u32 = 0xfade_0c02;
/// `CSSLOT_CODEDIRECTORY` — index slot for the primary CodeDirectory blob.
const CSSLOT_CODEDIRECTORY:       u32 = 0;

/// CodeDirectory version that includes the `execSeg*` fields.  Required
/// for ad-hoc-signed binaries on Apple Silicon.
const CODEDIR_VERSION_20400: u32 = 0x0002_0400;

/// `CS_ADHOC` flag — signed without a certificate chain.
const CS_ADHOC: u32 = 0x0000_0002;

/// `CS_HASHTYPE_SHA256` — page hash algorithm.
const CS_HASHTYPE_SHA256: u8 = 2;

/// `CS_EXECSEG_MAIN_BINARY` — flag in `execSegFlags`.
const CS_EXECSEG_MAIN_BINARY: u64 = 0x1;

/// Page size for code-signature hashing.  4096 on every platform we
/// produce binaries for.
const CODESIGN_PAGE_SIZE: u64 = 4096;
/// `log2(CODESIGN_PAGE_SIZE)` — stored as a single byte in the
/// CodeDirectory header.
const CODESIGN_PAGE_SHIFT: u8 = 12;

/// Identifier embedded in the code signature.  Apple's tooling derives
/// this from the binary's basename; for our generated executables a
/// fixed string is fine — the kernel only validates the structure.
const CODESIGN_IDENTIFIER: &str = "code-packager-adhoc";

/// Fixed-size portion of the CodeDirectory header (everything before
/// the identifier and hash slots).
const CODE_DIRECTORY_HEADER_SIZE: usize = 88;
/// SuperBlob magic + length + count = 12 bytes.
const SUPERBLOB_HEADER_SIZE: usize = 12;
/// One BlobIndex entry: type + offset = 8 bytes.
const BLOB_INDEX_SIZE: usize = 8;
/// SHA-256 produces 32 bytes per page.
const SHA256_HASH_SIZE: usize = 32;

// Thread-state flavor constants (`<mach/arm/thread_status.h>` and `<mach/i386/thread_status.h>`).
const ARM_THREAD_STATE64: u32 = 6;
const X86_THREAD_STATE64: u32 = 4;

// `count` field — number of `uint32_t` slots in the thread state.
const ARM_THREAD_STATE64_COUNT: u32 = (ARM_THREAD_STATE64_SIZE / 4) as u32; // 68
const X86_THREAD_STATE64_COUNT: u32 = (X86_THREAD_STATE64_SIZE / 4) as u32; // 42

// PC offset within each thread state.
const ARM_PC_OFFSET: usize = 256;
const X86_RIP_OFFSET: usize = 128;

// ── Section flags ─────────────────────────────────────────────────────────────

// S_ATTR_PURE_INSTRUCTIONS (0x80000000) | S_ATTR_SOME_INSTRUCTIONS (0x00000400).
const SECTION_FLAGS_CODE: u32 = 0x80000400;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Determine CPU type and subtype for the given target.
fn cpu_type(target: &Target) -> Result<(u32, u32), PackagerError> {
    match target.arch.as_str() {
        "x86_64" => Ok((CPU_TYPE_X86_64, CPU_SUBTYPE_X86_ALL)),
        "arm64" => Ok((CPU_TYPE_ARM64, CPU_SUBTYPE_ARM_ALL)),
        _ => Err(PackagerError::UnsupportedTarget(format!(
            "macho64 packager does not support arch={:?}",
            target.arch
        ))),
    }
}

/// Per-arch thread-state metadata used by `LC_UNIXTHREAD`.
struct ThreadStateInfo {
    flavor:   u32,
    count:    u32,
    /// Total bytes occupied by the thread state.
    state_size: u64,
    /// Byte offset within the thread state where the entry-point PC goes.
    pc_offset: usize,
}

fn thread_state_info(arch: &str) -> Result<ThreadStateInfo, PackagerError> {
    match arch {
        "arm64" => Ok(ThreadStateInfo {
            flavor: ARM_THREAD_STATE64, count: ARM_THREAD_STATE64_COUNT,
            state_size: ARM_THREAD_STATE64_SIZE, pc_offset: ARM_PC_OFFSET,
        }),
        "x86_64" => Ok(ThreadStateInfo {
            flavor: X86_THREAD_STATE64, count: X86_THREAD_STATE64_COUNT,
            state_size: X86_THREAD_STATE64_SIZE, pc_offset: X86_RIP_OFFSET,
        }),
        _ => Err(PackagerError::UnsupportedTarget(format!(
            "macho64 packager does not have thread state for arch={arch:?}"
        ))),
    }
}

/// Validate that the artifact is destined for a macOS Mach-O target.
fn validate(artifact: &CodeArtifact) -> Result<(), PackagerError> {
    if artifact.target.binary_format != "macho64" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "macho64 packager does not handle binary_format={:?}",
            artifact.target.binary_format
        )));
    }
    if artifact.target.os != "macos" {
        return Err(PackagerError::UnsupportedTarget(format!(
            "macho64 packager expects os=macos, got {:?}",
            artifact.target.os
        )));
    }
    Ok(())
}

/// Write a fixed-length segment or section name field (always 16 bytes, NUL-padded).
fn write_name(out: &mut Vec<u8>, name: &[u8]) {
    // The Mach-O spec requires exactly 16 bytes for segname/sectname fields.
    let mut buf = [0u8; 16];
    let n = name.len().min(16);
    buf[..n].copy_from_slice(&name[..n]);
    out.extend_from_slice(&buf);
}

// ===========================================================================
// Ad-hoc Mach-O code signature
// ===========================================================================
//
// Apple Silicon (Big Sur and later) requires every executable to carry a
// valid embedded code signature — the kernel will refuse to `exec()` an
// unsigned Mach-O with `ENOEXEC`.  Real toolchains (clang + ld) embed an
// "ad-hoc" signature for unsigned development binaries; we replicate the
// minimal subset of that machinery here.
//
// The signature is a `SuperBlob` containing one `CodeDirectory`:
//
//     SuperBlob {
//         magic        u32 BE  = 0xfade0cc0
//         length       u32 BE  = total size of SuperBlob
//         count        u32 BE  = 1 (one blob: the CodeDirectory)
//         BlobIndex {
//             type     u32 BE  = 0 (CSSLOT_CODEDIRECTORY)
//             offset   u32 BE  = SUPERBLOB_HEADER + BLOB_INDEX = 20
//         }
//         CodeDirectory {
//             magic           u32 BE = 0xfade0c02
//             length          u32 BE
//             version         u32 BE = 0x20400
//             flags           u32 BE = CS_ADHOC = 0x2
//             hashOffset      u32 BE = 88 + ident_len   (offset within CD)
//             identOffset     u32 BE = 88
//             nSpecialSlots   u32 BE = 0
//             nCodeSlots      u32 BE = ceil(codeLimit / 4096)
//             codeLimit       u32 BE = file offset where signature begins
//             hashSize        u8     = 32 (SHA-256)
//             hashType        u8     = 2  (CS_HASHTYPE_SHA256)
//             platform        u8     = 0
//             pageSize        u8     = 12 (log2 4096)
//             spare2          u32 BE = 0
//             scatterOffset   u32 BE = 0
//             teamOffset      u32 BE = 0
//             spare3          u32 BE = 0
//             codeLimit64     u64 BE = 0
//             execSegBase     u64 BE = file offset of __TEXT segment
//             execSegLimit    u64 BE = vmsize of __TEXT segment
//             execSegFlags    u64 BE = CS_EXECSEG_MAIN_BINARY = 1
//             [identifier]    null-terminated UTF-8 bytes
//             [hashes]        nCodeSlots × 32 bytes (SHA-256 of each page)
//         }
//     }
//
// All multi-byte fields are big-endian — a legacy of the format's
// PowerPC heritage.

/// Round up `n` to the next multiple of `align` (must be a power of two).
fn align_up(n: u64, align: u64) -> u64 {
    (n + align - 1) & !(align - 1)
}

/// Compute the size in bytes of the ad-hoc signature for a binary whose
/// hashable prefix is `code_limit` bytes long.
fn signature_size(code_limit: u64) -> usize {
    let n_pages = code_limit.div_ceil(CODESIGN_PAGE_SIZE) as usize;
    let ident_with_null = CODESIGN_IDENTIFIER.len() + 1;
    SUPERBLOB_HEADER_SIZE
        + BLOB_INDEX_SIZE
        + CODE_DIRECTORY_HEADER_SIZE
        + ident_with_null
        + n_pages * SHA256_HASH_SIZE
}

/// Build the ad-hoc SuperBlob + CodeDirectory + page hashes.
///
/// `unsigned_prefix` is the file content from offset 0 up to (but not
/// including) the signature data — i.e. the headers + code + any
/// page-padding zeros.  Its length must equal `code_limit`.
///
/// `exec_seg_base` and `exec_seg_limit` describe the `__TEXT` segment
/// (file offset and vmsize).  These go into the CodeDirectory's
/// `execSeg*` fields.
fn build_adhoc_signature(
    unsigned_prefix: &[u8],
    exec_seg_base: u64,
    exec_seg_limit: u64,
) -> Vec<u8> {
    use coding_adventures_sha256::sha256;

    let code_limit = unsigned_prefix.len() as u64;
    let n_pages = code_limit.div_ceil(CODESIGN_PAGE_SIZE) as usize;
    let ident_bytes = CODESIGN_IDENTIFIER.as_bytes();
    let ident_with_null_len = ident_bytes.len() + 1;

    // ----- 1. Hash each 4 KiB page of `unsigned_prefix` ----------------
    //
    // The last page is padded out to 4096 bytes with zeros for hashing
    // (this matches what `codesign` does: the file may end mid-page,
    // but the hash always covers a full page worth of input).
    let mut hashes: Vec<u8> = Vec::with_capacity(n_pages * SHA256_HASH_SIZE);
    for i in 0..n_pages {
        let start = i * CODESIGN_PAGE_SIZE as usize;
        let end   = ((i + 1) * CODESIGN_PAGE_SIZE as usize).min(unsigned_prefix.len());
        let page  = &unsigned_prefix[start..end];
        let h = if page.len() == CODESIGN_PAGE_SIZE as usize {
            sha256(page)
        } else {
            // Pad with zeros to a full page before hashing.
            let mut buf = vec![0u8; CODESIGN_PAGE_SIZE as usize];
            buf[..page.len()].copy_from_slice(page);
            sha256(&buf)
        };
        hashes.extend_from_slice(&h);
    }

    // ----- 2. Build the CodeDirectory -----------------------------------
    let cd_total_len = CODE_DIRECTORY_HEADER_SIZE
        + ident_with_null_len
        + hashes.len();
    let ident_offset_in_cd: u32 = CODE_DIRECTORY_HEADER_SIZE as u32;
    let hash_offset_in_cd:  u32 = ident_offset_in_cd + ident_with_null_len as u32;

    let mut cd: Vec<u8> = Vec::with_capacity(cd_total_len);
    cd.extend_from_slice(&CSMAGIC_CODEDIRECTORY.to_be_bytes());
    cd.extend_from_slice(&(cd_total_len as u32).to_be_bytes());
    cd.extend_from_slice(&CODEDIR_VERSION_20400.to_be_bytes());
    cd.extend_from_slice(&CS_ADHOC.to_be_bytes());
    cd.extend_from_slice(&hash_offset_in_cd.to_be_bytes());
    cd.extend_from_slice(&ident_offset_in_cd.to_be_bytes());
    cd.extend_from_slice(&0u32.to_be_bytes());                 // nSpecialSlots
    cd.extend_from_slice(&(n_pages as u32).to_be_bytes());     // nCodeSlots
    cd.extend_from_slice(&(code_limit as u32).to_be_bytes());  // codeLimit (low 32)
    cd.push(SHA256_HASH_SIZE as u8);                           // hashSize
    cd.push(CS_HASHTYPE_SHA256);                               // hashType
    cd.push(0);                                                // platform
    cd.push(CODESIGN_PAGE_SHIFT);                              // pageSize (log2)
    cd.extend_from_slice(&0u32.to_be_bytes());                 // spare2
    cd.extend_from_slice(&0u32.to_be_bytes());                 // scatterOffset
    cd.extend_from_slice(&0u32.to_be_bytes());                 // teamOffset
    cd.extend_from_slice(&0u32.to_be_bytes());                 // spare3
    cd.extend_from_slice(&0u64.to_be_bytes());                 // codeLimit64 (unused)
    cd.extend_from_slice(&exec_seg_base.to_be_bytes());        // execSegBase
    cd.extend_from_slice(&exec_seg_limit.to_be_bytes());       // execSegLimit
    cd.extend_from_slice(&CS_EXECSEG_MAIN_BINARY.to_be_bytes()); // execSegFlags

    debug_assert_eq!(cd.len(), CODE_DIRECTORY_HEADER_SIZE);

    // identifier (null-terminated)
    cd.extend_from_slice(ident_bytes);
    cd.push(0);

    // page hashes
    cd.extend_from_slice(&hashes);

    debug_assert_eq!(cd.len(), cd_total_len);

    // ----- 3. Wrap the CodeDirectory in a SuperBlob --------------------
    let total_len = SUPERBLOB_HEADER_SIZE + BLOB_INDEX_SIZE + cd.len();
    let cd_offset_in_super = (SUPERBLOB_HEADER_SIZE + BLOB_INDEX_SIZE) as u32;

    let mut sb: Vec<u8> = Vec::with_capacity(total_len);
    sb.extend_from_slice(&CSMAGIC_EMBEDDED_SIGNATURE.to_be_bytes());
    sb.extend_from_slice(&(total_len as u32).to_be_bytes());
    sb.extend_from_slice(&1u32.to_be_bytes());                  // count = 1
    sb.extend_from_slice(&CSSLOT_CODEDIRECTORY.to_be_bytes());  // slot type
    sb.extend_from_slice(&cd_offset_in_super.to_be_bytes());    // offset of CD
    sb.extend_from_slice(&cd);

    debug_assert_eq!(sb.len(), total_len);
    sb
}

/// Pack `artifact` into a Mach-O 64-bit executable binary.
///
/// Returns the raw bytes of the `.macho` file.  All multi-byte fields are
/// written in little-endian byte order.
///
/// # Errors
///
/// Returns `PackagerError::UnsupportedTarget` if the artifact's target is not
/// `macos_x64()` or `macos_arm64()`.
pub fn pack(artifact: &CodeArtifact) -> Result<Vec<u8>, PackagerError> {
    validate(artifact)?;
    let (cputype, cpusubtype) = cpu_type(&artifact.target)?;

    // Default load address for macOS 64-bit = 0x100000000 (above 4 GiB boundary).
    // Reject negative values: a negative load_address casts to an enormous u64,
    // producing a nonsensical vmaddr in the LC_SEGMENT_64 header.
    let load_addr_i = artifact.metadata_int("load_address", 0x100000000);
    if load_addr_i < 0 {
        return Err(PackagerError::UnsupportedTarget(format!(
            "macho64 packager: load_address must be non-negative, got {load_addr_i}"
        )));
    }
    let load_addr = load_addr_i as u64;
    let code_len = artifact.native_bytes.len() as u64;

    // Compute per-arch sizes.  Modern macOS requires six load commands
    // for an executable that the kernel will actually exec():
    //   1. LC_SEGMENT_64 __PAGEZERO        (traps null-pointer derefs)
    //   2. LC_SEGMENT_64 __TEXT            (one section: __text)
    //   3. LC_SEGMENT_64 __LINKEDIT        (holds the code-signature blob)
    //   4. LC_BUILD_VERSION                (min OS + SDK; required on arm64)
    //   5. LC_UNIXTHREAD                   (initial PC, no dyld)
    //   6. LC_CODE_SIGNATURE               (points into __LINKEDIT)
    //
    // LC_UNIXTHREAD's body length depends on the architecture
    // (272 bytes for arm64, 168 for x86-64).
    let ts = thread_state_info(&artifact.target.arch)?;
    let lc_unixthread_size: u64 = LC_UNIXTHREAD_HEADER_SIZE + ts.state_size;
    let header_total: u64 = MACH_HEADER_SIZE
        + LC_SEGMENT_SIZE                          // __PAGEZERO segment (no sections)
        + LC_SEGMENT_SIZE + SECTION_SIZE           // __TEXT segment + __text section
        + LC_SEGMENT_SIZE                          // __LINKEDIT segment (no sections)
        + LC_BUILD_VERSION_SIZE                    // min OS + SDK declaration
        + lc_unixthread_size                       // initial thread state
        + LC_CODE_SIGNATURE_SIZE;                  // code signature pointer
    let sizeofcmds: u32 = LC_SEGMENT_SIZE as u32   // __PAGEZERO
        + LC_SEGMENT_CMDSIZE                       // __TEXT
        + LC_SEGMENT_SIZE as u32                   // __LINKEDIT
        + LC_BUILD_VERSION_SIZE as u32             // LC_BUILD_VERSION
        + lc_unixthread_size as u32                // LC_UNIXTHREAD
        + LC_CODE_SIGNATURE_SIZE as u32;           // LC_CODE_SIGNATURE
    let text_segment_vmsize = header_total + code_len; // __TEXT covers headers + code

    // Code-signature placement: the signature lives in __LINKEDIT, which
    // starts at the next page boundary after the end of code.
    let code_end_offset: u64 = header_total + code_len;
    let sig_offset:      u64 = align_up(code_end_offset, CODESIGN_PAGE_SIZE);
    let sig_size:        u64 = signature_size(sig_offset) as u64;
    let linkedit_filesize: u64 = sig_size;
    let linkedit_vmsize:   u64 = align_up(sig_size, CODESIGN_PAGE_SIZE).max(CODESIGN_PAGE_SIZE);
    let total_file_size:   u64 = sig_offset + sig_size;

    let mut out: Vec<u8> = Vec::with_capacity(total_file_size as usize);

    // ── mach_header_64 ────────────────────────────────────────────────────────

    // magic = 0xFEEDFACF (MH_MAGIC_64).
    // In little-endian memory this is stored as [CF EA FE DE] = 0xCFFAEDFE.
    // We write the *value* 0xFEEDFACF; to_le_bytes() gives [CF FA ED FE].
    // Wait — let's be precise: MH_MAGIC_64 in the Mach-O spec is 0xFEEDFACF
    // (host-endian). On a little-endian host, the on-disk bytes are
    // [0xCF, 0xFA, 0xED, 0xFE], which is what a hex editor shows.
    out.extend_from_slice(&0xFEEDFACFu32.to_le_bytes());
    // cputype: CPU_TYPE_X86_64 or CPU_TYPE_ARM64.
    out.extend_from_slice(&cputype.to_le_bytes());
    // cpusubtype: CPU_SUBTYPE_X86_ALL or CPU_SUBTYPE_ARM_ALL.
    out.extend_from_slice(&cpusubtype.to_le_bytes());
    // filetype = 2 = MH_EXECUTE (executable binary).
    out.extend_from_slice(&2u32.to_le_bytes());
    // ncmds = 6: __PAGEZERO + __TEXT + __LINKEDIT + LC_BUILD_VERSION
    //           + LC_UNIXTHREAD + LC_CODE_SIGNATURE.
    out.extend_from_slice(&6u32.to_le_bytes());
    // sizeofcmds: total byte size of all load commands.
    out.extend_from_slice(&sizeofcmds.to_le_bytes());
    // flags: MH_NOUNDEFS (0x1) — no undefined references.  Tells the
    // kernel and dyld that the binary doesn't need dynamic linking.
    out.extend_from_slice(&0x00000001u32.to_le_bytes());
    // reserved (64-bit header only): must be 0.
    out.extend_from_slice(&0u32.to_le_bytes());

    debug_assert_eq!(out.len(), MACH_HEADER_SIZE as usize);

    // ── LC_SEGMENT_64 __PAGEZERO ──────────────────────────────────────────────
    //
    // A non-readable, non-writable, non-executable segment at virtual
    // address 0 that catches null-pointer dereferences.  Required by
    // modern macOS (≥ 10.x) — without it the kernel rejects the binary
    // at exec time with `ENOEXEC`.

    out.extend_from_slice(&LC_SEGMENT_64.to_le_bytes());
    out.extend_from_slice(&(LC_SEGMENT_SIZE as u32).to_le_bytes()); // cmdsize: header only
    write_name(&mut out, b"__PAGEZERO");
    out.extend_from_slice(&0u64.to_le_bytes());          // vmaddr   = 0
    out.extend_from_slice(&load_addr.to_le_bytes());     // vmsize   = entire user space below load_addr (4 GB by default)
    out.extend_from_slice(&0u64.to_le_bytes());          // fileoff  = 0
    out.extend_from_slice(&0u64.to_le_bytes());          // filesize = 0 (no backing)
    out.extend_from_slice(&0u32.to_le_bytes());          // maxprot  = 0
    out.extend_from_slice(&0u32.to_le_bytes());          // initprot = 0
    out.extend_from_slice(&0u32.to_le_bytes());          // nsects   = 0
    out.extend_from_slice(&0u32.to_le_bytes());          // flags    = 0

    debug_assert_eq!(out.len(), (MACH_HEADER_SIZE + LC_SEGMENT_SIZE) as usize);

    // ── LC_SEGMENT_64 __TEXT ──────────────────────────────────────────────────

    // cmd = LC_SEGMENT_64 = 0x19.
    out.extend_from_slice(&LC_SEGMENT_64.to_le_bytes());
    // cmdsize = 152: the size of this load command including all section_64 structs.
    out.extend_from_slice(&LC_SEGMENT_CMDSIZE.to_le_bytes());
    // segname: "__TEXT" padded to 16 bytes.
    write_name(&mut out, b"__TEXT");
    // vmaddr: virtual address of the segment's start.
    out.extend_from_slice(&load_addr.to_le_bytes());
    // vmsize: size of the segment in virtual memory.
    out.extend_from_slice(&text_segment_vmsize.to_le_bytes());
    // fileoff = 0: segment contents start at byte 0 of the file.
    out.extend_from_slice(&0u64.to_le_bytes());
    // filesize: number of bytes from the file that back this segment.
    out.extend_from_slice(&text_segment_vmsize.to_le_bytes());
    // maxprot = 7 = VM_PROT_READ | VM_PROT_WRITE | VM_PROT_EXECUTE (maximum allowed).
    out.extend_from_slice(&7u32.to_le_bytes());
    // initprot = 5 = VM_PROT_READ | VM_PROT_EXECUTE (initial protection at load).
    out.extend_from_slice(&5u32.to_le_bytes());
    // nsects = 1: this segment contains one section (__text).
    out.extend_from_slice(&1u32.to_le_bytes());
    // flags = 0: no special segment flags.
    out.extend_from_slice(&0u32.to_le_bytes());

    debug_assert_eq!(
        out.len(),
        (MACH_HEADER_SIZE + LC_SEGMENT_SIZE * 2) as usize,
        "header + __PAGEZERO segment + __TEXT segment header"
    );

    // ── section_64 for __text ─────────────────────────────────────────────────

    // sectname: "__text" padded to 16 bytes.
    write_name(&mut out, b"__text");
    // segname: "__TEXT" padded to 16 bytes.
    write_name(&mut out, b"__TEXT");
    // addr: virtual address of the section = segment start + header size.
    let text_addr = load_addr + header_total;
    out.extend_from_slice(&text_addr.to_le_bytes());
    // size: byte length of the section's content.
    out.extend_from_slice(&code_len.to_le_bytes());
    // offset: file offset of the section's content (right after all headers).
    out.extend_from_slice(&(header_total as u32).to_le_bytes());
    // align = 4: the section is aligned to 2^4 = 16 bytes (standard code alignment).
    out.extend_from_slice(&4u32.to_le_bytes());
    // reloff = 0: no relocations.
    out.extend_from_slice(&0u32.to_le_bytes());
    // nreloc = 0: no relocation entries.
    out.extend_from_slice(&0u32.to_le_bytes());
    // flags = S_ATTR_PURE_INSTRUCTIONS | S_ATTR_SOME_INSTRUCTIONS.
    //   S_ATTR_PURE_INSTRUCTIONS (0x80000000): all bytes are machine instructions.
    //   S_ATTR_SOME_INSTRUCTIONS (0x00000400): contains at least one instruction.
    out.extend_from_slice(&SECTION_FLAGS_CODE.to_le_bytes());
    // reserved1 = 0 (unused for code sections).
    out.extend_from_slice(&0u32.to_le_bytes());
    // reserved2 = 0 (unused for code sections).
    out.extend_from_slice(&0u32.to_le_bytes());
    // reserved3 = 0 (only in 64-bit section_64 struct).
    out.extend_from_slice(&0u32.to_le_bytes());

    debug_assert_eq!(
        out.len(),
        (MACH_HEADER_SIZE + LC_SEGMENT_SIZE * 2 + SECTION_SIZE) as usize
    );

    // ── LC_SEGMENT_64 __LINKEDIT ──────────────────────────────────────────────
    //
    // Holds the embedded code-signature blob.  Its file range starts at
    // the page-aligned end of __TEXT and extends through the signature.
    // Required by `codesign` and the kernel's code-signing path.

    out.extend_from_slice(&LC_SEGMENT_64.to_le_bytes());
    out.extend_from_slice(&(LC_SEGMENT_SIZE as u32).to_le_bytes());
    write_name(&mut out, b"__LINKEDIT");
    out.extend_from_slice(&(load_addr + sig_offset).to_le_bytes()); // vmaddr
    out.extend_from_slice(&linkedit_vmsize.to_le_bytes());          // vmsize
    out.extend_from_slice(&sig_offset.to_le_bytes());               // fileoff
    out.extend_from_slice(&linkedit_filesize.to_le_bytes());        // filesize
    out.extend_from_slice(&7u32.to_le_bytes());                     // maxprot
    out.extend_from_slice(&1u32.to_le_bytes());                     // initprot = read-only
    out.extend_from_slice(&0u32.to_le_bytes());                     // nsects = 0
    out.extend_from_slice(&0u32.to_le_bytes());                     // flags

    debug_assert_eq!(
        out.len(),
        (MACH_HEADER_SIZE + LC_SEGMENT_SIZE * 3 + SECTION_SIZE) as usize
    );

    // ── LC_BUILD_VERSION ──────────────────────────────────────────────────────
    //
    // Modern macOS (especially Apple Silicon) `SIGKILL`s binaries that
    // omit a build-version load command.  Declares the minimum OS that
    // can run this binary plus the SDK it was built against.

    out.extend_from_slice(&LC_BUILD_VERSION.to_le_bytes());
    out.extend_from_slice(&(LC_BUILD_VERSION_SIZE as u32).to_le_bytes());
    out.extend_from_slice(&PLATFORM_MACOS.to_le_bytes());
    out.extend_from_slice(&MIN_OS_VERSION.to_le_bytes());
    out.extend_from_slice(&SDK_VERSION.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());          // ntools = 0

    // ── LC_UNIXTHREAD ─────────────────────────────────────────────────────────
    //
    // Specifies an initial thread state — entire register file zeroed
    // except for the program counter, which is set to the absolute load
    // address of the entry point.  No dyld involvement; the kernel jumps
    // directly to PC after setting up the thread.

    out.extend_from_slice(&LC_UNIXTHREAD.to_le_bytes());
    out.extend_from_slice(&(lc_unixthread_size as u32).to_le_bytes());
    out.extend_from_slice(&ts.flavor.to_le_bytes());
    out.extend_from_slice(&ts.count.to_le_bytes());

    // Build the thread state — all zero, then patch in the PC value.
    let entry_pc = load_addr + header_total + artifact.entry_point as u64;
    let mut state = vec![0u8; ts.state_size as usize];
    state[ts.pc_offset .. ts.pc_offset + 8].copy_from_slice(&entry_pc.to_le_bytes());
    out.extend_from_slice(&state);

    // ── LC_CODE_SIGNATURE ─────────────────────────────────────────────────────
    //
    // Points the kernel's code-signing path at our embedded SuperBlob.

    out.extend_from_slice(&LC_CODE_SIGNATURE.to_le_bytes());
    out.extend_from_slice(&(LC_CODE_SIGNATURE_SIZE as u32).to_le_bytes());
    out.extend_from_slice(&(sig_offset as u32).to_le_bytes());      // dataoff
    out.extend_from_slice(&(sig_size  as u32).to_le_bytes());       // datasize

    debug_assert_eq!(out.len(), header_total as usize);

    // ── Machine code ──────────────────────────────────────────────────────────

    out.extend_from_slice(&artifact.native_bytes);

    // ── Page-pad up to sig_offset ─────────────────────────────────────────────
    //
    // The hashed prefix must end on a page boundary.  Real disk content
    // is zero-padded; the same zeros also appear in the hash of the
    // last partial page.

    while (out.len() as u64) < sig_offset {
        out.push(0);
    }

    debug_assert_eq!(out.len() as u64, sig_offset);

    // ── Embed the ad-hoc code signature ───────────────────────────────────────
    //
    // The signature SuperBlob contains a CodeDirectory whose page hashes
    // cover everything in the file from offset 0 to `sig_offset` — i.e.
    // the bytes we just emitted.  After this append the file is
    // complete.

    let signature = build_adhoc_signature(
        &out,                     // hash everything written so far
        0,                        // execSegBase: __TEXT starts at file offset 0
        text_segment_vmsize,      // execSegLimit: __TEXT covers headers + code
    );
    debug_assert_eq!(signature.len() as u64, sig_size);
    out.extend_from_slice(&signature);

    Ok(out)
}

/// The conventional file extension for Mach-O files.
pub fn file_extension() -> &'static str {
    ".macho"
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn macos_arm64_art() -> CodeArtifact {
        CodeArtifact::new(vec![0x1F, 0x20, 0x03, 0xD5], 0, Target::macos_arm64())
    }

    // Test 1: Mach-O magic bytes (little-endian stored as [CF FA ED FE])
    #[test]
    fn produces_macho_magic() {
        let bytes = pack(&macos_arm64_art()).unwrap();
        // 0xFEEDFACF.to_le_bytes() = [0xCF, 0xFA, 0xED, 0xFE]
        assert_eq!(&bytes[0..4], &[0xCF, 0xFA, 0xED, 0xFE]);
    }

    // Test 2: rejects linux_x64
    #[test]
    fn rejects_linux_target() {
        let art = CodeArtifact::new(vec![0x90], 0, Target::linux_x64());
        assert!(matches!(pack(&art), Err(PackagerError::UnsupportedTarget(_))));
    }

    // ncmds: 6 = __PAGEZERO + __TEXT + __LINKEDIT + LC_BUILD_VERSION
    //              + LC_UNIXTHREAD + LC_CODE_SIGNATURE.
    #[test]
    fn ncmds_is_six() {
        let bytes = pack(&macos_arm64_art()).unwrap();
        let ncmds = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        assert_eq!(ncmds, 6);
    }

    /// First load command is __PAGEZERO.
    #[test]
    fn first_load_command_is_pagezero() {
        let bytes = pack(&macos_arm64_art()).unwrap();
        // segname starts at offset 32 (after mach_header) + 8 (after cmd+cmdsize).
        let segname_off = 32 + 8;
        assert_eq!(&bytes[segname_off..segname_off + 10], b"__PAGEZERO");
    }

    /// Output is page-aligned at the signature offset (≥ 4096) and ends
    /// with a SuperBlob whose magic is `0xfade0cc0` (big-endian).
    #[test]
    fn produces_codesign_superblob() {
        let bytes = pack(&macos_arm64_art()).unwrap();
        // The signature blob magic is at sig_offset.  We don't know
        // sig_offset exactly without re-implementing the layout maths,
        // but it must be at *some* page boundary ≥ 4096 and the magic
        // is unique enough to find by scanning.
        let needle = [0xfa, 0xde, 0x0c, 0xc0];
        assert!(
            bytes.windows(4).any(|w| w == needle),
            "expected SuperBlob magic in output"
        );
    }

    // Test 5: ARM64 CPU type at offset 4
    #[test]
    fn arm64_cpu_type() {
        let bytes = pack(&macos_arm64_art()).unwrap();
        let cputype = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(cputype, CPU_TYPE_ARM64);
    }

    // Test 6: x86_64 CPU type
    #[test]
    fn x86_64_cpu_type() {
        let art = CodeArtifact::new(vec![0x90], 0, Target::macos_x64());
        let bytes = pack(&art).unwrap();
        let cputype = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(cputype, CPU_TYPE_X86_64);
    }

    /// __LINKEDIT segment name appears somewhere in the load commands.
    #[test]
    fn linkedit_segment_present() {
        let bytes = pack(&macos_arm64_art()).unwrap();
        let needle = b"__LINKEDIT\0\0\0\0\0\0";
        assert!(
            bytes.windows(needle.len()).any(|w| w == needle),
            "expected __LINKEDIT segment name in output"
        );
    }

    /// LC_UNIXTHREAD command id (0x05) appears in the load commands.
    #[test]
    fn lc_unixthread_present() {
        let bytes = pack(&macos_arm64_art()).unwrap();
        let mut cmd_offset = 32; // skip mach_header
        let mut found = false;
        // ncmds = 6, walk them to find LC_UNIXTHREAD.
        for _ in 0..6 {
            let cmd = u32::from_le_bytes(bytes[cmd_offset..cmd_offset + 4].try_into().unwrap());
            let cmdsize = u32::from_le_bytes(bytes[cmd_offset + 4..cmd_offset + 8].try_into().unwrap()) as usize;
            if cmd == LC_UNIXTHREAD { found = true; break; }
            cmd_offset += cmdsize;
        }
        assert!(found, "LC_UNIXTHREAD not found in load commands");
    }

    // Test 8: file_extension
    #[test]
    fn file_extension_is_macho() {
        assert_eq!(file_extension(), ".macho");
    }
}
