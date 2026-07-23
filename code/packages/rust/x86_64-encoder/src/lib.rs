//! # `x86_64-encoder` — x86-64 (AMD64 / Intel 64) instruction encoder.
//!
//! Pure-Rust encoder that produces little-endian byte streams for the
//! x86-64 instruction set in 64-bit (long) mode.  Designed to be the
//! bottom-of-stack for any CIR → native-code lowering in this repo
//! (`jit-core` / `aot-core`) on Linux x86-64 and Windows x86-64.
//! Implements [LANG44](../../../../specs/LANG44-x86_64-encoder.md).
//!
//! ## Contract
//!
//! - Each high-level method on [`Assembler`] emits **one** logical
//!   instruction, of variable byte length (1–15 bytes).
//! - Branches reference [`LabelId`]s that are resolved at
//!   [`Assembler::finish`] time — no two-pass bookkeeping is exposed
//!   to callers.
//! - Output is the raw `.text` byte stream — no headers, no relocations
//!   beyond branch fix-ups.  Wrapping into ELF / PE / Mach-O is the
//!   job of `code-packager` (LANG45).
//! - **OS- and ABI-agnostic.**  The encoder produces the same bytes
//!   for the same instruction regardless of host OS or ABI.  Argument-
//!   register choice (System V's RDI/RSI/RDX/RCX/R8/R9 vs Microsoft
//!   x64's RCX/RDX/R8/R9) is a backend concern (LANG43); object-format
//!   reloc IDs are a packager concern (LANG45).
//!
//! ## Always-long-form policy
//!
//! V1 always emits `Rel32` displacements for branches and `disp32`
//! displacements for memory operands, even when shorter forms exist.
//! This wastes a few bytes per branch/memory access but makes label
//! resolution trivial: the byte length of every instruction is known
//! at the moment its first byte is written, so fix-ups patch a fixed-
//! offset rel32 slot.
//!
//! ## Reference
//!
//! - Intel SDM Vol. 2 (instruction set reference)
//! - AMD64 APM Vol. 3 (instruction encodings)

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

// ===========================================================================
// Registers
// ===========================================================================

/// 64-bit general-purpose register.
///
/// All 16 GPRs are exposed.  The enum discriminant happens to equal the
/// 4-bit register code used in Intel ModR/M / SIB encoding (with the high
/// bit routed into the REX prefix's `R` / `X` / `B` field, depending on
/// position).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
#[repr(u8)]
pub enum Reg {
    Rax = 0, Rcx = 1, Rdx = 2, Rbx = 3,
    Rsp = 4, Rbp = 5, Rsi = 6, Rdi = 7,
    R8  = 8, R9  = 9, R10 = 10, R11 = 11,
    R12 = 12, R13 = 13, R14 = 14, R15 = 15,
}

impl Reg {
    /// Full 4-bit register code (0..=15).
    #[inline]
    pub fn code(self) -> u8 { self as u8 }

    /// Low 3 bits — the field that lands in ModR/M `reg` / `rm` or SIB
    /// `index` / `base`.
    #[inline]
    fn low3(self) -> u8 { (self as u8) & 0x7 }

    /// High bit — feeds REX.R / REX.X / REX.B depending on slot.
    #[inline]
    fn high1(self) -> bool { (self as u8) >= 8 }
}

// ===========================================================================
// Condition codes
// ===========================================================================

/// x86-64 branch / `SETcc` / `CMOVcc` condition codes.
///
/// Discriminant equals the 4-bit `tttn` field in `Jcc` / `SETcc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
#[repr(u8)]
pub enum Cond {
    /// Overflow.
    O  = 0x0,
    /// No overflow.
    No = 0x1,
    /// Below / carry / unsigned `<`.
    B  = 0x2,
    /// Above or equal / not carry / unsigned `>=`.
    Ae = 0x3,
    /// Equal / zero.
    E  = 0x4,
    /// Not equal / not zero.
    Ne = 0x5,
    /// Below or equal / unsigned `<=`.
    Be = 0x6,
    /// Above / unsigned `>`.
    A  = 0x7,
    /// Sign (negative).
    S  = 0x8,
    /// Not sign (non-negative).
    Ns = 0x9,
    /// Parity even.
    P  = 0xA,
    /// Parity odd.
    Np = 0xB,
    /// Less / signed `<`.
    L  = 0xC,
    /// Greater or equal / signed `>=`.
    Ge = 0xD,
    /// Less or equal / signed `<=`.
    Le = 0xE,
    /// Greater / signed `>`.
    G  = 0xF,
}

impl Cond {
    #[inline]
    fn tttn(self) -> u8 { self as u8 }
}

// ===========================================================================
// Labels & fix-ups
// ===========================================================================

/// Opaque label handle returned by [`Assembler::create_label`].
///
/// Has no fixed byte address until [`Assembler::bind`] is called.  Any
/// branch encoded against an unbound label is patched at
/// [`Assembler::finish`] time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LabelId(u32);

/// A branch fix-up recorded at instruction emission time.
///
/// `slot_offset` is the byte offset in `code` of the *first byte* of
/// the 4-byte `rel32` field within the instruction.  `instr_end_offset`
/// is the byte offset of the instruction immediately following — used
/// as the PC reference point for x86-64 PC-relative displacement.
#[derive(Debug, Clone, Copy)]
struct Fixup {
    slot_offset: usize,
    instr_end_offset: usize,
    target: LabelId,
}

// ===========================================================================
// Errors
// ===========================================================================

/// Errors detected when finalising an instruction stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// A label was referenced by a branch but never bound to an
    /// instruction.
    UnboundLabel(LabelId),
    /// A label was bound twice — labels must designate exactly one
    /// location.
    LabelAlreadyBound(LabelId),
    /// A PC-relative branch displacement exceeded the signed 32-bit
    /// range (`±2 GiB`).
    BranchOutOfRange {
        /// Byte delta from the instruction end to the target.
        delta_bytes: i64,
    },
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::UnboundLabel(l) =>
                write!(f, "label {:?} referenced but never bound", l),
            EncodeError::LabelAlreadyBound(l) =>
                write!(f, "label {:?} bound twice", l),
            EncodeError::BranchOutOfRange { delta_bytes } =>
                write!(f, "branch displacement {delta_bytes} bytes \
                          doesn't fit in 32 bits"),
        }
    }
}

impl std::error::Error for EncodeError {}

// ===========================================================================
// External relocations
// ===========================================================================

/// Abstract relocation kind.
///
/// The encoder stays OS-agnostic by surfacing relocations using these
/// platform-neutral labels.  LANG45's `code-packager` translates each
/// kind to the right OS-specific relocation type ID at emit time:
///
/// | Kind | ELF | PE/COFF | Mach-O |
/// |---|---|---|---|
/// | `PltRel32` | `R_X86_64_PLT32` (4) | `IMAGE_REL_AMD64_REL32` (0x04) | `X86_64_RELOC_BRANCH` |
/// | `PcRel32` | `R_X86_64_PC32` (2) | `IMAGE_REL_AMD64_REL32` (0x04) | `X86_64_RELOC_SIGNED` |
/// | `GotPcRel32` | `R_X86_64_REX_GOTPCRELX` (42) | (collapses to `REL32`) | (collapses to `SIGNED`) |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalRelocKind {
    /// PC-relative branch target — for `CALL rel32` to external
    /// functions.  Goes through the PLT on dynamically-linked ELF.
    PltRel32,
    /// PC-relative 32-bit displacement — for RIP-relative loads/stores
    /// of statically-resolved symbols.
    PcRel32,
    /// RIP-relative GOT load — for possibly-external globals on ELF.
    /// On PE/COFF and Mach-O this collapses to a plain PC-relative
    /// reloc (no separate GOT in V1).
    GotPcRel32,
}

/// A pending external relocation recorded by the encoder.
///
/// The packager (LANG45) consumes these to emit OS-specific reloc
/// records into the `.rela.text` / relocation tables of the produced
/// object file.
#[derive(Debug, Clone)]
pub struct ExternalReloc {
    /// Byte offset in `code` of the 4-byte slot to patch.
    pub patch_offset: usize,
    /// Symbol name (e.g., `"__twig_print_i64"`).
    pub symbol: String,
    /// Relocation kind.
    pub kind: ExternalRelocKind,
    /// Addend stored in the reloc record (ELF) or baked into the
    /// instruction's displacement (PE/COFF).  Always `-4` for the
    /// V1 instruction forms because x86-64 PC-relative is measured
    /// from the *end* of the instruction.
    pub addend: i32,
}

// ===========================================================================
// Bit-packing helpers
// ===========================================================================

/// Compose the REX prefix byte.
///
/// `w=1` selects 64-bit operand size; `r` / `x` / `b` extend
/// ModR/M.reg, SIB.index, and ModR/M.rm (or SIB.base, or opcode
/// embedded register) respectively to 4-bit registers.
#[inline]
fn rex(w: bool, r: bool, x: bool, b: bool) -> u8 {
    0x40
        | ((w as u8) << 3)
        | ((r as u8) << 2)
        | ((x as u8) << 1)
        | (b as u8)
}

/// Compose a ModR/M byte.
///
/// `mode ∈ {00, 01, 10, 11}`: addressing mode (memory with optional
/// disp8/disp32, or pure register).  `reg` and `rm` are 3-bit fields.
#[inline]
fn modrm(mode: u8, reg: u8, rm: u8) -> u8 {
    debug_assert!(mode < 4);
    debug_assert!(reg < 8);
    debug_assert!(rm < 8);
    (mode << 6) | (reg << 3) | rm
}

// ===========================================================================
// Assembler
// ===========================================================================

/// Stream-style x86-64 assembler.
///
/// Emits instructions as bytes into an internal buffer; resolves
/// label-relative branches at [`Assembler::finish`] time.  External
/// (cross-function or runtime) relocations are recorded for the
/// packager to resolve later.
#[derive(Debug)]
pub struct Assembler {
    /// Emitted bytes in order.
    code: Vec<u8>,
    /// `labels[i]` is `Some(byte_offset)` once the i-th label is bound.
    labels: Vec<Option<usize>>,
    /// Pending branch fix-ups.
    fixups: Vec<Fixup>,
    /// External relocations (calls into the runtime, RIP-rel to
    /// globals, etc.) — exposed publicly so the backend can route
    /// them to the packager.
    pub external_relocs: Vec<ExternalReloc>,
}

impl Default for Assembler {
    fn default() -> Self { Self::new() }
}

impl Assembler {
    /// Create an empty assembler.
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            labels: Vec::new(),
            fixups: Vec::new(),
            external_relocs: Vec::new(),
        }
    }

    /// Current byte offset (number of bytes emitted so far).
    #[inline]
    pub fn len(&self) -> usize { self.code.len() }

    /// True if no bytes have been emitted.
    #[inline]
    pub fn is_empty(&self) -> bool { self.code.is_empty() }

    // -----------------------------------------------------------------------
    // Labels
    // -----------------------------------------------------------------------

    /// Allocate a fresh, unbound label.
    pub fn create_label(&mut self) -> LabelId {
        let id = LabelId(self.labels.len() as u32);
        self.labels.push(None);
        id
    }

    /// Bind `label` to the *next* byte that will be emitted.
    ///
    /// Returns [`EncodeError::LabelAlreadyBound`] if the label was
    /// already bound.
    pub fn bind(&mut self, label: LabelId) -> Result<(), EncodeError> {
        let slot = &mut self.labels[label.0 as usize];
        if slot.is_some() {
            return Err(EncodeError::LabelAlreadyBound(label));
        }
        *slot = Some(self.code.len());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Low-level emission
    // -----------------------------------------------------------------------

    #[inline]
    fn emit_u8(&mut self, b: u8) { self.code.push(b); }

    #[inline]
    fn emit_u32_le(&mut self, w: u32) {
        self.code.extend_from_slice(&w.to_le_bytes());
    }

    #[inline]
    fn emit_u64_le(&mut self, w: u64) {
        self.code.extend_from_slice(&w.to_le_bytes());
    }

    /// Emit a REX prefix + opcode + ModR/M `mod=11` (register-to-register)
    /// instruction.
    ///
    /// This is the common shape for `ADD`/`SUB`/`MOV`/`CMP`/`AND`/`OR`/
    /// `XOR`/`TEST` register-form opcodes.  Returns the instruction's
    /// total byte length (3 for the always-REX-W form).
    #[inline]
    fn emit_rr(&mut self, opcode: u8, reg_src: Reg, rm_dst: Reg) -> usize {
        // `reg_src` lands in ModR/M.reg; `rm_dst` in ModR/M.rm.
        // For Intel `r/m64, r64` opcodes (e.g. ADD `0x01`), the
        // destination is the `r/m` operand and the source is `r`.
        self.emit_u8(rex(true, reg_src.high1(), false, rm_dst.high1()));
        self.emit_u8(opcode);
        self.emit_u8(modrm(0b11, reg_src.low3(), rm_dst.low3()));
        3
    }

    /// Emit `[REX.W + B=dst.high] [opcode] [ModR/M mod=11, reg=ext, rm=dst.low3] [imm32]`
    /// — the canonical "immediate to register, opcode-extension in reg field"
    /// shape used by `ADD r/m64, imm32` / `SUB r/m64, imm32` / `CMP r/m64,
    /// imm32` etc.
    #[inline]
    fn emit_ri32(&mut self, opcode: u8, ext: u8, dst: Reg, imm: i32) -> usize {
        self.emit_u8(rex(true, false, false, dst.high1()));
        self.emit_u8(opcode);
        self.emit_u8(modrm(0b11, ext, dst.low3()));
        self.emit_u32_le(imm as u32);
        7
    }

    // -----------------------------------------------------------------------
    // MOV family
    // -----------------------------------------------------------------------

    /// `MOV r/m64, r64` — register-to-register move.
    ///
    /// Opcode `0x89 /r`.  Encoding `[REX.W B=dst.high R=src.high] 89
    /// [mod=11 reg=src.low3 rm=dst.low3]` — note the source is the
    /// ModR/M.reg operand, destination is ModR/M.rm (Intel `r/m64, r64`).
    pub fn mov_r64_r64(&mut self, dst: Reg, src: Reg) {
        self.emit_rr(0x89, src, dst);
    }

    /// `MOV r/m64, imm32` (sign-extended).
    ///
    /// Opcode `0xC7 /0`.  Use this when the constant fits in `i32`;
    /// for full 64-bit constants use [`Assembler::mov_r64_imm64`].
    pub fn mov_r64_imm32(&mut self, dst: Reg, imm: i32) {
        self.emit_u8(rex(true, false, false, dst.high1()));
        self.emit_u8(0xC7);
        self.emit_u8(modrm(0b11, 0, dst.low3()));
        self.emit_u32_le(imm as u32);
    }

    /// `MOVABS r64, imm64` — load 64-bit immediate.
    ///
    /// Opcode `B8+rd`, with the destination register encoded in the
    /// opcode byte's low 3 bits and its high bit going into `REX.B`.
    /// 10 bytes total.
    pub fn mov_r64_imm64(&mut self, dst: Reg, imm: u64) {
        self.emit_u8(rex(true, false, false, dst.high1()));
        self.emit_u8(0xB8 + dst.low3());
        self.emit_u64_le(imm);
    }

    /// `MOV r64, [base + disp32]` — load 64-bit value from memory.
    ///
    /// Opcode `0x8B /r`.  Always uses `disp32` form (V1 policy).
    /// When `base == RSP` or `R12`, an SIB byte is required by the
    /// ISA — we emit it automatically.
    pub fn mov_r64_mem(&mut self, dst: Reg, base: Reg, disp: i32) {
        self.emit_load_store(0x8B, dst, base, disp);
    }

    /// `MOV [base + disp32], r64` — store 64-bit register to memory.
    ///
    /// Opcode `0x89 /r`.
    pub fn mov_mem_r64(&mut self, base: Reg, disp: i32, src: Reg) {
        self.emit_load_store(0x89, src, base, disp);
    }

    /// Shared encoding for `[base + disp32]` `r/m64, r64` loads and stores.
    ///
    /// The `reg` operand lives in ModR/M.reg; the memory operand is
    /// `[base + disp32]` encoded in ModR/M.rm + (SIB when base is
    /// RSP / R12) + disp32.
    fn emit_load_store(&mut self, opcode: u8, reg: Reg, base: Reg, disp: i32) {
        // RBP and R13 also have a special case when disp == 0 (it would
        // otherwise mean RIP-relative addressing), but disp32 form
        // (mod=10) avoids that entirely.
        let needs_sib = base.low3() == 4; // RSP or R12
        self.emit_u8(rex(true, reg.high1(), false, base.high1()));
        self.emit_u8(opcode);
        if needs_sib {
            // mod=10, reg, rm=100 (SIB indicator)
            self.emit_u8(modrm(0b10, reg.low3(), 0b100));
            // SIB: scale=00, index=100 (none), base=base.low3
            self.emit_u8((0b100 << 3) | base.low3());
        } else {
            self.emit_u8(modrm(0b10, reg.low3(), base.low3()));
        }
        self.emit_u32_le(disp as u32);
    }

    /// `LEA r64, [RIP + disp32]` — load RIP-relative address.
    ///
    /// Records an external relocation with the supplied symbol name,
    /// so the linker can patch in the final offset.  Opcode `0x8D /r`
    /// with ModR/M `mod=00, rm=101` (RIP-relative form).  7 bytes.
    pub fn lea_rip_rel(&mut self, dst: Reg, symbol: &str, kind: ExternalRelocKind) {
        self.emit_u8(rex(true, dst.high1(), false, false));
        self.emit_u8(0x8D);
        self.emit_u8(modrm(0b00, dst.low3(), 0b101));
        let patch_offset = self.code.len();
        self.emit_u32_le(0); // placeholder disp32
        self.external_relocs.push(ExternalReloc {
            patch_offset,
            symbol: symbol.to_owned(),
            kind,
            addend: -4,
        });
    }

    /// `LEA r64, [RIP + <label>]` — load the RIP-relative address of a **label** bound
    /// later in this same function; resolved at [`finish`](Self::finish) like a `jmp`.
    /// Unlike [`lea_rip_rel`](Self::lea_rip_rel) this needs no relocation — the target is
    /// intra-function, so the displacement is known once the label is bound. Used to
    /// point a register at a constant table embedded in the instruction stream (see
    /// [`emit_data_u32`](Self::emit_data_u32)). `0x8D /r`, RIP-relative form (7 bytes).
    pub fn lea_rip_label(&mut self, dst: Reg, target: LabelId) {
        self.emit_u8(rex(true, dst.high1(), false, false));
        self.emit_u8(0x8D);
        self.emit_u8(modrm(0b00, dst.low3(), 0b101));
        let slot_offset = self.code.len();
        self.emit_u32_le(0); // placeholder disp32
        let instr_end_offset = self.code.len();
        self.fixups.push(Fixup { slot_offset, instr_end_offset, target });
    }

    /// `LEA r64, [RIP + #0]` as a **placeholder**, returning the byte offset of its
    /// `disp32` slot for a caller that patches it once a target offset is known — the
    /// RIP-relative analogue of a cross-function reference. The caller writes
    /// `disp32 = target_off − (slot + 4)` (the `+4` is the distance from the slot to the
    /// end of the instruction = the RIP value). Base-independent: `RIP` and the target
    /// both carry the load base, so it cancels — no relocation needed. `0x8D /r` (7
    /// bytes). No fix-up is queued; `finish()` leaves the placeholder for the caller.
    pub fn lea_rip_placeholder(&mut self, dst: Reg) -> usize {
        self.emit_u8(rex(true, dst.high1(), false, false));
        self.emit_u8(0x8D);
        self.emit_u8(modrm(0b00, dst.low3(), 0b101));
        let slot = self.code.len();
        self.emit_u32_le(0); // placeholder disp32 — patched by the caller
        slot
    }

    /// Append a raw little-endian `u32` **data word** to the stream and return its byte
    /// offset. This is constant data (a `u32`/`i32` table element), not an instruction;
    /// it is only safe where control flow cannot reach it (after a `ret`, or a region
    /// referenced solely by [`lea_rip_label`](Self::lea_rip_label)).
    pub fn emit_data_u32(&mut self, w: u32) -> usize {
        let off = self.code.len();
        self.emit_u32_le(w);
        off
    }

    // -----------------------------------------------------------------------
    // Arithmetic
    // -----------------------------------------------------------------------

    /// `ADD r/m64, r64` — add `src` into `dst`.
    ///
    /// Opcode `0x01 /r`.
    pub fn add(&mut self, dst: Reg, src: Reg) {
        self.emit_rr(0x01, src, dst);
    }

    /// `SUB r/m64, r64` — subtract `src` from `dst`.
    ///
    /// Opcode `0x29 /r`.
    pub fn sub(&mut self, dst: Reg, src: Reg) {
        self.emit_rr(0x29, src, dst);
    }

    /// `IMUL r64, r/m64` — signed multiply `dst *= src`.
    ///
    /// Two-byte opcode `0x0F 0xAF`.  Note: the operand order is the
    /// *opposite* of `ADD`/`SUB` because `IMUL r64, r/m64` puts the
    /// destination in ModR/M.reg.
    pub fn imul(&mut self, dst: Reg, src: Reg) {
        self.emit_u8(rex(true, dst.high1(), false, src.high1()));
        self.emit_u8(0x0F);
        self.emit_u8(0xAF);
        self.emit_u8(modrm(0b11, dst.low3(), src.low3()));
    }

    /// `IDIV r/m64` — signed divide `RDX:RAX` by `divisor`.
    ///
    /// Yields quotient in `RAX`, remainder in `RDX`.  Caller must
    /// sign-extend the dividend into `RDX:RAX` first (see
    /// [`Assembler::cqo`]).  Opcode `0xF7 /7`.
    pub fn idiv(&mut self, divisor: Reg) {
        self.emit_u8(rex(true, false, false, divisor.high1()));
        self.emit_u8(0xF7);
        self.emit_u8(modrm(0b11, 7, divisor.low3()));
    }

    /// `DIV r/m64` — unsigned divide `RDX:RAX` by `divisor`.
    ///
    /// Caller must zero `RDX` first (e.g., `XOR RDX, RDX`).
    /// Opcode `0xF7 /6`.
    pub fn div(&mut self, divisor: Reg) {
        self.emit_u8(rex(true, false, false, divisor.high1()));
        self.emit_u8(0xF7);
        self.emit_u8(modrm(0b11, 6, divisor.low3()));
    }

    /// `CQO` — sign-extend `RAX` into `RDX:RAX`.  Required before
    /// `IDIV`.  Opcode `0x48 0x99`.
    pub fn cqo(&mut self) {
        self.emit_u8(0x48);
        self.emit_u8(0x99);
    }

    /// `ADD r/m64, imm32` (sign-extended).  Opcode `0x81 /0`.
    pub fn add_imm32(&mut self, dst: Reg, imm: i32) {
        self.emit_ri32(0x81, 0, dst, imm);
    }

    /// `SUB r/m64, imm32` (sign-extended).  Opcode `0x81 /5`.
    pub fn sub_imm32(&mut self, dst: Reg, imm: i32) {
        self.emit_ri32(0x81, 5, dst, imm);
    }

    /// `NEG r/m64` — two's-complement negate.  Opcode `0xF7 /3`.
    pub fn neg_(&mut self, dst: Reg) {
        self.emit_u8(rex(true, false, false, dst.high1()));
        self.emit_u8(0xF7);
        self.emit_u8(modrm(0b11, 3, dst.low3()));
    }

    // -----------------------------------------------------------------------
    // Logical
    // -----------------------------------------------------------------------

    /// `AND r/m64, r64`.  Opcode `0x21 /r`.
    pub fn and_(&mut self, dst: Reg, src: Reg) { self.emit_rr(0x21, src, dst); }

    /// `OR r/m64, r64`.  Opcode `0x09 /r`.
    pub fn or_(&mut self, dst: Reg, src: Reg) { self.emit_rr(0x09, src, dst); }

    /// `XOR r/m64, r64`.  Opcode `0x31 /r`.
    pub fn xor_(&mut self, dst: Reg, src: Reg) { self.emit_rr(0x31, src, dst); }

    /// `TEST r/m64, r64` — bitwise AND, discard result, set flags.
    ///
    /// Used to implement `jmp_if_true/false v` after loading `v` into
    /// a register: `TEST rax, rax; JNE label`.  Opcode `0x85 /r`.
    pub fn test_(&mut self, lhs: Reg, rhs: Reg) {
        self.emit_rr(0x85, rhs, lhs);
    }

    /// `NOT r/m64` — bitwise complement.  Opcode `0xF7 /2`.
    pub fn not_(&mut self, dst: Reg) {
        self.emit_u8(rex(true, false, false, dst.high1()));
        self.emit_u8(0xF7);
        self.emit_u8(modrm(0b11, 2, dst.low3()));
    }

    // -----------------------------------------------------------------------
    // Shifts
    // -----------------------------------------------------------------------

    /// `SHL r/m64, CL` — logical shift left by CL.  Opcode `0xD3 /4`.
    pub fn shl_cl(&mut self, dst: Reg) { self.emit_shift_cl(dst, 4); }

    /// `SHR r/m64, CL` — logical shift right by CL.  Opcode `0xD3 /5`.
    pub fn shr_cl(&mut self, dst: Reg) { self.emit_shift_cl(dst, 5); }

    /// `SAR r/m64, CL` — arithmetic shift right by CL.  Opcode `0xD3 /7`.
    pub fn sar_cl(&mut self, dst: Reg) { self.emit_shift_cl(dst, 7); }

    fn emit_shift_cl(&mut self, dst: Reg, ext: u8) {
        self.emit_u8(rex(true, false, false, dst.high1()));
        self.emit_u8(0xD3);
        self.emit_u8(modrm(0b11, ext, dst.low3()));
    }

    /// `SHL r/m64, imm8`.  Opcode `0xC1 /4 ib`.
    pub fn shl_imm8(&mut self, dst: Reg, imm: u8) { self.emit_shift_imm(dst, 4, imm); }

    /// `SHR r/m64, imm8`.  Opcode `0xC1 /5 ib`.
    pub fn shr_imm8(&mut self, dst: Reg, imm: u8) { self.emit_shift_imm(dst, 5, imm); }

    /// `SAR r/m64, imm8`.  Opcode `0xC1 /7 ib`.
    pub fn sar_imm8(&mut self, dst: Reg, imm: u8) { self.emit_shift_imm(dst, 7, imm); }

    fn emit_shift_imm(&mut self, dst: Reg, ext: u8, imm: u8) {
        self.emit_u8(rex(true, false, false, dst.high1()));
        self.emit_u8(0xC1);
        self.emit_u8(modrm(0b11, ext, dst.low3()));
        self.emit_u8(imm);
    }

    // -----------------------------------------------------------------------
    // Compare + set
    // -----------------------------------------------------------------------

    /// `CMP r/m64, r64` — compute `lhs - rhs`, discard, set flags.
    ///
    /// Opcode `0x39 /r`.
    pub fn cmp(&mut self, lhs: Reg, rhs: Reg) { self.emit_rr(0x39, rhs, lhs); }

    /// `CMP r/m64, imm32` (sign-extended).  Opcode `0x81 /7`.
    pub fn cmp_imm32(&mut self, lhs: Reg, imm: i32) {
        self.emit_ri32(0x81, 7, lhs, imm);
    }

    /// `SETcc r/m8` — store 0/1 into the low byte of `dst` based on
    /// the condition.
    ///
    /// Two-byte opcode `0x0F 0x90+cc`, ModR/M `mod=11, reg=0`.  The
    /// destination is treated as 8-bit; callers wanting full-width
    /// results should follow with [`Assembler::movzx_r64_r8`].
    pub fn setcc(&mut self, cond: Cond, dst: Reg) {
        // For 8-bit destinations RAX..RBX we don't strictly need REX,
        // but accessing the byte forms of RSP..RDI (SPL/BPL/SIL/DIL)
        // *does* require REX.  Emit REX unconditionally for uniformity.
        self.emit_u8(rex(false, false, false, dst.high1()));
        self.emit_u8(0x0F);
        self.emit_u8(0x90 | cond.tttn());
        self.emit_u8(modrm(0b11, 0, dst.low3()));
    }

    /// `MOVZX r64, r8` — zero-extend low byte of `src` into `dst`.
    ///
    /// Opcode `0x0F 0xB6 /r`.  Useful after [`Assembler::setcc`].
    pub fn movzx_r64_r8(&mut self, dst: Reg, src: Reg) {
        self.emit_u8(rex(true, dst.high1(), false, src.high1()));
        self.emit_u8(0x0F);
        self.emit_u8(0xB6);
        self.emit_u8(modrm(0b11, dst.low3(), src.low3()));
    }

    /// `MOVZX r64, byte ptr [base]` — load one byte from `[base]`, zero-extend
    /// to 64 bits, store into `dst` (LANG76).
    ///
    /// Opcode `REX.W 0F B6 /r ModRM(mod=00, reg=dst, rm=base)`, 4 bytes.
    ///
    /// **Constraints on `base`:** must not be `RSP`/`R12` (low3 == 4, which
    /// would require an SIB byte) and must not be `RBP`/`R13` (low3 == 5,
    /// which `mod=00` reinterprets as RIP-relative addressing).  Callers
    /// should always pre-compute the effective address into `RAX` / `RCX`
    /// / `RDX` (low3 ∈ {0, 1, 2}) before invoking this helper.  Violating
    /// this contract is a programmer error and the function panics.
    pub fn movzx_r64_byte_at(&mut self, dst: Reg, base: Reg) {
        assert_ne!(base.low3(), 4, "movzx_r64_byte_at: RSP/R12 needs SIB — use a different base register");
        assert_ne!(base.low3(), 5, "movzx_r64_byte_at: RBP/R13 with mod=00 means RIP-relative — use a different base register");
        self.emit_u8(rex(true, dst.high1(), false, base.high1()));
        self.emit_u8(0x0F);
        self.emit_u8(0xB6);
        self.emit_u8(modrm(0b00, dst.low3(), base.low3()));
    }

    // -----------------------------------------------------------------------
    // SSE2 — scalar double-precision floating-point (LANG-FULL E3 / ALGOL `real`)
    //
    // The `Reg` numbers double as XMM register numbers (`Rax`→`xmm0`,
    // `Rcx`→`xmm1`, …) — the mandatory prefix + 0F-opcode select the XMM
    // register file. REX is emitted only when a high register (≥ 8) is used,
    // always with `W=0` (these ops don't use the 64-bit-operand bit). Every
    // encoding was verified byte-for-byte against the system assembler.
    // -----------------------------------------------------------------------

    /// Shared SSE register-register form: `<prefix> [REX] 0F <opcode> ModRM(11,dst,src)`.
    fn emit_sse_rr(&mut self, prefix: u8, opcode: u8, dst: Reg, src: Reg) {
        self.emit_u8(prefix);
        if dst.high1() || src.high1() {
            self.emit_u8(rex(false, dst.high1(), false, src.high1()));
        }
        self.emit_u8(0x0F);
        self.emit_u8(opcode);
        self.emit_u8(modrm(0b11, dst.low3(), src.low3()));
    }

    /// Shared SSE register-memory form `[base + disp32]` (always `mod=10`, so the
    /// RBP/disp=0 special case never bites). `xmm` is the ModR/M.reg operand.
    fn emit_sse_mem(&mut self, prefix: u8, opcode: u8, xmm: Reg, base: Reg, disp: i32) {
        self.emit_u8(prefix);
        if xmm.high1() || base.high1() {
            self.emit_u8(rex(false, xmm.high1(), false, base.high1()));
        }
        self.emit_u8(0x0F);
        self.emit_u8(opcode);
        let needs_sib = base.low3() == 4; // RSP / R12
        if needs_sib {
            self.emit_u8(modrm(0b10, xmm.low3(), 0b100));
            self.emit_u8((0b100 << 3) | base.low3());
        } else {
            self.emit_u8(modrm(0b10, xmm.low3(), base.low3()));
        }
        self.emit_u32_le(disp as u32);
    }

    /// `MOVSD xmm, [base + disp32]` — load a double (`F2 0F 10 /r`).
    pub fn movsd_load(&mut self, dst_xmm: Reg, base: Reg, disp: i32) {
        self.emit_sse_mem(0xF2, 0x10, dst_xmm, base, disp);
    }

    /// `MOVSD [base + disp32], xmm` — store a double (`F2 0F 11 /r`).
    pub fn movsd_store(&mut self, base: Reg, disp: i32, src_xmm: Reg) {
        self.emit_sse_mem(0xF2, 0x11, src_xmm, base, disp);
    }

    /// `ADDSD xmm_dst, xmm_src` — double add (`F2 0F 58 /r`).
    pub fn addsd(&mut self, dst: Reg, src: Reg) { self.emit_sse_rr(0xF2, 0x58, dst, src); }
    /// `SUBSD xmm_dst, xmm_src` — double subtract (`F2 0F 5C /r`).
    pub fn subsd(&mut self, dst: Reg, src: Reg) { self.emit_sse_rr(0xF2, 0x5C, dst, src); }
    /// `MULSD xmm_dst, xmm_src` — double multiply (`F2 0F 59 /r`).
    pub fn mulsd(&mut self, dst: Reg, src: Reg) { self.emit_sse_rr(0xF2, 0x59, dst, src); }
    /// `DIVSD xmm_dst, xmm_src` — double divide (`F2 0F 5E /r`). IEEE div-by-zero
    /// → `±inf`/`NaN`, never a trap.
    pub fn divsd(&mut self, dst: Reg, src: Reg) { self.emit_sse_rr(0xF2, 0x5E, dst, src); }

    /// `UCOMISD xmm_a, xmm_b` — unordered compare two doubles, set `ZF`/`PF`/`CF`
    /// (`66 0F 2E /r`). Read with `setcc` (NaN sets `PF=1`).
    pub fn ucomisd(&mut self, a: Reg, b: Reg) { self.emit_sse_rr(0x66, 0x2E, a, b); }

    // ── LANG-FULL E8: int ⇄ real conversions ───────────────────────────────
    //
    // Two encoding shapes the existing SSE helpers don't cover:
    //   * `cvtsi2sd` / `cvttsd2si` mix a 64-bit GPR with an XMM register, so
    //     they need a **mandatory REX.W** (the `emit_sse_rr` helper only adds a
    //     REX byte for high registers and never sets W).  `emit_sse_rr_w` always
    //     emits `REX.W`.  As in `emit_sse_rr`, `reg` is the ModRM.reg operand
    //     (REX.R) and `rm` is the ModRM.rm operand (REX.B).
    //   * `roundsd` is a **three-byte opcode** (`66 0F 3A 0B`) with a trailing
    //     `imm8` rounding-mode selector — `emit_sse_rri_0f3a` handles it.

    /// SSE reg/reg with a mandatory `REX.W` (64-bit operand). `reg`→ModRM.reg
    /// (REX.R), `rm`→ModRM.rm (REX.B). Layout: `prefix REX.W 0F opcode ModRM`.
    fn emit_sse_rr_w(&mut self, prefix: u8, opcode: u8, reg: Reg, rm: Reg) {
        self.emit_u8(prefix);
        self.emit_u8(rex(true, reg.high1(), false, rm.high1()));
        self.emit_u8(0x0F);
        self.emit_u8(opcode);
        self.emit_u8(modrm(0b11, reg.low3(), rm.low3()));
    }

    /// SSE reg/reg with the `66 0F 3A <opcode>` three-byte form + trailing
    /// `imm8` (used by `roundsd`). `reg`→ModRM.reg, `rm`→ModRM.rm.
    fn emit_sse_rri_0f3a(&mut self, opcode: u8, reg: Reg, rm: Reg, imm8: u8) {
        self.emit_u8(0x66);
        if reg.high1() || rm.high1() {
            self.emit_u8(rex(false, reg.high1(), false, rm.high1()));
        }
        self.emit_u8(0x0F);
        self.emit_u8(0x3A);
        self.emit_u8(opcode);
        self.emit_u8(modrm(0b11, reg.low3(), rm.low3()));
        self.emit_u8(imm8);
    }

    /// `CVTSI2SD xmm_dst, r/m64` — convert a signed 64-bit integer (GPR) to a
    /// double (XMM). Exact for |x|<2⁵³, round-to-nearest-even beyond — the IIR
    /// `int_to_real` op. `F2 REX.W 0F 2A /r` → e.g. `cvtsi2sd xmm0,rax` =
    /// `F2 48 0F 2A C0`.
    pub fn cvtsi2sd(&mut self, xmm_dst: Reg, gpr_src: Reg) {
        self.emit_sse_rr_w(0xF2, 0x2A, xmm_dst, gpr_src);
    }

    /// `CVTTSD2SI r64, xmm/m64` — convert a double (XMM) to a signed 64-bit
    /// integer (GPR), **truncating toward zero**. On NaN / ±∞ / out-of-range it
    /// yields the "integer indefinite" `0x8000_0000_0000_0000` (no trap) — a
    /// documented divergence from the VM trap, shared with JVM/aarch64. The IIR
    /// `real_to_int_trunc` op (and the tail of `real_to_int_floor`).
    /// `F2 REX.W 0F 2C /r` → e.g. `cvttsd2si rax,xmm0` = `F2 48 0F 2C C0`.
    pub fn cvttsd2si(&mut self, gpr_dst: Reg, xmm_src: Reg) {
        self.emit_sse_rr_w(0xF2, 0x2C, gpr_dst, xmm_src);
    }

    /// `ROUNDSD xmm_dst, xmm_src, imm8` — SSE4.1 round a double under the mode
    /// in `imm8` (`1` = toward −∞ / floor). Composed with `cvttsd2si` it gives
    /// the IIR `real_to_int_floor` (ALGOL `entier`). `66 0F 3A 0B /r ib` → e.g.
    /// `roundsd xmm0,xmm0,1` = `66 0F 3A 0B C0 01`.
    pub fn roundsd(&mut self, xmm_dst: Reg, xmm_src: Reg, imm8: u8) {
        self.emit_sse_rri_0f3a(0x0B, xmm_dst, xmm_src, imm8);
    }

    /// `SQRTSD xmm_dst, xmm_src` — SSE2 double-precision square root (AL8 `sqrt`).
    ///
    /// Single hardware FP instruction — no libm call.  NaN propagates; negative
    /// input returns NaN per IEEE-754 (matches Rust `f64::sqrt` and the VM handler).
    /// Opcode: `F2 0F 51 /r` → e.g. `sqrtsd xmm0,xmm0` = `F2 0F 51 C0`.
    pub fn sqrtsd(&mut self, xmm_dst: Reg, xmm_src: Reg) {
        self.emit_sse_rr(0xF2, 0x51, xmm_dst, xmm_src);
    }

    /// `MOV byte ptr [base], r8` — store the low 8 bits of `src` to `[base]`
    /// (LANG76).
    ///
    /// Opcode `REX 88 /r ModRM(mod=00, reg=src, rm=base)`, 3 bytes.  An
    /// "empty" REX prefix (`0x40`) is always emitted so the byte-register
    /// encoding is unambiguous for *any* `src` (without REX the
    /// `src.low3 ∈ {4,5,6,7}` slots map to legacy `AH`/`CH`/`DH`/`BH`
    /// instead of `SPL`/`BPL`/`SIL`/`DIL`, which we don't want).
    ///
    /// **Constraints on `base`:** same as [`movzx_r64_byte_at`] — must not
    /// be RSP/R12 (SIB) or RBP/R13 (RIP-relative).
    pub fn mov_byte_at_r8(&mut self, base: Reg, src: Reg) {
        assert_ne!(base.low3(), 4, "mov_byte_at_r8: RSP/R12 needs SIB — use a different base register");
        assert_ne!(base.low3(), 5, "mov_byte_at_r8: RBP/R13 with mod=00 means RIP-relative — use a different base register");
        // Force REX prefix so byte-reg encoding is unambiguous.
        self.emit_u8(rex(false, src.high1(), false, base.high1()));
        self.emit_u8(0x88);
        self.emit_u8(modrm(0b00, src.low3(), base.low3()));
    }

    // -----------------------------------------------------------------------
    // Stack
    // -----------------------------------------------------------------------

    /// `PUSH r64`.  Opcode `0x50+rd`.  No REX.W needed (push/pop are
    /// implicitly 64-bit in long mode); REX.B extends the register.
    pub fn push(&mut self, src: Reg) {
        if src.high1() {
            self.emit_u8(rex(false, false, false, true));
        }
        self.emit_u8(0x50 + src.low3());
    }

    /// `POP r64`.  Opcode `0x58+rd`.
    pub fn pop(&mut self, dst: Reg) {
        if dst.high1() {
            self.emit_u8(rex(false, false, false, true));
        }
        self.emit_u8(0x58 + dst.low3());
    }

    // -----------------------------------------------------------------------
    // Control flow
    // -----------------------------------------------------------------------

    /// `JMP rel32` to a label.  Opcode `0xE9 cd` (5 bytes total).
    ///
    /// Always emitted in Rel32 form per V1 policy.
    pub fn jmp(&mut self, target: LabelId) {
        self.emit_u8(0xE9);
        let slot_offset = self.code.len();
        self.emit_u32_le(0); // placeholder
        let instr_end_offset = self.code.len();
        self.fixups.push(Fixup { slot_offset, instr_end_offset, target });
    }

    /// `Jcc rel32` to a label.  Two-byte opcode `0x0F 0x80+cc` plus
    /// 4-byte displacement (6 bytes total).
    pub fn jcc(&mut self, cond: Cond, target: LabelId) {
        self.emit_u8(0x0F);
        self.emit_u8(0x80 | cond.tttn());
        let slot_offset = self.code.len();
        self.emit_u32_le(0);
        let instr_end_offset = self.code.len();
        self.fixups.push(Fixup { slot_offset, instr_end_offset, target });
    }

    /// `CALL rel32` to an *external* symbol.  Opcode `0xE8 cd` (5
    /// bytes total).  Records an external relocation for the
    /// packager to resolve.
    ///
    /// `kind` should normally be [`ExternalRelocKind::PltRel32`] for a
    /// call into the runtime or another module.
    pub fn call_rel32(&mut self, symbol: &str, kind: ExternalRelocKind) {
        self.emit_u8(0xE8);
        let patch_offset = self.code.len();
        self.emit_u32_le(0);
        self.external_relocs.push(ExternalReloc {
            patch_offset,
            symbol: symbol.to_owned(),
            kind,
            addend: -4,
        });
    }

    /// `CALL rel32` to an *internal* label (same `.text` section).
    /// Opcode `0xE8 cd`.  Used for self-recursive calls where the
    /// callee is bound to a label inside this function's bytes; the
    /// displacement is resolved at [`Assembler::finish`] time exactly
    /// like a [`Assembler::jmp`].
    ///
    /// 5 bytes total.
    pub fn call_label(&mut self, target: LabelId) {
        self.emit_u8(0xE8);
        let slot_offset = self.code.len();
        self.emit_u32_le(0);
        let instr_end_offset = self.code.len();
        self.fixups.push(Fixup { slot_offset, instr_end_offset, target });
    }

    /// `CALL r/m64` — indirect call through a register.  Opcode
    /// `0xFF /2`.
    pub fn call_r64(&mut self, target: Reg) {
        self.emit_u8(rex(false, false, false, target.high1()));
        self.emit_u8(0xFF);
        self.emit_u8(modrm(0b11, 2, target.low3()));
    }

    /// `RET` (near return).  Opcode `0xC3`.
    pub fn ret(&mut self) { self.emit_u8(0xC3); }

    // -----------------------------------------------------------------------
    // Misc
    // -----------------------------------------------------------------------

    /// `NOP`.  Opcode `0x90`.
    pub fn nop(&mut self) { self.emit_u8(0x90); }

    /// `INT3` — software breakpoint.  Opcode `0xCC`.
    pub fn int3(&mut self) { self.emit_u8(0xCC); }

    /// `UD2` — undefined-instruction trap (for `type_assert` lowering).
    /// Opcode `0x0F 0x0B`.  Two bytes.
    pub fn ud2(&mut self) {
        self.emit_u8(0x0F);
        self.emit_u8(0x0B);
    }

    // -----------------------------------------------------------------------
    // Finalisation
    // -----------------------------------------------------------------------

    /// Consume the assembler, resolve all label fix-ups, and return
    /// the final byte stream.
    ///
    /// Errors:
    /// - [`EncodeError::UnboundLabel`] if any branch references a
    ///   label that was never `bind`-ed.
    /// - [`EncodeError::BranchOutOfRange`] if a displacement does not
    ///   fit in signed 32 bits (`±2 GiB`).
    pub fn finish(mut self) -> Result<Vec<u8>, EncodeError> {
        for f in &self.fixups {
            let target = self.labels[f.target.0 as usize]
                .ok_or(EncodeError::UnboundLabel(f.target))?;
            let delta = (target as i64) - (f.instr_end_offset as i64);
            if delta < i32::MIN as i64 || delta > i32::MAX as i64 {
                return Err(EncodeError::BranchOutOfRange { delta_bytes: delta });
            }
            let bytes = (delta as i32).to_le_bytes();
            self.code[f.slot_offset    ] = bytes[0];
            self.code[f.slot_offset + 1] = bytes[1];
            self.code[f.slot_offset + 2] = bytes[2];
            self.code[f.slot_offset + 3] = bytes[3];
        }
        Ok(self.code)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn finish(a: Assembler) -> Vec<u8> { a.finish().unwrap() }

    // ---- MOV ----

    #[test]
    fn mov_rax_rdi() {
        // mov rax, rdi  →  48 89 F8
        let mut a = Assembler::new();
        a.mov_r64_r64(Reg::Rax, Reg::Rdi);
        assert_eq!(finish(a), vec![0x48, 0x89, 0xF8]);
    }

    #[test]
    fn mov_r15_r8() {
        // mov r15, r8  →  4D 89 C7
        let mut a = Assembler::new();
        a.mov_r64_r64(Reg::R15, Reg::R8);
        assert_eq!(finish(a), vec![0x4D, 0x89, 0xC7]);
    }

    #[test]
    fn mov_rax_imm32_positive() {
        // mov rax, 42  →  48 C7 C0 2A 00 00 00
        let mut a = Assembler::new();
        a.mov_r64_imm32(Reg::Rax, 42);
        assert_eq!(finish(a), vec![0x48, 0xC7, 0xC0, 0x2A, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn mov_rax_imm32_negative() {
        // mov rax, -1  →  48 C7 C0 FF FF FF FF
        let mut a = Assembler::new();
        a.mov_r64_imm32(Reg::Rax, -1);
        assert_eq!(finish(a), vec![0x48, 0xC7, 0xC0, 0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn mov_rax_imm64() {
        // movabs rax, 0x1234567890ABCDEF
        //   →  48 B8 EF CD AB 90 78 56 34 12
        let mut a = Assembler::new();
        a.mov_r64_imm64(Reg::Rax, 0x1234567890ABCDEF);
        assert_eq!(finish(a), vec![
            0x48, 0xB8, 0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12,
        ]);
    }

    #[test]
    fn mov_r10_imm64() {
        // movabs r10, 0xDEADBEEFCAFEBABE
        //   →  49 BA BE BA FE CA EF BE AD DE
        let mut a = Assembler::new();
        a.mov_r64_imm64(Reg::R10, 0xDEADBEEF_CAFEBABE);
        assert_eq!(finish(a), vec![
            0x49, 0xBA, 0xBE, 0xBA, 0xFE, 0xCA, 0xEF, 0xBE, 0xAD, 0xDE,
        ]);
    }

    // ---- Memory ----

    #[test]
    fn mov_rax_from_rbp_minus_8() {
        // mov rax, [rbp - 8]  →  48 8B 85 F8 FF FF FF
        let mut a = Assembler::new();
        a.mov_r64_mem(Reg::Rax, Reg::Rbp, -8);
        assert_eq!(finish(a), vec![0x48, 0x8B, 0x85, 0xF8, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn mov_to_rbp_minus_8_from_rdi() {
        // mov [rbp - 8], rdi  →  48 89 BD F8 FF FF FF
        let mut a = Assembler::new();
        a.mov_mem_r64(Reg::Rbp, -8, Reg::Rdi);
        assert_eq!(finish(a), vec![0x48, 0x89, 0xBD, 0xF8, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn mov_with_rsp_base_emits_sib() {
        // [rsp - 8] requires SIB byte.
        // mov rax, [rsp - 8]  →  48 8B 84 24 F8 FF FF FF
        let mut a = Assembler::new();
        a.mov_r64_mem(Reg::Rax, Reg::Rsp, -8);
        assert_eq!(finish(a), vec![0x48, 0x8B, 0x84, 0x24, 0xF8, 0xFF, 0xFF, 0xFF]);
    }

    // ---- Arithmetic ----

    #[test]
    fn add_rax_rcx() {
        // add rax, rcx  →  48 01 C8
        let mut a = Assembler::new();
        a.add(Reg::Rax, Reg::Rcx);
        assert_eq!(finish(a), vec![0x48, 0x01, 0xC8]);
    }

    #[test]
    fn sub_rax_rcx() {
        // sub rax, rcx  →  48 29 C8
        let mut a = Assembler::new();
        a.sub(Reg::Rax, Reg::Rcx);
        assert_eq!(finish(a), vec![0x48, 0x29, 0xC8]);
    }

    #[test]
    fn imul_rax_rcx() {
        // imul rax, rcx  →  48 0F AF C1
        let mut a = Assembler::new();
        a.imul(Reg::Rax, Reg::Rcx);
        assert_eq!(finish(a), vec![0x48, 0x0F, 0xAF, 0xC1]);
    }

    #[test]
    fn add_rax_imm32() {
        // add rax, 1000  →  48 81 C0 E8 03 00 00
        let mut a = Assembler::new();
        a.add_imm32(Reg::Rax, 1000);
        assert_eq!(finish(a), vec![0x48, 0x81, 0xC0, 0xE8, 0x03, 0x00, 0x00]);
    }

    #[test]
    fn neg_rax() {
        // neg rax  →  48 F7 D8
        let mut a = Assembler::new();
        a.neg_(Reg::Rax);
        assert_eq!(finish(a), vec![0x48, 0xF7, 0xD8]);
    }

    #[test]
    fn idiv_rcx() {
        // idiv rcx  →  48 F7 F9
        let mut a = Assembler::new();
        a.idiv(Reg::Rcx);
        assert_eq!(finish(a), vec![0x48, 0xF7, 0xF9]);
    }

    #[test]
    fn div_rcx() {
        // div rcx  →  48 F7 F1
        let mut a = Assembler::new();
        a.div(Reg::Rcx);
        assert_eq!(finish(a), vec![0x48, 0xF7, 0xF1]);
    }

    #[test]
    fn cqo_encodes() {
        // cqo  →  48 99
        let mut a = Assembler::new();
        a.cqo();
        assert_eq!(finish(a), vec![0x48, 0x99]);
    }

    // ---- Logical ----

    #[test]
    fn and_or_xor_test_not() {
        let mut a = Assembler::new();
        a.and_(Reg::Rax, Reg::Rcx);  // 48 21 C8
        a.or_(Reg::Rax, Reg::Rcx);   // 48 09 C8
        a.xor_(Reg::Rax, Reg::Rcx);  // 48 31 C8
        a.test_(Reg::Rax, Reg::Rcx); // 48 85 C8
        a.not_(Reg::Rax);            // 48 F7 D0
        assert_eq!(finish(a), vec![
            0x48, 0x21, 0xC8,
            0x48, 0x09, 0xC8,
            0x48, 0x31, 0xC8,
            0x48, 0x85, 0xC8,
            0x48, 0xF7, 0xD0,
        ]);
    }

    // ---- Shifts ----

    #[test]
    fn shl_cl_rax() {
        // shl rax, cl  →  48 D3 E0
        let mut a = Assembler::new();
        a.shl_cl(Reg::Rax);
        assert_eq!(finish(a), vec![0x48, 0xD3, 0xE0]);
    }

    #[test]
    fn shr_cl_rax() {
        // shr rax, cl  →  48 D3 E8
        let mut a = Assembler::new();
        a.shr_cl(Reg::Rax);
        assert_eq!(finish(a), vec![0x48, 0xD3, 0xE8]);
    }

    #[test]
    fn sar_cl_rax() {
        // sar rax, cl  →  48 D3 F8
        let mut a = Assembler::new();
        a.sar_cl(Reg::Rax);
        assert_eq!(finish(a), vec![0x48, 0xD3, 0xF8]);
    }

    #[test]
    fn shl_imm_rax_3() {
        // shl rax, 3  →  48 C1 E0 03
        let mut a = Assembler::new();
        a.shl_imm8(Reg::Rax, 3);
        assert_eq!(finish(a), vec![0x48, 0xC1, 0xE0, 0x03]);
    }

    // ---- Compare + set ----

    #[test]
    fn cmp_rax_rcx() {
        // cmp rax, rcx  →  48 39 C8
        let mut a = Assembler::new();
        a.cmp(Reg::Rax, Reg::Rcx);
        assert_eq!(finish(a), vec![0x48, 0x39, 0xC8]);
    }

    #[test]
    fn setcc_e_al() {
        // sete al  →  40 0F 94 C0
        // (REX present uniformly per encoder policy)
        let mut a = Assembler::new();
        a.setcc(Cond::E, Reg::Rax);
        assert_eq!(finish(a), vec![0x40, 0x0F, 0x94, 0xC0]);
    }

    #[test]
    fn movzx_rax_al() {
        // movzx rax, al  →  48 0F B6 C0
        let mut a = Assembler::new();
        a.movzx_r64_r8(Reg::Rax, Reg::Rax);
        assert_eq!(finish(a), vec![0x48, 0x0F, 0xB6, 0xC0]);
    }

    // ---- SSE2 scalar double (LANG-FULL E3) ----
    // Reg-reg opcodes verified byte-identical against the system assembler
    // (`clang -masm=intel`); the mem forms use this encoder's disp32 policy
    // (the assembler picks disp8 for small offsets — semantically identical).

    #[test]
    fn sse_movsd_load_store_rbp_8() {
        // movsd xmm0, [rbp+8] → F2 0F 10 85 08000000 ; store → F2 0F 11 85 …
        let mut a = Assembler::new();
        a.movsd_load(Reg::Rax, Reg::Rbp, 8);   // xmm0
        a.movsd_store(Reg::Rbp, 8, Reg::Rax);
        assert_eq!(finish(a), vec![
            0xF2, 0x0F, 0x10, 0x85, 0x08, 0x00, 0x00, 0x00,
            0xF2, 0x0F, 0x11, 0x85, 0x08, 0x00, 0x00, 0x00,
        ]);
    }

    #[test]
    fn sse_arith_xmm0_xmm1() {
        let mut a = Assembler::new();
        a.addsd(Reg::Rax, Reg::Rcx); // xmm0, xmm1
        a.subsd(Reg::Rax, Reg::Rcx);
        a.mulsd(Reg::Rax, Reg::Rcx);
        a.divsd(Reg::Rax, Reg::Rcx);
        assert_eq!(finish(a), vec![
            0xF2, 0x0F, 0x58, 0xC1, // addsd xmm0, xmm1
            0xF2, 0x0F, 0x5C, 0xC1, // subsd
            0xF2, 0x0F, 0x59, 0xC1, // mulsd
            0xF2, 0x0F, 0x5E, 0xC1, // divsd
        ]);
    }

    #[test]
    fn sse_ucomisd_xmm0_xmm1() {
        // ucomisd xmm0, xmm1 → 66 0F 2E C1
        let mut a = Assembler::new();
        a.ucomisd(Reg::Rax, Reg::Rcx);
        assert_eq!(finish(a), vec![0x66, 0x0F, 0x2E, 0xC1]);
    }

    // ---- LANG-FULL E8: int ⇄ real conversions ----

    #[test]
    fn sse_conversions_e8_base() {
        // Base (xmm0 / rax) encodings, as documented on each method.
        let mut a = Assembler::new();
        a.cvtsi2sd(Reg::Rax, Reg::Rax);   // xmm0, rax  → F2 48 0F 2A C0
        a.cvttsd2si(Reg::Rax, Reg::Rax);  // rax, xmm0  → F2 48 0F 2C C0
        a.roundsd(Reg::Rax, Reg::Rax, 1); // xmm0,xmm0,1 → 66 0F 3A 0B C0 01
        assert_eq!(finish(a), vec![
            0xF2, 0x48, 0x0F, 0x2A, 0xC0,
            0xF2, 0x48, 0x0F, 0x2C, 0xC0,
            0x66, 0x0F, 0x3A, 0x0B, 0xC0, 0x01,
        ]);
    }

    #[test]
    fn sse_conversions_e8_reg_fields() {
        // Verify ModRM.reg / ModRM.rm placement and REX bit routing with a mix
        // of registers (Rcx = idx 1, R8 = idx 8 → REX.B/REX.R extension bit).
        let mut a = Assembler::new();
        // cvtsi2sd xmm1, r8 : reg=xmm1(low3=1), rm=r8(low3=0,high1) → REX.W+B,
        //   ModRM = 11 001 000 = 0xC8 → F2 49 0F 2A C8
        a.cvtsi2sd(Reg::Rcx, Reg::R8);
        // cvttsd2si r8, xmm1 : reg=r8(low3=0,high1→REX.R), rm=xmm1(low3=1) →
        //   REX.W+R, ModRM = 11 000 001 = 0xC1 → F2 4C 0F 2C C1
        a.cvttsd2si(Reg::R8, Reg::Rcx);
        assert_eq!(finish(a), vec![
            0xF2, 0x49, 0x0F, 0x2A, 0xC8,
            0xF2, 0x4C, 0x0F, 0x2C, 0xC1,
        ]);
    }

    // ---- Stack ----

    #[test]
    fn push_pop_rbp() {
        // push rbp  →  55
        // pop  rbp  →  5D
        let mut a = Assembler::new();
        a.push(Reg::Rbp);
        a.pop(Reg::Rbp);
        assert_eq!(finish(a), vec![0x55, 0x5D]);
    }

    #[test]
    fn push_r15() {
        // push r15  →  41 57
        let mut a = Assembler::new();
        a.push(Reg::R15);
        assert_eq!(finish(a), vec![0x41, 0x57]);
    }

    // ---- Control flow ----

    #[test]
    fn jmp_forward_label() {
        // jmp fwd            →  E9 02 00 00 00     (skip 2 bytes)
        // nop; nop           →  90 90
        // bind(fwd); ret     →  C3
        let mut a = Assembler::new();
        let fwd = a.create_label();
        a.jmp(fwd);
        a.nop();
        a.nop();
        a.bind(fwd).unwrap();
        a.ret();
        assert_eq!(finish(a), vec![
            0xE9, 0x02, 0x00, 0x00, 0x00,
            0x90, 0x90,
            0xC3,
        ]);
    }

    #[test]
    fn jcc_backward_label() {
        // bind(top); nop          →  90       (top = offset 0)
        // jne top                 →  0F 85 FA FF FF FF    (delta = -6)
        let mut a = Assembler::new();
        let top = a.create_label();
        a.bind(top).unwrap();
        a.nop();
        a.jcc(Cond::Ne, top);
        let bytes = finish(a);
        assert_eq!(&bytes[0..2], &[0x90, 0x0F]);
        assert_eq!(bytes[2], 0x85);
        let disp = i32::from_le_bytes([bytes[3], bytes[4], bytes[5], bytes[6]]);
        // Instruction ends at byte 7; target is byte 0 → delta -7.
        assert_eq!(disp, -7);
    }

    #[test]
    fn unbound_label_errors() {
        let mut a = Assembler::new();
        let l = a.create_label();
        a.jmp(l);
        match a.finish() {
            Err(EncodeError::UnboundLabel(_)) => {},
            other => panic!("expected UnboundLabel, got {:?}", other),
        }
    }

    #[test]
    fn double_bind_errors() {
        let mut a = Assembler::new();
        let l = a.create_label();
        a.bind(l).unwrap();
        match a.bind(l) {
            Err(EncodeError::LabelAlreadyBound(_)) => {},
            other => panic!("expected LabelAlreadyBound, got {:?}", other),
        }
    }

    // ---- Calls ----

    #[test]
    fn call_rel32_records_reloc() {
        // call __twig_print_i64   →  E8 00 00 00 00
        let mut a = Assembler::new();
        a.call_rel32("__twig_print_i64", ExternalRelocKind::PltRel32);
        // Assert the bytes pre-finish (call_rel32 doesn't create a fixup)
        let relocs = a.external_relocs.clone();
        assert_eq!(finish(a), vec![0xE8, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(relocs.len(), 1);
        assert_eq!(relocs[0].symbol, "__twig_print_i64");
        assert_eq!(relocs[0].kind, ExternalRelocKind::PltRel32);
        assert_eq!(relocs[0].addend, -4);
        assert_eq!(relocs[0].patch_offset, 1);
    }

    #[test]
    fn call_label_back_edge() {
        // bind(top); nop                      → 90       (top = offset 0)
        // call top                            → E8 ?? ?? ?? ??   (rel32 from instr end)
        let mut a = Assembler::new();
        let top = a.create_label();
        a.bind(top).unwrap();
        a.nop();
        a.call_label(top);
        let bytes = finish(a);
        // Bytes: 90 E8 disp32
        assert_eq!(&bytes[..2], &[0x90, 0xE8]);
        let disp = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        // Instruction ends at byte 6; target is byte 0 → delta = -6.
        assert_eq!(disp, -6);
    }

    #[test]
    fn call_r64_indirect() {
        // call rax  →  FF D0  (REX optional — encoder emits 40 prefix
        //                       only when the register has the high bit)
        let mut a = Assembler::new();
        a.call_r64(Reg::Rax);
        assert_eq!(finish(a), vec![0x40, 0xFF, 0xD0]);
    }

    #[test]
    fn ret_one_byte() {
        let mut a = Assembler::new();
        a.ret();
        assert_eq!(finish(a), vec![0xC3]);
    }

    // ---- Misc ----

    #[test]
    fn ud2_encodes() {
        let mut a = Assembler::new();
        a.ud2();
        assert_eq!(finish(a), vec![0x0F, 0x0B]);
    }

    #[test]
    fn lea_rip_rel_records_reloc() {
        // lea rax, [rip + 0]  →  48 8D 05 00 00 00 00
        let mut a = Assembler::new();
        a.lea_rip_rel(Reg::Rax, "_twig_globals", ExternalRelocKind::PcRel32);
        let relocs = a.external_relocs.clone();
        assert_eq!(finish(a), vec![0x48, 0x8D, 0x05, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(relocs.len(), 1);
        assert_eq!(relocs[0].symbol, "_twig_globals");
        assert_eq!(relocs[0].kind, ExternalRelocKind::PcRel32);
        assert_eq!(relocs[0].patch_offset, 3);
    }

    // ---- Worked example: simple function ----

    #[test]
    fn function_a_plus_b_sysv() {
        // System V: arg 0 in RDI, arg 1 in RSI, return in RAX.
        //   mov rax, rdi    48 89 F8
        //   add rax, rsi    48 01 F0
        //   ret             C3
        let mut a = Assembler::new();
        a.mov_r64_r64(Reg::Rax, Reg::Rdi);
        a.add(Reg::Rax, Reg::Rsi);
        a.ret();
        assert_eq!(finish(a), vec![
            0x48, 0x89, 0xF8,
            0x48, 0x01, 0xF0,
            0xC3,
        ]);
    }

    #[test]
    fn function_a_plus_b_msx64() {
        // MS x64: arg 0 in RCX, arg 1 in RDX, return in RAX.
        //   mov rax, rcx    48 89 C8
        //   add rax, rdx    48 01 D0
        //   ret             C3
        let mut a = Assembler::new();
        a.mov_r64_r64(Reg::Rax, Reg::Rcx);
        a.add(Reg::Rax, Reg::Rdx);
        a.ret();
        assert_eq!(finish(a), vec![
            0x48, 0x89, 0xC8,
            0x48, 0x01, 0xD0,
            0xC3,
        ]);
    }

    // ---- RIP-relative lea (GC stack-map registration) ----

    #[test]
    fn lea_rip_label_resolves_to_embedded_data() {
        // lea rcx, [rip + <data>]  =  48 8D 0D <disp32>
        // ret ; then a data word the lea points at.
        let mut a = Assembler::new();
        let lbl = a.create_label();
        a.lea_rip_label(Reg::Rcx, lbl);
        a.ret();
        a.bind(lbl).unwrap();
        a.emit_data_u32(0xDEAD_BEEF);
        let bytes = finish(a);
        assert_eq!(&bytes[0..3], &[0x48, 0x8D, 0x0D], "REX.W lea rcx, [rip+...]");
        // The lea is 7 bytes; instr_end = 7; the data label is bound at offset 8 (after
        // the 1-byte ret). disp32 = 8 - 7 = 1.
        assert_eq!(&bytes[3..7], &1i32.to_le_bytes(), "disp32 = data_off - instr_end");
        assert_eq!(&bytes[7..8], &[0xC3], "ret");
        assert_eq!(&bytes[8..12], &0xDEAD_BEEFu32.to_le_bytes(), "data word verbatim");
    }

    #[test]
    fn lea_rip_placeholder_returns_disp_slot() {
        // lea rdi, [rip + #0]  =  48 8D 3D 00 00 00 00 ; slot at offset 3.
        let mut a = Assembler::new();
        let slot = a.lea_rip_placeholder(Reg::Rdi);
        assert_eq!(slot, 3, "disp32 slot is 3 bytes into the instruction");
        let bytes = finish(a);
        assert_eq!(bytes, vec![0x48, 0x8D, 0x3D, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn iced_roundtrip_rip_lea() {
        use iced_x86::{Decoder, DecoderOptions, Formatter, IntelFormatter};
        let mut a = Assembler::new();
        let lbl = a.create_label();
        a.lea_rip_label(Reg::Rsi, lbl);
        a.ret();
        a.bind(lbl).unwrap();
        a.emit_data_u32(0);
        let bytes = finish(a);
        let mut decoder = Decoder::with_ip(64, &bytes, 0, DecoderOptions::NONE);
        let mut f = IntelFormatter::new();
        let mut s = String::new();
        f.format(&decoder.decode(), &mut s);
        assert!(s.starts_with("lea "), "decodes as lea, got {s}");
        assert!(s.contains("rsi"), "targets rsi, got {s}");
    }

    // ---- Round-trip decode via iced-x86 ----
    //
    // Catches encoding regressions by decoding the byte stream we emit
    // and asserting the mnemonic + operand string matches what we
    // expect.  Belt-and-braces alongside byte-exact tests.

    #[test]
    fn iced_roundtrip_add_imul_ret() {
        use iced_x86::{Decoder, DecoderOptions, Formatter, IntelFormatter};

        let mut a = Assembler::new();
        a.mov_r64_r64(Reg::Rax, Reg::Rdi);
        a.imul(Reg::Rax, Reg::Rsi);
        a.add_imm32(Reg::Rax, 7);
        a.ret();
        let bytes = finish(a);

        let mut decoder = Decoder::with_ip(64, &bytes, 0, DecoderOptions::NONE);
        let mut formatter = IntelFormatter::new();
        let mut decoded: Vec<String> = Vec::new();
        for instr in &mut decoder {
            let mut s = String::new();
            formatter.format(&instr, &mut s);
            decoded.push(s);
        }
        // iced prints qword sizes, hex immediates, etc. — just check
        // the mnemonic prefixes.
        assert!(decoded[0].starts_with("mov "));
        assert!(decoded[1].starts_with("imul "));
        assert!(decoded[2].starts_with("add "));
        assert_eq!(decoded[3], "ret");
    }
}
