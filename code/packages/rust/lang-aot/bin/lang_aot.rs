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
        other => Err(format!(
            "unknown --emit value {other:?}; expected one of: \
             native | llvm-ir | riscv32 | intel8008 | armv7 | intel4004 | ge225 | ibm704 | arm1 | mos6502"
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
