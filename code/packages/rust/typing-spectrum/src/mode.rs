//! # `CompilationMode` — the five modes of the LANG compilation pipeline.
//!
//! LANG22 §"The five compilation modes" organises every execution path into
//! five mutually-understandable modes.  This module provides the enum and
//! query methods so the rest of the pipeline — `aot-no-profile`, `aot-with-pgo`,
//! `jit-core`, and the CLI driver — can reason about modes uniformly.
//!
//! ## Mode relationships
//!
//! ```text
//! Mode 1: Tree-walking interpretation
//!   ↓ profile collected (Mode 4)
//! Mode 4: JIT (tier-up from interpreter)
//!   ↓ profile written to .ldp
//! Mode 3: AOT-with-PGO   ←──────────────── Mode 2: AOT-no-profile (parallel path)
//!   ↓ compound (Mode 5)                          ↓
//! Mode 5: JIT-then-write-profile-then-AOT-PGO
//! ```
//!
//! Mode 2 (AOT-no-profile) is the *conservative* path — no speculation, no
//! deopt mechanism, ships before the JIT exists.  Mode 3 is the *aggressive*
//! path — speculates on profiled types, requires deopt machinery.

use iir_type_checker::tier::TypingTier;

// ---------------------------------------------------------------------------
// CompilationMode
// ---------------------------------------------------------------------------

/// The five compilation modes of the LANG pipeline.
///
/// Each mode has a different cost/benefit profile depending on the
/// [`TypingTier`] of the program being compiled.  Use
/// [`CompilationMode::recommended_for`] to get a data-driven suggestion.
///
/// # Example
///
/// ```
/// use typing_spectrum::mode::CompilationMode;
/// use iir_type_checker::tier::TypingTier;
///
/// let mode = CompilationMode::recommended_for(&TypingTier::Untyped);
/// // Untyped programs warm up best via JIT; AOT-with-PGO on the next build.
/// assert_eq!(mode, CompilationMode::Jit);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CompilationMode {
    /// **Mode 1** — tree-walking interpretation of `IIRModule`.
    ///
    /// - Input: `IIRModule`
    /// - Output: live `Value` (per-language)
    /// - Speed: baseline (1×)
    /// - Use: development, REPL, cold paths after AOT deopts
    /// - Deopt mechanism: none (this *is* the deopt target)
    TreeWalking,

    /// **Mode 2** — ahead-of-time compilation without a profile.
    ///
    /// Typed instructions → native machine code.
    /// Untyped instructions → calls into `liblang-runtime.a`.
    ///
    /// - Input: `IIRModule` (with or without type hints)
    /// - Output: native binary statically linked against the runtime
    /// - Speed: ~3–10× tree-walking for dynamic languages,
    ///          ~50–100× for fully-typed languages
    /// - Use: cold-start–sensitive CLIs, cross-compilation, programs without warmup budget
    /// - Deopt mechanism: **not required** — no speculation emitted
    AotNoProfile,

    /// **Mode 3** — ahead-of-time compilation with a PGO profile artefact.
    ///
    /// The `.ldp` profile promotes `type_hint = "any"` to observed types
    /// wherever confidence exceeds the threshold (default 99%).  The codegen
    /// emits speculative native code plus deopt anchors on the hot path.
    ///
    /// - Input: `IIRModule` + `.ldp` profile artefact
    /// - Output: native binary with embedded `.deopt` metadata
    /// - Speed: ~30–50× tree-walking for type-stable programs
    /// - Use: production deployment after profile collection
    /// - Deopt mechanism: **required** (`BoxedReprToken` + frame descriptors)
    AotWithPgo,

    /// **Mode 4** — JIT tier-up from the interpreter.
    ///
    /// The interpreter profiles instruction types as the program runs.  When
    /// a function's call count crosses the tier threshold (see
    /// [`crate::threshold::JitPromotionThreshold`]) the JIT compiles it using
    /// the live observations and patches the function table in-process.
    ///
    /// - Input: `IIRModule` + live observations from the interpreter
    /// - Output: native code in this process's memory
    /// - Speed: ~30–50× tree-walking once warm
    /// - Use: long-running programs, servers, REPLs
    /// - Deopt mechanism: **required** (same as Mode 3)
    Jit,

    /// **Mode 5** — compound JIT → profile → AOT-with-PGO.
    ///
    /// Run Mode 4 in development; on shutdown the JIT writes a `.ldp`
    /// artefact; the release build uses Mode 3 with that artefact.
    /// GraalVM Native Image's primary mode.
    ///
    /// - Input: `IIRModule` (run); `IIRModule + .ldp` (compile)
    /// - Speed: as good as Mode 3 on the profiled workload
    /// - Use: release builds of servers, libraries, and CLIs where
    ///        cold-start *and* peak throughput matter
    JitThenAotWithPgo,
}

impl CompilationMode {
    /// Recommend the best **initial** compilation mode for a module at
    /// the given typing tier.
    ///
    /// The recommendation is based purely on the typing tier.  Deployment
    /// context (server vs CLI, warmup budget, etc.) may override it; this
    /// is the reasonable default.
    ///
    /// | Tier | Recommended mode | Rationale |
    /// |------|-----------------|-----------|
    /// | `FullyTyped` | [`AotNoProfile`](CompilationMode::AotNoProfile) | All types known statically; AOT produces near-C code without profiling overhead. |
    /// | `Partial` ≥ 70% | [`AotNoProfile`](CompilationMode::AotNoProfile) | Enough static types to win from AOT immediately; profile fills the rest. |
    /// | `Partial` < 70% | [`Jit`](CompilationMode::Jit) | Too many untyped paths; JIT learns them and writes a `.ldp` for later AOT-PGO. |
    /// | `Untyped` | [`Jit`](CompilationMode::Jit) | No static types at all; JIT is the only source of specialisation data. |
    ///
    /// ```
    /// use typing_spectrum::mode::CompilationMode;
    /// use iir_type_checker::tier::TypingTier;
    ///
    /// assert_eq!(CompilationMode::recommended_for(&TypingTier::FullyTyped),  CompilationMode::AotNoProfile);
    /// assert_eq!(CompilationMode::recommended_for(&TypingTier::Partial(0.8)), CompilationMode::AotNoProfile);
    /// assert_eq!(CompilationMode::recommended_for(&TypingTier::Partial(0.5)), CompilationMode::Jit);
    /// assert_eq!(CompilationMode::recommended_for(&TypingTier::Untyped),     CompilationMode::Jit);
    /// ```
    pub fn recommended_for(tier: &TypingTier) -> Self {
        match tier {
            TypingTier::FullyTyped => CompilationMode::AotNoProfile,
            TypingTier::Partial(frac) if *frac >= 0.70 => CompilationMode::AotNoProfile,
            TypingTier::Partial(_) => CompilationMode::Jit,
            TypingTier::Untyped => CompilationMode::Jit,
        }
    }

    /// Human-readable one-line description of this mode.
    pub fn description(&self) -> &'static str {
        match self {
            CompilationMode::TreeWalking =>
                "Mode 1: Tree-walking interpretation (baseline)",
            CompilationMode::AotNoProfile =>
                "Mode 2: AOT-no-profile (typed → native, untyped → runtime calls)",
            CompilationMode::AotWithPgo =>
                "Mode 3: AOT-with-PGO (speculation + deopt anchors from .ldp profile)",
            CompilationMode::Jit =>
                "Mode 4: JIT tier-up (live type observations → native code in-process)",
            CompilationMode::JitThenAotWithPgo =>
                "Mode 5: JIT-then-AOT-PGO (run JIT, save .ldp, AOT-compile for release)",
        }
    }

    /// Whether this mode requires the deopt mechanism
    /// (`BoxedReprToken` + frame descriptors + `lang_deopt` ABI entry).
    ///
    /// [`AotNoProfile`](CompilationMode::AotNoProfile) and
    /// [`TreeWalking`](CompilationMode::TreeWalking) are the only modes
    /// that do **not** speculate, so they need no deopt.
    ///
    /// ```
    /// use typing_spectrum::mode::CompilationMode;
    ///
    /// assert!(!CompilationMode::TreeWalking.requires_deopt());
    /// assert!(!CompilationMode::AotNoProfile.requires_deopt());
    /// assert!(CompilationMode::AotWithPgo.requires_deopt());
    /// assert!(CompilationMode::Jit.requires_deopt());
    /// assert!(CompilationMode::JitThenAotWithPgo.requires_deopt());
    /// ```
    pub fn requires_deopt(&self) -> bool {
        matches!(
            self,
            CompilationMode::AotWithPgo
                | CompilationMode::Jit
                | CompilationMode::JitThenAotWithPgo
        )
    }

    /// Whether this mode needs a `.ldp` profile artefact as **input**.
    ///
    /// Modes 3 and 5 read a `.ldp` file at compile time to promote
    /// `type_hint = "any"` fields.  Modes 1, 2, and 4 do not.
    ///
    /// ```
    /// use typing_spectrum::mode::CompilationMode;
    ///
    /// assert!(!CompilationMode::AotNoProfile.requires_profile_input());
    /// assert!(CompilationMode::AotWithPgo.requires_profile_input());
    /// assert!(CompilationMode::JitThenAotWithPgo.requires_profile_input());
    /// ```
    pub fn requires_profile_input(&self) -> bool {
        matches!(
            self,
            CompilationMode::AotWithPgo | CompilationMode::JitThenAotWithPgo
        )
    }

    /// Whether this mode **writes** a `.ldp` artefact (JIT observations
    /// serialised to disk on shutdown / flush).
    ///
    /// ```
    /// use typing_spectrum::mode::CompilationMode;
    ///
    /// assert!(!CompilationMode::AotNoProfile.writes_profile());
    /// assert!(CompilationMode::Jit.writes_profile());
    /// assert!(CompilationMode::JitThenAotWithPgo.writes_profile());
    /// ```
    pub fn writes_profile(&self) -> bool {
        matches!(
            self,
            CompilationMode::Jit | CompilationMode::JitThenAotWithPgo
        )
    }

    /// Approximate speedup range over tree-walking interpretation
    /// (returned as `(low_multiplier, high_multiplier)`) at the given
    /// typing tier.  These are the spec's ballpark figures, not guarantees.
    ///
    /// ```
    /// use typing_spectrum::mode::CompilationMode;
    /// use iir_type_checker::tier::TypingTier;
    ///
    /// let (lo, hi) = CompilationMode::AotNoProfile
    ///     .expected_speedup_over_interp(&TypingTier::Untyped);
    /// assert!(lo <= hi);
    /// ```
    pub fn expected_speedup_over_interp(&self, tier: &TypingTier) -> (u32, u32) {
        match (self, tier) {
            (CompilationMode::TreeWalking, _) => (1, 1),

            (CompilationMode::AotNoProfile, TypingTier::FullyTyped) => (50, 100),
            (CompilationMode::AotNoProfile, TypingTier::Partial(_)) => (5, 30),
            (CompilationMode::AotNoProfile, TypingTier::Untyped)    => (3, 10),

            (CompilationMode::AotWithPgo,  TypingTier::FullyTyped) => (50, 100),
            (CompilationMode::AotWithPgo,  _)                      => (30, 50),

            (CompilationMode::Jit, TypingTier::FullyTyped) => (10, 30), // modest: already typed
            (CompilationMode::Jit, _)                      => (30, 50),

            (CompilationMode::JitThenAotWithPgo, TypingTier::FullyTyped) => (50, 100),
            (CompilationMode::JitThenAotWithPgo, _)                      => (30, 50),
        }
    }
}

impl std::fmt::Display for CompilationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let short = match self {
            CompilationMode::TreeWalking       => "tree-walking",
            CompilationMode::AotNoProfile      => "aot-no-profile",
            CompilationMode::AotWithPgo        => "aot-with-pgo",
            CompilationMode::Jit               => "jit",
            CompilationMode::JitThenAotWithPgo => "jit-then-aot-pgo",
        };
        f.write_str(short)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fully_typed_recommends_aot_no_profile() {
        assert_eq!(
            CompilationMode::recommended_for(&TypingTier::FullyTyped),
            CompilationMode::AotNoProfile
        );
    }

    #[test]
    fn partial_high_recommends_aot_no_profile() {
        assert_eq!(
            CompilationMode::recommended_for(&TypingTier::Partial(0.70)),
            CompilationMode::AotNoProfile
        );
        assert_eq!(
            CompilationMode::recommended_for(&TypingTier::Partial(0.99)),
            CompilationMode::AotNoProfile
        );
    }

    #[test]
    fn partial_low_recommends_jit() {
        assert_eq!(
            CompilationMode::recommended_for(&TypingTier::Partial(0.69)),
            CompilationMode::Jit
        );
        assert_eq!(
            CompilationMode::recommended_for(&TypingTier::Partial(0.10)),
            CompilationMode::Jit
        );
    }

    #[test]
    fn untyped_recommends_jit() {
        assert_eq!(
            CompilationMode::recommended_for(&TypingTier::Untyped),
            CompilationMode::Jit
        );
    }

    #[test]
    fn deopt_requirement_correct() {
        assert!(!CompilationMode::TreeWalking.requires_deopt());
        assert!(!CompilationMode::AotNoProfile.requires_deopt());
        assert!(CompilationMode::AotWithPgo.requires_deopt());
        assert!(CompilationMode::Jit.requires_deopt());
        assert!(CompilationMode::JitThenAotWithPgo.requires_deopt());
    }

    #[test]
    fn profile_input_correct() {
        assert!(!CompilationMode::TreeWalking.requires_profile_input());
        assert!(!CompilationMode::AotNoProfile.requires_profile_input());
        assert!(!CompilationMode::Jit.requires_profile_input());
        assert!(CompilationMode::AotWithPgo.requires_profile_input());
        assert!(CompilationMode::JitThenAotWithPgo.requires_profile_input());
    }

    #[test]
    fn profile_write_correct() {
        assert!(!CompilationMode::TreeWalking.writes_profile());
        assert!(!CompilationMode::AotNoProfile.writes_profile());
        assert!(!CompilationMode::AotWithPgo.writes_profile());
        assert!(CompilationMode::Jit.writes_profile());
        assert!(CompilationMode::JitThenAotWithPgo.writes_profile());
    }

    #[test]
    fn speedup_range_is_sensible() {
        for tier in [
            TypingTier::FullyTyped,
            TypingTier::Partial(0.5),
            TypingTier::Untyped,
        ] {
            for mode in [
                CompilationMode::TreeWalking,
                CompilationMode::AotNoProfile,
                CompilationMode::AotWithPgo,
                CompilationMode::Jit,
                CompilationMode::JitThenAotWithPgo,
            ] {
                let (lo, hi) = mode.expected_speedup_over_interp(&tier);
                assert!(lo <= hi, "mode={mode}, tier={tier:?}: lo={lo} > hi={hi}");
                assert!(lo >= 1, "speedup below 1× for mode={mode}");
            }
        }
    }

    #[test]
    fn display_matches_short_name() {
        assert_eq!(CompilationMode::AotNoProfile.to_string(), "aot-no-profile");
        assert_eq!(CompilationMode::Jit.to_string(), "jit");
    }
}
