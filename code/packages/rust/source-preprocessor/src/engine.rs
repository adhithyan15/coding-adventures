//! # The engine — one ordered pass over the token stream.
//!
//! ## Why one pass, and not include-then-conditionals
//!
//! It is tempting to resolve includes as a text transform and handle
//! conditionals later over tokens. `lexer-parser-hooks.md` originally
//! suggested exactly that. It cannot work, because the two are mutually
//! dependent:
//!
//! ```text
//!   @if USE_FAST          ← decides whether the include happens at all …
//!       @include "fast"   ← … and "fast" defines names that …
//!   @end
//!   @if FAST_VERSION > 2  ← … later conditionals test.
//! ```
//!
//! A text-level include pass would pull in `fast` unconditionally, and a
//! later token-level conditional pass could not have seen what `fast` defined
//! in time to affect the decision. So inclusion and conditional selection
//! interleave in a single traversal — this one.
//!
//! ## Why an explicit stack instead of recursion
//!
//! An included file contains tokens that may themselves include. The natural
//! implementation recurses. We do not, and this is normative rather than
//! stylistic: in Rust a stack overflow is an **abort**, not a catchable panic,
//! and not containable by `catch_unwind` in an embedding host. A recursive
//! engine would convert [`Bounds::include_depth`] from a diagnostic into a
//! process kill — defeating the no-panic contract the bounds exist to uphold.
//!
//! So inclusion is a [`Vec`] of [`Frame`]s, and depth is a number we compare
//! rather than a thing the machine does to us.
//!
//! ## Cycle detection is stack-based, and that is not enough on its own
//!
//! A file may not appear twice on the *active* include stack. It may appear
//! many times overall — C requires that, and so does any language with a
//! shared header. Which leaves the fan-out case:
//!
//! ```text
//!   a  includes  b0..b9
//!   b* includes  c0..c9      ← acyclic, depth 8, 10^8 inclusions
//!   …
//! ```
//!
//! never repeating on the stack and never exceeding the depth bound. That is
//! what [`Bounds::total_inclusions`] is for, and why it is a cumulative total
//! rather than a per-level check.

use crate::bounds::{Bounds, Spend};
use crate::diag::PpError;
use crate::dialect::{Dialect, Directive};
use crate::fs::SourceFs;
use crate::source_map::{FileId, Locus, Position, SourceMap};
use lexer::token::Token;

/// One file being traversed: its tokens and how far through them we are.
struct Frame {
    tokens: Vec<Token>,
    cursor: usize,
    file: FileId,
}

/// One open conditional group.
struct Cond {
    /// Whether any branch of this group has already been taken, so a later
    /// `@else` knows to stay dark.
    branch_taken: bool,
    /// Whether this group's current branch is emitting.
    emitting: bool,
    /// Whether the enclosing context was emitting. A nested group inside a
    /// skipped group stays skipped no matter what its own condition says —
    /// indeed its condition is never evaluated at all.
    parent_emitting: bool,
    /// Where the group opened, so an unterminated group can say where.
    opened_at: Position,
}

/// The result of preprocessing one translation unit.
pub struct Preprocessed {
    pub tokens: Vec<Token>,
    pub map: SourceMap,
}

/// Preprocess one translation unit.
///
/// `tokens` is the already-lexed primary file. `file` names it. Everything
/// else the engine needs, it gets through `dialect` and `fs`.
pub fn preprocess(
    tokens: Vec<Token>,
    file: FileId,
    dialect: &dyn Dialect,
    fs: &mut dyn SourceFs,
    bounds: Bounds,
) -> Result<Preprocessed, PpError> {
    let mut out = Vec::new();
    let mut map = SourceMap::new();
    let mut spend = Spend::default();

    let mut frames: Vec<Frame> = vec![Frame { tokens, cursor: 0, file }];
    let mut open_files: Vec<FileId> = vec![file];
    let mut conds: Vec<Cond> = Vec::new();
    // How many conditional groups were open when each frame was pushed, so an
    // unterminated group inside an included file is caught at that file's end
    // rather than leaking into its includer.
    let mut cond_floor: Vec<usize> = vec![0];

    while let Some(frame) = frames.last_mut() {
        if frame.cursor >= frame.tokens.len() {
            let floor = cond_floor.pop().unwrap_or(0);
            if conds.len() > floor {
                let at = conds[floor].opened_at;
                return Err(PpError::new(
                    "conditional group is never closed before the end of the file",
                )
                .at(at));
            }
            frames.pop();
            open_files.pop();
            continue;
        }

        // Take the run of tokens sharing this token's line. Directives are
        // line-oriented in every dialect we serve, so a "logical line" is the
        // unit of classification; ordinary code simply passes through run by
        // run.
        let start = frame.cursor;
        let line_no = frame.tokens[start].line;
        let mut end = start;
        while end < frame.tokens.len() && frame.tokens[end].line == line_no {
            end += 1;
        }
        frame.cursor = end;
        let run: Vec<Token> = frame.tokens[start..end].to_vec();
        let current_file = frame.file;

        spend.fuel_used += run.len() as u64;
        if spend.fuel_used > bounds.fuel {
            return Err(PpError::new(format!(
                "exhausted the {}-step preprocessing budget",
                bounds.fuel
            )));
        }

        let emitting = conds.last().is_none_or(|c| c.emitting && c.parent_emitting);

        match dialect.classify(&run) {
            None => {
                if emitting {
                    emit(&run, current_file, &mut out, &mut map, &mut spend, &bounds)?;
                }
            }
            Some(Err(e)) => return Err(e),
            Some(Ok(directive)) => {
                apply_directive(
                    directive,
                    &run,
                    current_file,
                    emitting,
                    dialect,
                    fs,
                    &bounds,
                    &mut spend,
                    &mut conds,
                    &mut frames,
                    &mut open_files,
                    &mut cond_floor,
                )?;
            }
        }
    }

    if let Some(c) = conds.first() {
        return Err(
            PpError::new("conditional group is never closed").at(c.opened_at)
        );
    }

    map.check_len(out.len()).map_err(|e| PpError::new(e.to_string()))?;
    Ok(Preprocessed { tokens: out, map })
}

fn emit(
    run: &[Token],
    file: FileId,
    out: &mut Vec<Token>,
    map: &mut SourceMap,
    spend: &mut Spend,
    bounds: &Bounds,
) -> Result<(), PpError> {
    for t in run {
        spend.tokens_produced += 1;
        if spend.tokens_produced > bounds.tokens_produced {
            return Err(PpError::new(format!(
                "produced more than {} tokens",
                bounds.tokens_produced
            )));
        }
        if t.value.len() as u64 > bounds.token_spelling_bytes {
            return Err(PpError::new(format!(
                "a token's spelling exceeds {} bytes",
                bounds.token_spelling_bytes
            )));
        }
        map.push(Locus {
            position: Position { file, line: t.line as u32, column: t.column as u32 },
            expansion: None,
        });
        out.push(t.clone());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_directive(
    directive: Directive,
    run: &[Token],
    current_file: FileId,
    emitting: bool,
    dialect: &dyn Dialect,
    fs: &mut dyn SourceFs,
    bounds: &Bounds,
    spend: &mut Spend,
    conds: &mut Vec<Cond>,
    frames: &mut Vec<Frame>,
    open_files: &mut Vec<FileId>,
    cond_floor: &mut Vec<usize>,
) -> Result<(), PpError> {
    let here = Position {
        file: current_file,
        line: run.first().map_or(1, |t| t.line as u32),
        column: run.first().map_or(1, |t| t.column as u32),
    };

    match directive {
        // --- conditionals --------------------------------------------------
        //
        // Note what happens in a SKIPPED group: the condition is not
        // evaluated, the include is not resolved, nothing is expanded. Only
        // the nesting is tracked. Otherwise every expansion bomb would be
        // reachable from inside `@if 0` -- which is precisely where hostile
        // input would put it, and where a human reviewer stops reading.
        Directive::If(condition) => {
            if conds.len() as u32 >= bounds.conditional_depth {
                return Err(PpError::new(format!(
                    "conditional nesting deeper than {}",
                    bounds.conditional_depth
                ))
                .at(here));
            }
            let parent_emitting = emitting;
            let value = if parent_emitting {
                check_group_depth(&condition, bounds.condition_depth, here)?;
                dialect.eval_condition(&condition)?
            } else {
                false
            };
            conds.push(Cond {
                branch_taken: value,
                emitting: value,
                parent_emitting,
                opened_at: here,
            });
        }
        Directive::Else => {
            let floor = *cond_floor.last().unwrap_or(&0);
            if conds.len() <= floor {
                return Err(PpError::new("`else` without an open conditional").at(here));
            }
            let c = conds.last_mut().expect("checked non-empty");
            c.emitting = !c.branch_taken;
            c.branch_taken = true;
        }
        Directive::EndIf => {
            let floor = *cond_floor.last().unwrap_or(&0);
            if conds.len() <= floor {
                return Err(PpError::new("conditional closed without being opened").at(here));
            }
            conds.pop();
        }

        // --- inclusion -----------------------------------------------------
        Directive::Include(request) => {
            if !emitting {
                return Ok(());
            }
            if frames.len() as u32 >= bounds.include_depth {
                return Err(PpError::new(format!(
                    "includes nested deeper than {}",
                    bounds.include_depth
                ))
                .at(here));
            }
            spend.inclusions += 1;
            if spend.inclusions > bounds.total_inclusions {
                return Err(PpError::new(format!(
                    "more than {} total inclusions — a fan-out include graph can exceed this \
                     while staying acyclic and shallow",
                    bounds.total_inclusions
                ))
                .at(here));
            }
            spend.fuel_used += 1;

            let mut request = request;
            request.from = Some(current_file);
            let id = fs.resolve(&request).map_err(|e| e.at(here))?;

            if open_files.contains(&id) {
                return Err(PpError::new(format!(
                    "include cycle: {} is already open",
                    fs.name_of(id)
                ))
                .at(here));
            }

            let text = fs.read(id).map_err(|e| e.at(here))?;
            spend.source_bytes += text.len() as u64;
            if spend.source_bytes > bounds.total_source_bytes {
                return Err(PpError::new(format!(
                    "read more than {} total source bytes",
                    bounds.total_source_bytes
                ))
                .at(here));
            }

            let tokens = dialect.lex(&text, id).map_err(|e| e.at(here))?;
            cond_floor.push(conds.len());
            open_files.push(id);
            frames.push(Frame { tokens, cursor: 0, file: id });
        }

        // --- not yet live --------------------------------------------------
        Directive::Define { .. } => {
            // Slice 1 has no macro table. A dialect that emits this before
            // macros land should hear about it rather than have it silently
            // ignored -- a `@define` that does nothing would be far more
            // confusing than one that refuses.
            return Err(PpError::new(
                "macro definitions are not supported yet (PREP01 slice 2)",
            )
            .at(here));
        }
        Directive::Ignore => {}
    }
    Ok(())
}

/// Conservative structural depth check over a controlling expression.
///
/// Run by the engine *before* the slice reaches [`Dialect::eval_condition`],
/// rather than left to each dialect. If every dialect had to remember to do
/// this, one authored later would forget, and a million nested parentheses
/// would abort the process inside its expression parser — the exact failure
/// the no-native-recursion rule exists to prevent.
///
/// It matches bracket *spellings* rather than token kinds, because token kinds
/// are per-language and this check must work for a dialect the engine has
/// never seen.
fn check_group_depth(tokens: &[Token], limit: u32, at: Position) -> Result<(), PpError> {
    let mut depth: u32 = 0;
    for t in tokens {
        match t.value.as_str() {
            "(" | "[" | "{" => {
                depth += 1;
                if depth > limit {
                    return Err(PpError::new(format!(
                        "controlling expression nested deeper than {limit}"
                    ))
                    .at(at));
                }
            }
            ")" | "]" | "}" => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}
