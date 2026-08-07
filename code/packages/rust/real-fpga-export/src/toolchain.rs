//! iCE40 toolchain driver: yosys → nextpnr-ice40 → icepack → iceprog.
//!
//! ## How the pipeline works
//!
//! ```text
//! Step 1  yosys  -p "synth_ice40 -top <top> -json <out>.json" <top>.v
//!         Synthesis: technology-maps RTL Verilog to iCE40 primitives.
//!
//! Step 2  nextpnr-ice40 --<part> --package <pkg>
//!                        --json <in>.json --pcf <constraints>.pcf
//!                        --asc <out>.asc
//!         Place and route: assigns each LUT/FF to a tile and routes wires.
//!
//! Step 3  icepack <in>.asc <out>.bin
//!         Packs the ASCII bitstream into the binary format for iceprog.
//!
//! Step 4  iceprog <out>.bin   (optional — requires physical board)
//!         Flashes the bitstream to the FPGA via USB.
//! ```
//!
//! ## skip_missing mode
//!
//! When `skip_missing = true`, `to_ice40` stops after Verilog emission if
//! `yosys` is not found on PATH (or after synthesis if `nextpnr-ice40` is
//! absent, etc.).  This lets CI verify the Verilog writer without needing
//! the full open-tool stack installed.

use std::path::{Path, PathBuf};
use std::process::Command;

use hdl_ir::hir::Hir;

use crate::verilog_writer::write_verilog;

// ---------------------------------------------------------------------------
// ToolchainOptions
// ---------------------------------------------------------------------------

/// Names / locations of the FPGA tool executables.
///
/// Override any field to point at a non-default binary (e.g. `/opt/oss-cad-suite/bin/yosys`).
#[derive(Debug, Clone)]
pub struct ToolchainOptions {
    pub yosys:           String,
    pub nextpnr_ice40:   String,
    pub icepack:         String,
    pub iceprog:         String,
    /// Maximum wall-clock seconds to allow any single tool invocation.
    pub timeout_s:       u64,
}

impl Default for ToolchainOptions {
    fn default() -> Self {
        Self {
            yosys:          "yosys".to_string(),
            nextpnr_ice40:  "nextpnr-ice40".to_string(),
            icepack:        "icepack".to_string(),
            iceprog:        "iceprog".to_string(),
            timeout_s:      600,
        }
    }
}

// ---------------------------------------------------------------------------
// ToolchainResult
// ---------------------------------------------------------------------------

/// Artifacts produced by the toolchain, plus accumulated log output.
#[derive(Debug, Clone, Default)]
pub struct ToolchainResult {
    /// Always present: the emitted Verilog file.
    pub verilog_path: PathBuf,
    /// Present after a successful `yosys` run.
    pub json_path:    Option<PathBuf>,
    /// Present after a successful `nextpnr-ice40` run.
    pub asc_path:     Option<PathBuf>,
    /// Present after a successful `icepack` run.
    pub bin_path:     Option<PathBuf>,
    /// Captured stdout + stderr from all tool invocations, in order.
    pub log_lines:    Vec<String>,
}

// ---------------------------------------------------------------------------
// to_ice40 — the main driver
// ---------------------------------------------------------------------------

/// Run the full iCE40 toolchain.
///
/// # Arguments
///
/// - `hir` — the design to synthesise
/// - `top` — name of the top-level module
/// - `pcf` — optional pin-constraint file; without it, nextpnr is skipped
/// - `out_dir` — directory to write intermediate files into (created if absent)
/// - `part` — iCE40 part code: `"hx1k"`, `"hx8k"`, `"up5k"`, …
/// - `package` — package code: `"tq144"`, `"sg48"`, …
/// - `opts` — tool executable names + timeout (defaults via `ToolchainOptions::default()`)
/// - `skip_missing` — if `true`, stop gracefully when a tool isn't on PATH instead of `Err`
///
/// # Errors
///
/// Returns `Err(String)` when a tool exits with a non-zero status or when a
/// required tool is missing and `skip_missing = false`.
// Each argument is a distinct toolchain input (design, top module, constraint
// file, output paths, flags); a config struct would not clarify this call site.
#[allow(clippy::too_many_arguments)]
pub fn to_ice40(
    hir:          &Hir,
    top:          &str,
    pcf:          Option<&Path>,
    out_dir:      &Path,
    part:         &str,
    package:      &str,
    opts:         Option<&ToolchainOptions>,
    skip_missing: bool,
) -> Result<ToolchainResult, String> {
    let default_opts = ToolchainOptions::default();
    let opts = opts.unwrap_or(&default_opts);

    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("cannot create output directory: {e}"))?;

    // Step 1 — emit Verilog
    let v_path = out_dir.join(format!("{top}.v"));
    write_verilog(hir, &v_path)
        .map_err(|e| format!("write_verilog failed: {e}"))?;

    let mut result = ToolchainResult {
        verilog_path: v_path.clone(),
        ..Default::default()
    };

    // Step 2 — yosys
    if skip_missing && which(&opts.yosys).is_none() {
        result.log_lines.push(format!("{} not found; skipping toolchain", opts.yosys));
        return Ok(result);
    }
    let json_path = out_dir.join(format!("{top}.json"));
    run_tool(
        &[
            opts.yosys.as_str(), "-q",
            "-p", &format!("synth_ice40 -top {top} -json {}", json_path.display()),
            v_path.to_str().unwrap_or(""),
        ],
        &mut result,
    )?;
    result.json_path = Some(json_path.clone());

    // Step 3 — nextpnr-ice40 (only if a PCF is provided)
    let Some(pcf) = pcf else {
        result.log_lines.push("no PCF provided; skipping place-route".to_string());
        return Ok(result);
    };
    let asc_path = out_dir.join(format!("{top}.asc"));
    run_tool(
        &[
            opts.nextpnr_ice40.as_str(),
            &format!("--{part}"),
            "--package", package,
            "--json", json_path.to_str().unwrap_or(""),
            "--pcf",  pcf.to_str().unwrap_or(""),
            "--asc",  asc_path.to_str().unwrap_or(""),
        ],
        &mut result,
    )?;
    result.asc_path = Some(asc_path.clone());

    // Step 4 — icepack
    let bin_path = out_dir.join(format!("{top}.bin"));
    run_tool(
        &[
            opts.icepack.as_str(),
            asc_path.to_str().unwrap_or(""),
            bin_path.to_str().unwrap_or(""),
        ],
        &mut result,
    )?;
    result.bin_path = Some(bin_path);

    Ok(result)
}

// ---------------------------------------------------------------------------
// program_ice40
// ---------------------------------------------------------------------------

/// Flash a bitstream binary to a real iCE40 board via `iceprog`.
///
/// # Errors
///
/// Returns `Err(String)` when `iceprog` is not on PATH or exits with a
/// non-zero status.
pub fn program_ice40(bin_path: &Path, opts: Option<&ToolchainOptions>) -> Result<(), String> {
    let default_opts = ToolchainOptions::default();
    let opts = opts.unwrap_or(&default_opts);

    if which(&opts.iceprog).is_none() {
        return Err(format!("{} not on PATH", opts.iceprog));
    }

    let mut dummy = ToolchainResult::default();
    run_tool(
        &[opts.iceprog.as_str(), bin_path.to_str().unwrap_or("")],
        &mut dummy,
    )
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Check if `name` resolves to an executable on PATH.
///
/// Uses `where` on Windows and `which` (PATH search) semantics on Unix.
/// We implement a simple PATH scan instead of shelling out to avoid
/// platform-specific tool dependencies.
fn which(name: &str) -> Option<PathBuf> {
    // If it's an absolute or relative path that exists, take it.
    let p = PathBuf::from(name);
    if p.is_absolute() || name.contains('/') || name.contains('\\') {
        if p.exists() { return Some(p); }
        return None;
    }

    // Search PATH.
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.exists() {
            return Some(candidate);
        }
        // On Windows, also try with .exe suffix.
        #[cfg(target_os = "windows")]
        {
            let exe = dir.join(format!("{name}.exe"));
            if exe.exists() { return Some(exe); }
        }
    }
    None
}

/// Run a tool, capturing stdout + stderr into `result.log_lines`.
/// Returns `Err` if the tool exits with non-zero status or isn't found.
fn run_tool(args: &[&str], result: &mut ToolchainResult) -> Result<(), String> {
    let (prog, rest) = match args {
        [prog, rest @ ..] => (*prog, rest),
        [] => return Err("run_tool called with empty args".to_string()),
    };

    if which(prog).is_none() {
        return Err(format!("{prog:?} not on PATH"));
    }

    let output = Command::new(prog)
        .args(rest)
        .output()
        .map_err(|e| format!("failed to run {prog}: {e}"))?;

    if !output.stdout.is_empty() {
        result.log_lines.push(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    if !output.stderr.is_empty() {
        result.log_lines.push(String::from_utf8_lossy(&output.stderr).into_owned());
    }

    if output.status.success() {
        Ok(())
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "{prog} failed with exit code {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            output.status.code()
        ))
    }
}
