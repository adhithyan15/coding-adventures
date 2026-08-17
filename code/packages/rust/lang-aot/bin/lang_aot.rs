//! `lang-aot` CLI — multi-language AOT driver.
//!
//! ## Usage
//!
//! ```text
//! lang-aot <FILE> [-o <OUT>] [--lang <LANG>]
//! lang-aot --help
//! ```
//!
//! `--lang` is optional: if omitted, the language is inferred from the
//! input's file extension (`.twig`, `.nib`, `.bf`, `.bas`, `.oct`,
//! `.mcl`/`.lisp`, `.algol`/`.alg`/`.a60`).
//! When inference fails, the CLI prints the recognised extensions and
//! exits non-zero.
//!
//! The output target is **always** the build host's platform — same
//! V1 host-targets-host policy as `twig-aot`.  Use `twig-aot` (or
//! `lang-aot` once `--target`/`--emit-object` land here) for cross-OS
//! workflows.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lang_aot::{detect_language_from_path, Language};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let cmd = parse_args(&args);
    let cmd = match cmd {
        Ok(c) => c,
        Err(e) => { eprintln!("lang-aot: {e}"); return ExitCode::from(2); }
    };

    if cmd.help {
        print_help();
        return ExitCode::SUCCESS;
    }

    let input = match cmd.input {
        Some(p) => p,
        None => { eprintln!("lang-aot: missing input file"); print_help(); return ExitCode::from(2); }
    };
    // Default output extension depends on emit mode:
    //   * Native       → strip extension (foo.bas → foo)
    //   * LlvmIr       → .ll  (foo.bas → foo.ll),  matching `llc` input convention
    //   * Riscv32Bin   → .bin (foo.bas → foo.bin), the conventional flat ELF-less name
    //   * Intel8008Bin → .bin (foo.oct → foo.bin), shares the `.bin` convention with RV32I
    //   * Armv7Bin     → .bin (foo.twig → foo.bin), shares the `.bin` convention
    //   * Intel4004Bin → .bin (foo.bf → foo.bin), shares the `.bin` convention
    //   * Arm1Bin       → .bin (foo.twig → foo.bin), shares the `.bin` convention
    //   * Mos6502Bin    → .bin (foo.twig → foo.bin), shares the `.bin` convention
    //   * M68kBin       → .bin (foo.twig → foo.bin), shares the `.bin` convention
    //   * Intel8080Bin → .bin (foo.twig → foo.bin), shares the `.bin` convention
    //   * Z80Bin        → .bin (foo.twig → foo.bin), shares the `.bin` convention
    //   * Intel8086Bin  → .bin (foo.twig → foo.bin), shares the `.bin` convention
    let output = cmd.output.unwrap_or_else(|| match cmd.emit {
        EmitMode::Native       => input.with_extension(""),
        EmitMode::LlvmIr       => input.with_extension("ll"),
        EmitMode::Riscv32Bin   => input.with_extension("bin"),
        EmitMode::Intel8008Bin => input.with_extension("bin"),
        EmitMode::Armv7Bin     => input.with_extension("bin"),
        EmitMode::Intel4004Bin => input.with_extension("bin"),
        EmitMode::Ge225Bin => input.with_extension("bin"),
        EmitMode::Ibm704Bin => input.with_extension("bin"),
        EmitMode::Arm1Bin => input.with_extension("bin"),
        EmitMode::Mos6502Bin => input.with_extension("bin"),
        EmitMode::M68kBin => input.with_extension("bin"),
        EmitMode::Intel8080Bin => input.with_extension("bin"),
        EmitMode::MipsR2000Bin => input.with_extension("bin"),
        EmitMode::Z80Bin => input.with_extension("bin"),
        EmitMode::Intel8051Bin => input.with_extension("bin"),
        EmitMode::Intel8086Bin => input.with_extension("bin"),
    });

    let language = match cmd.language {
        Some(l) => l,
        None => match detect_language_from_path(&input) {
            Some(l) => l,
            None => {
                eprintln!("lang-aot: could not infer language from {:?}; pass --lang explicitly", input);
                eprintln!("  recognised extensions: .twig .nib .bf .bas .oct .mcl .lisp .algol .alg .a60");
                return ExitCode::from(2);
            }
        },
    };

    match dispatch(&input, &output, language, cmd.emit) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("lang-aot: {e}"); ExitCode::from(1) }
    }
}

/// Choice of emission target.
///
/// `Native` is the default and matches the pre-LLVM04 behaviour: produce a
/// native executable for the build host (Linux ELF / Windows PE / macOS
/// Mach-O).  `LlvmIr` produces a `.ll` textual LLVM IR file; the LLVM
/// toolchain (`llc` / `opt`) is the caller's job to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmitMode {
    Native,
    LlvmIr,
    /// Flat `.bin` of little-endian 32-bit RV32I words via `iir-to-riscv`.
    /// Cross-platform (no host gating).  Downstream consumers: the
    /// in-tree `riscv-simulator`, `qemu-riscv32`, or a flash loader.
    Riscv32Bin,
    /// Flat `.bin` of 8-bit Intel 8008 opcode bytes via
    /// `iir-to-intel8008`.  Cross-platform.  Downstream consumers:
    /// the in-tree `intel8008-simulator`, an external 8008 emulator,
    /// or a 1702 EPROM burner.  Oct's native target — its IIR is
    /// designed to round-trip through 8008 silicon.
    Intel8008Bin,
    /// Flat `.bin` of little-endian 32-bit ARMv7-A (A32) instruction
    /// words via `iir-to-armv7`.  Cross-platform.  Downstream
    /// consumers: the in-tree `arm-simulator`, `qemu-arm`,
    /// `objcopy` + a phone-class Linux linker, or a Cortex-A7/A8/A9-
    /// era SoC flash loader.  Phone-class target — billions of
    /// deployed silicon units.
    Armv7Bin,
    /// Flat `.bin` of 1- or 2-byte Intel 4004 opcodes via
    /// `iir-to-intel4004`.  Cross-platform.  Downstream consumers:
    /// any 4004 simulator, `intel-4004-assembler` for round-trip,
    /// or an EPROM burner for a 4004 dev board.  The 4004 (1971)
    /// is the world's first commercial microprocessor.
    Intel4004Bin,
    /// Flat `.bin` of 20-bit GE-225 instruction words via
    /// `iir-to-ge225`, packed as 3 bytes per word (big-endian, top
    /// 4 bits of byte 0 always zero).  Cross-platform.  Downstream
    /// consumers: any GE-225 simulator or a custom 3-byte-per-word
    /// decoder.  The GE-225 (1959) was the mainframe at Dartmouth
    /// College where Dartmouth BASIC was DESIGNED in 1964.
    Ge225Bin,
    /// Flat `.bin` of 36-bit IBM 704 instruction words, packed as
    /// 5 bytes per word (low byte first, high 4 bits of the top
    /// byte always zero).  Cross-platform.  Downstream consumers:
    /// any IBM 704 emulator, period scholarship, replica hardware.
    /// The IBM 704 (1954) is the vacuum-tube mainframe McCarthy's
    /// students ran the FIRST LISP implementation on at MIT in
    /// 1959 — the closing half of the **CAR/CDR birthplace
    /// round-trip**.
    Ibm704Bin,
    /// Flat `.bin` of little-endian 32-bit ARM1 (ARMv1) instruction
    /// words via `arm1-backend`.  Cross-platform.  Downstream
    /// consumers: the in-tree `arm1-simulator` or any external
    /// ARM1/ARMv1 emulator.  The ARM1 (1985) is Sophie Wilson and
    /// Steve Furber's original Acorn RISC Machine — the first
    /// commercially successful RISC chip and architectural ancestor
    /// of the already-migrated ARMv7 lane.
    Arm1Bin,
    /// Flat `.bin` of MOS 6502 opcode bytes via `mos6502-backend`.
    /// Cross-platform.  Downstream consumers: the in-tree
    /// `mos6502-simulator` or any external MOS 6502/NMOS emulator.  The
    /// MOS 6502 (1975) is Chuck Peddle's $25 chip that powered the Apple
    /// II, Commodore 64, Atari 8-bit line, BBC Micro, and (via the Ricoh
    /// 2A03 variant) the NES/Famicom — a byte-oriented ISA, unlike every
    /// other target above (no word endianness to flatten).
    Mos6502Bin,
    /// Flat `.bin` of big-endian Motorola 68000 machine code bytes via
    /// `m68k-backend`.  Cross-platform.  Downstream consumers: the
    /// in-tree `m68k-simulator` or any external Motorola 68000 emulator.
    /// The 68000 (1979) is the landmark 16/32-bit processor behind the
    /// original Apple Macintosh, Commodore Amiga, Atari ST, early Sun
    /// workstations, and the Sega Genesis/Mega Drive — unlike every
    /// little-endian target above (ARM1, MIPS R2000, RV32I), the 68000
    /// is big-endian, so `m68k-encoder`'s bytes are already the wire
    /// format with no flattening step, same simplicity as MOS 6502 (for
    /// a different reason — the 6502 has no word endianness at all).
    M68kBin,
    /// Flat `.bin` of variable-length (1/2/3-byte) Intel 8080 opcode
    /// bytes via `intel8080-backend`.  Cross-platform.  Downstream
    /// consumers: the in-tree `intel8080-simulator`, an external 8080
    /// emulator, or an EPROM burner.  Third lane of the
    /// 9-architecture expansion — the 8080 (1974) is the 8008's
    /// direct successor and the CPU inside the Altair 8800 that
    /// launched the personal-computer era.
    Intel8080Bin,
    /// Flat `.bin` of 32-bit MIPS R2000 instruction words via
    /// `mips-r2000-backend`, encoded **big-endian** (MIPS R2000's
    /// default byte order — unlike RISC-V/ARMv7/x86, which are
    /// little-endian).  Cross-platform.  Downstream consumers: the
    /// in-tree `mips-r2000-simulator` or any external MIPS R2000/MIPS I
    /// emulator.  The MIPS R2000 (1985) is the first commercially
    /// successful RISC processor — used in SGI IRIS workstations, DEC
    /// DECstation, the original PlayStation, and the Nintendo 64.
    /// First lane of the 9-architecture expansion following the
    /// historical-arch backend migration pattern.
    MipsR2000Bin,
    /// Flat `.bin` of 1- to 4-byte Zilog Z80 opcode sequences via
    /// `z80-backend`.  Cross-platform.  Downstream consumers: the
    /// in-tree `z80-simulator` or any external Z80 emulator.  The Z80
    /// (1976) is a source/binary-compatible superset of the Intel
    /// 8080 that powered the TRS-80, ZX Spectrum, MSX, the original
    /// Game Boy (via a variant core), and countless CP/M machines —
    /// one of the most widely produced microprocessors ever.
    Z80Bin,
    /// Flat `.bin` of 8-bit Intel 8051 (MCS-51) opcode bytes via
    /// `intel8051-backend`.  Cross-platform.  Downstream consumers:
    /// the in-tree `intel8051-simulator`, an external MCS-51
    /// emulator/in-circuit debugger, or a flash/EPROM burner for a
    /// real 8051-family part.  The 8051 (1980) is the most-
    /// manufactured CPU architecture in history — over 20 billion
    /// units, still fabricated today (Atmel/Microchip AT89, NXP
    /// 80C51, and others).
    Intel8051Bin,
    /// Flat `.bin` of Intel 8086 machine code bytes via
    /// `intel8086-backend`.  Cross-platform.  Downstream consumers: the
    /// in-tree `intel8086-simulator` or any external 8086/8088
    /// emulator.  The Intel 8086 (1978) is the direct architectural
    /// ancestor of every x86 CPU made today — its cheaper sibling the
    /// 8088 shipped in the original IBM PC (1981), founding the
    /// "PC-compatible" industry.  Ninth and final lane of the
    /// 9-architecture expansion.
    Intel8086Bin,
}

struct CliArgs {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    language: Option<Language>,
    emit: EmitMode,
    help: bool,
}

fn parse_args(args: &[String]) -> Result<CliArgs, String> {
    let mut input = None;
    let mut output = None;
    let mut language = None;
    let mut emit = EmitMode::Native;
    let mut help = false;
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-h" | "--help" => help = true,
            "-o" | "--output" => {
                i += 1;
                let v = args.get(i).ok_or_else(|| "-o requires a value".to_string())?;
                output = Some(PathBuf::from(v));
            }
            s if s.starts_with("--output=") => {
                output = Some(PathBuf::from(&s["--output=".len()..]));
            }
            "-l" | "--lang" => {
                i += 1;
                let v = args.get(i).ok_or_else(|| "--lang requires a value".to_string())?;
                language = Some(Language::parse(v)?);
            }
            s if s.starts_with("--lang=") => {
                language = Some(Language::parse(&s["--lang=".len()..])?);
            }
            // --emit=<native|llvm-ir>  (LLVM04)
            //
            // We accept `llvm-ir` as the canonical spelling (matches the
            // file extension downstream — `foo.ll` files are "LLVM IR").
            // `native` is included for explicitness even though it's the
            // default.
            s if s.starts_with("--emit=") => {
                emit = parse_emit_value(&s["--emit=".len()..])?;
            }
            "--emit" => {
                i += 1;
                let v = args.get(i).ok_or_else(|| "--emit requires a value".to_string())?;
                emit = parse_emit_value(v)?;
            }
            s if s.starts_with('-') => {
                return Err(format!("unknown flag {s:?}"));
            }
            _ => {
                if input.is_some() {
                    return Err(format!("unexpected extra argument {arg:?}"));
                }
                input = Some(PathBuf::from(arg));
            }
        }
        i += 1;
    }
    Ok(CliArgs { input, output, language, emit, help })
}

/// Parse the `<MODE>` argument of `--emit=<MODE>`.
///
/// Accepts a couple of friendly aliases per mode so users don't need to
/// remember the canonical spelling.
fn parse_emit_value(v: &str) -> Result<EmitMode, String> {
    match v {
        "native"                      => Ok(EmitMode::Native),
        "llvm-ir" | "llvm" | "ll"     => Ok(EmitMode::LlvmIr),
        "riscv32" | "rv32" | "bin"    => Ok(EmitMode::Riscv32Bin),
        "intel8008" | "i8008" | "8008" => Ok(EmitMode::Intel8008Bin),
        "armv7" | "arm" | "arm32" => Ok(EmitMode::Armv7Bin),
        "intel4004" | "i4004" | "4004" => Ok(EmitMode::Intel4004Bin),
        "ge225" | "ge-225" | "225" => Ok(EmitMode::Ge225Bin),
        "ibm704" | "ibm-704" | "704" => Ok(EmitMode::Ibm704Bin),
        "arm1" | "armv1" | "arm-1" => Ok(EmitMode::Arm1Bin),
        "mos6502" | "6502" | "mos-6502" => Ok(EmitMode::Mos6502Bin),
        "m68k" | "68000" | "mc68000" | "motorola68000" => Ok(EmitMode::M68kBin),
        "intel8080" | "i8080" | "8080" => Ok(EmitMode::Intel8080Bin),
        "mips-r2000" | "mips" | "r2000" => Ok(EmitMode::MipsR2000Bin),
        "z80" | "zilog-z80" => Ok(EmitMode::Z80Bin),
        "intel8051" | "i8051" | "8051" | "mcs51" => Ok(EmitMode::Intel8051Bin),
        "intel8086" | "i8086" | "8086" => Ok(EmitMode::Intel8086Bin),
        other => Err(format!(
            "unknown --emit value {other:?}; expected one of: \
             native | llvm-ir | riscv32 | intel8008 | armv7 | intel4004 | ge225 | ibm704 | arm1 | mos6502 | m68k | intel8080 | mips-r2000 | z80 | intel8051 | intel8086"
        )),
    }
}

fn print_help() {
    println!("\
Usage: lang-aot <FILE> [-o <OUT>] [--lang <LANG>]

Compile a source file in one of the supported LANG VM languages to a
native executable on the build host's platform.

Supported languages:
  twig            (.twig)        — full
  nib             (.nib)         — full
  brainfuck / bf  (.bf, .b)      — full
  dartmouth-basic (.bas, .basic) — TODO (no IIR frontend yet)
  oct             (.oct)         — TODO (no Rust frontend yet)
  mccarthy-lisp   (.mcl, .lisp)  — full IIR; scalar programs run on every
                                   AOT target (symbol/cons backend support: WIP)
  algol60         (.algol, .alg, .a60)
                                  — scalar integer/boolean subset over the
                                    shared LANG VM IIR

Options:
  -o, --output <PATH>      Output path. Default: input without extension
                           (native) or with .ll extension (--emit=llvm-ir).
  -l, --lang <LANG>        Override language detection
                           (twig, nib, bf, basic, oct, mccarthy-lisp/mcl/lisp,
                            algol60/algol/a60).
      --emit=<MODE>        What to emit:
                             native           → host executable (default)
                             llvm-ir | llvm | ll
                                              → textual LLVM IR (.ll) via iir-to-llvm;
                                                cross-platform; pipe to `llc` downstream
                             riscv32 | rv32 | bin
                                              → flat .bin of little-endian RV32I words
                                                via iir-to-riscv; cross-platform; load
                                                into riscv-simulator or qemu-riscv32
                             intel8008 | i8008 | 8008
                                              → flat .bin of 8-bit Intel 8008 opcodes
                                                via iir-to-intel8008; cross-platform;
                                                load into intel8008-simulator or burn
                                                to a 1702 EPROM (Oct's native target)
                             armv7 | arm | arm32
                                              → flat .bin of little-endian 32-bit
                                                ARMv7-A instruction words via
                                                iir-to-armv7; cross-platform; load
                                                into arm-simulator, qemu-arm, or
                                                objcopy + a phone-class Linux linker
                                                (Cortex-A7/A8/A9-era SoCs)
                             intel4004 | i4004 | 4004
                                              → flat .bin of 1- or 2-byte Intel 4004
                                                opcodes via iir-to-intel4004; cross-
                                                platform; load into a 4004 simulator
                                                or burn to an EPROM (the world's
                                                first commercial microprocessor, 1971)
                             ge225 | ge-225 | 225
                                              → flat .bin of 20-bit GE-225 instruction
                                                words via iir-to-ge225 (packed 3 bytes
                                                per word, big-endian, top 4 bits zero);
                                                cross-platform; load into a GE-225
                                                simulator or decode 3 bytes at a time
                                                (the mainframe where Dartmouth BASIC
                                                was DESIGNED in 1964)
                             ibm704 | ibm-704 | 704
                                              → flat .bin of 36-bit IBM 704 instruction
                                                words via ibm704-backend (packed 5 bytes
                                                per word, low byte first, top 4 bits of
                                                the high byte zero); cross-platform; the
                                                silicon Lisp was BORN on at MIT in 1959
                                                — the birthplace round-trip (CAR/CDR
                                                are literal 704 instruction-field
                                                mnemonics)
                             arm1 | armv1 | arm-1
                                              → flat .bin of little-endian 32-bit
                                                ARM1 (ARMv1) instruction words via
                                                arm1-backend; cross-platform; load
                                                into arm1-simulator or an external
                                                ARM1/ARMv1 emulator (Sophie Wilson
                                                and Steve Furber's original Acorn
                                                RISC Machine, 1985 — architectural
                                                ancestor of the ARMv7 lane above)
                             mos6502 | 6502 | mos-6502
                                              → flat .bin of MOS 6502 opcode bytes
                                                via mos6502-backend; cross-platform;
                                                load into mos6502-simulator or an
                                                external MOS 6502/NMOS emulator
                                                (Chuck Peddle's $25 chip, 1975 —
                                                Apple II, Commodore 64, Atari 8-bit,
                                                BBC Micro, and — via the Ricoh 2A03
                                                variant — the NES/Famicom)
                             m68k | 68000 | mc68000 | motorola68000
                                              → flat .bin of big-endian Motorola
                                                68000 machine code bytes via
                                                m68k-backend; cross-platform; load
                                                into m68k-simulator or an external
                                                Motorola 68000 emulator (the
                                                landmark 16/32-bit processor, 1979
                                                — original Macintosh, Commodore
                                                Amiga, Atari ST, early Sun
                                                workstations, Sega Genesis)
                             intel8080 | i8080 | 8080
                                              → flat .bin of variable-length (1/2/3-byte)
                                                Intel 8080 opcodes via intel8080-backend;
                                                cross-platform; load into
                                                intel8080-simulator or an external 8080
                                                emulator (the 8008's direct successor;
                                                CPU of the Altair 8800, 1974)
                             mips-r2000 | mips | r2000
                                              → flat .bin of big-endian 32-bit
                                                MIPS R2000 instruction words via
                                                mips-r2000-backend; cross-
                                                platform; load into
                                                mips-r2000-simulator (the first
                                                commercially successful RISC
                                                processor, 1985 — SGI IRIS, DEC
                                                DECstation, PlayStation, N64)
                             z80 | zilog-z80
                                              → flat .bin of 1- to 4-byte Zilog
                                                Z80 opcode sequences via
                                                z80-backend; cross-platform; load
                                                into z80-simulator or an external
                                                Z80 emulator (1976 — a
                                                source/binary-compatible
                                                superset of the Intel 8080 above
                                                that powered the TRS-80, ZX
                                                Spectrum, MSX, the original Game
                                                Boy, and countless CP/M machines)
                             intel8051 | i8051 | 8051 | mcs51
                                              → flat .bin of 8-bit Intel 8051
                                                (MCS-51) opcodes via
                                                intel8051-backend; cross-platform;
                                                load into intel8051-simulator, an
                                                external MCS-51 emulator, or burn
                                                to a real 8051-family part (the
                                                most-manufactured CPU architecture
                                                in history — 20+ billion units)
                             intel8086 | i8086 | 8086
                                              → flat .bin of Intel 8086 machine code
                                                bytes via intel8086-backend; cross-
                                                platform; load into intel8086-simulator
                                                or an external 8086/8088 emulator (the
                                                direct architectural ancestor of every
                                                x86 CPU made today, 1978 — its cheaper
                                                sibling the 8088 shipped in the original
                                                IBM PC, 1981, founding the
                                                PC-compatible industry)
  -h, --help               Show this help.\
");
}

fn dispatch(
    input: &Path,
    output: &Path,
    language: Language,
    emit: EmitMode,
) -> Result<(), String> {
    // LLVM IR emission is cross-platform — short-circuit before any host
    // cfg gating below.  Reading a file and writing a `.ll` string out
    // doesn't depend on the linker or platform binary format.
    if emit == EmitMode::LlvmIr {
        return lang_aot::compile_file_to_llvm_ir(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // RV32I .bin emission is also cross-platform — just write the
    // encoded words as little-endian bytes.  No linker, no host gating.
    if emit == EmitMode::Riscv32Bin {
        return lang_aot::compile_file_to_riscv32_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // Intel 8008 .bin emission is also cross-platform — just write the
    // encoded 8-bit opcodes byte-for-byte.  No linker, no endianness
    // conversion (the 8008 has no concept of word endianness — every
    // instruction is a byte sequence).  Oct's native target.
    if emit == EmitMode::Intel8008Bin {
        return lang_aot::compile_file_to_intel8008_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // ARMv7 .bin emission is also cross-platform — flatten each
    // 32-bit A32 word to little-endian bytes (ARM's default endian
    // on every modern Linux/Android/qemu setup).  No linker, no
    // host gating (an ARM Cortex-A class CPU isn't a common dev
    // host; downstream is always arm-simulator, qemu-arm, or a
    // phone-class Linux board).
    if emit == EmitMode::Armv7Bin {
        return lang_aot::compile_file_to_armv7_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // Intel 4004 .bin emission is also cross-platform — write the
    // 1- or 2-byte opcodes byte-for-byte (no endianness conversion;
    // the 4004 has no concept of word endian, like the 8008).
    // World's first commercial microprocessor; downstream is always
    // a simulator, the intel-4004-assembler crate, or an EPROM
    // burner.
    if emit == EmitMode::Intel4004Bin {
        return lang_aot::compile_file_to_intel4004_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // GE-225 .bin emission is also cross-platform — write each
    // 20-bit instruction word as 3 bytes (big-endian, top 4 bits of
    // byte 0 zero) as iir-to-ge225 emits them.  The GE-225 (1959) is
    // the mainframe where Dartmouth BASIC was designed in 1964 —
    // primarily a BASIC fit.  Downstream is always a simulator or
    // a custom decoder.
    if emit == EmitMode::Ge225Bin {
        return lang_aot::compile_file_to_ge225_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // IBM 704 .bin emission is also cross-platform — write each
    // 36-bit instruction word as 5 bytes (low byte first, high 4
    // bits of the top byte zero) as ibm704-backend emits them.
    // The IBM 704 (1954) is the silicon Lisp was BORN on at MIT
    // in 1959; CAR/CDR are literal 704 instruction-field
    // mnemonics.  Downstream is always an emulator or replica.
    if emit == EmitMode::Ibm704Bin {
        return lang_aot::compile_file_to_ibm704_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // ARM1 .bin emission is also cross-platform — flatten each
    // 32-bit ARM1 word to little-endian bytes (ARM1's byte order —
    // see arm1_simulator::ARM1::read_word/write_word).  No linker,
    // no host gating (no modern dev host is ARM1 silicon);
    // downstream is always arm1-simulator or an external ARM1/ARMv1
    // emulator.
    if emit == EmitMode::Arm1Bin {
        return lang_aot::compile_file_to_arm1_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // MOS 6502 .bin emission is also cross-platform — the 6502 is
    // byte-oriented with no word endianness, so mos6502-backend's bytes
    // are written straight to disk (no flattening step at all, unlike
    // every 32-bit-word target above).  No linker, no host gating (no
    // modern dev host is 6502 silicon); downstream is always
    // mos6502-simulator or an external MOS 6502/NMOS emulator.
    if emit == EmitMode::Mos6502Bin {
        return lang_aot::compile_file_to_mos6502_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // Motorola 68000 .bin emission is also cross-platform — the 68000 is
    // big-endian, so m68k-backend's bytes (already emitted big-endian by
    // m68k-encoder) are written straight to disk, same no-flattening
    // simplicity as the MOS 6502 path above (for a different reason —
    // the 68000 has a word endianness, it's just already the wire
    // format). No linker, no host gating (no modern dev host is 68000
    // silicon); downstream is always m68k-simulator or an external
    // Motorola 68000 emulator.
    if emit == EmitMode::M68kBin {
        return lang_aot::compile_file_to_m68k_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // Intel 8080 .bin emission is also cross-platform — write the
    // variable-length (1/2/3-byte) opcode bytes exactly as
    // intel8080-backend emits them (no endianness conversion at
    // this layer; 16-bit address/immediate operands are already
    // little-endian within each instruction).  Third lane of the
    // 9-architecture expansion — the 8080 (1974) is the 8008's
    // direct successor and the CPU inside the Altair 8800.
    // Downstream is always a simulator (in-tree
    // `intel8080-simulator` or external) or an EPROM burner.
    if emit == EmitMode::Intel8080Bin {
        return lang_aot::compile_file_to_intel8080_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // MIPS R2000 .bin emission is also cross-platform — write each
    // 32-bit instruction word as **big-endian** bytes (MIPS R2000's
    // default byte order) as mips-r2000-backend emits them.  The
    // MIPS R2000 (1985) is the first commercially successful RISC
    // processor; downstream is always mips-r2000-simulator or an
    // external MIPS I emulator.
    if emit == EmitMode::MipsR2000Bin {
        return lang_aot::compile_file_to_mips_r2000_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // Z80 .bin emission is also cross-platform — write the 1- to
    // 4-byte opcode sequences byte-for-byte (no endianness conversion;
    // like the 8080/8008, the Z80 has no concept of word endian; 16-bit
    // immediates within an instruction are already little-endian from
    // z80-encoder).  Seventh lane of the 9-architecture expansion;
    // downstream is always z80-simulator, an external Z80 emulator, or
    // an EPROM burner.
    if emit == EmitMode::Z80Bin {
        return lang_aot::compile_file_to_z80_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // Intel 8051 .bin emission is also cross-platform — write each
    // opcode byte-for-byte (no endianness conversion; like the 8008
    // and 4004, every 8051 instruction is a byte sequence, not a
    // fixed-width word).  No linker, no host gating (no modern dev
    // host is 8051 silicon); downstream is always
    // intel8051-simulator, an external MCS-51 emulator, or a real
    // 8051-family part.
    if emit == EmitMode::Intel8051Bin {
        return lang_aot::compile_file_to_intel8051_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    // Intel 8086 .bin emission is also cross-platform — like the 6502,
    // intel8086-backend's own encoded bytes are the wire format (multi-
    // byte immediates are already little-endian within each
    // instruction; there's no fixed instruction-word width to flatten).
    // No linker, no host gating (no modern dev host is 8086/8088
    // silicon); downstream is always intel8086-simulator or an external
    // 8086/8088 emulator.  Ninth and final lane of the 9-architecture
    // expansion.
    if emit == EmitMode::Intel8086Bin {
        return lang_aot::compile_file_to_intel8086_bin(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    #[cfg(target_os = "linux")]
    { lang_aot::compile_file_to_linux_executable(input, output, language)
          .map_err(|e| format!("{e}")) }
    #[cfg(target_os = "windows")]
    { lang_aot::compile_file_to_windows_executable(input, output, language)
          .map_err(|e| format!("{e}")) }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { lang_aot::compile_file_to_macos_executable(input, output, language)
          .map_err(|e| format!("{e}")) }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "windows",
        all(target_os = "macos", target_arch = "aarch64"),
    )))]
    { let _ = (input, output, language);
      Err("lang-aot: this host platform is not supported \
           (host-targets-host only; supported hosts: Linux x86-64, \
           Windows x86-64, macOS ARM64)".into()) }
}
