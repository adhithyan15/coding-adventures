export const meta = {
  name: 'adj52-rootcause',
  description: 'Root-cause each wrong (and a couple correct-control) ADJ52 diagnostic cases: read its rulebook+program, re-run the deterministic engine, and map the failure to a calibration lever (H1 correlation / H2 open-question discounting / H3 residual mass) with concrete evidence. Read-only analysis; no fixes applied.',
  phases: [{ title: 'RootCause' }],
}

const ROOT = '/Users/adhithya/Downloads/coding-adventures/code/specs/data/adj52'
const MANIFEST = `${ROOT}/Cargo.toml`
// Target list hardcoded (small, stable). Full held-aside context per case is on
// disk at cases/<id>/_rootcause-context.json (written by the parent before launch).
const targets = [
  { id: 'case-5', expected_label: 'wrong', framework_correct: 'incorrect' },
  { id: 'case-6', expected_label: 'wrong', framework_correct: 'partial' },
  { id: 'case-7', expected_label: 'wrong', framework_correct: 'partial' },
  { id: 'case-15', expected_label: 'wrong', framework_correct: 'partial' },
  { id: 'case-18', expected_label: 'wrong', framework_correct: 'incorrect' },
  { id: 'case-21', expected_label: 'wrong', framework_correct: 'partial' },
  { id: 'case-29', expected_label: 'wrong', framework_correct: 'partial' },
  { id: 'case-9', expected_label: 'control_correct', framework_correct: 'correct' },
  { id: 'case-2', expected_label: 'control_correct', framework_correct: 'correct' },
]
log(`root-causing ${targets.length} cases`)

const SCHEMA = {
  type: 'object',
  required: ['id', 'top1_correct', 'failure_class', 'primary_lever', 'posterior_top', 'evidence', 'what_fix_needed'],
  properties: {
    id: { type: 'string' },
    top1_correct: { type: 'boolean', description: 'did the engine top-1 diagnosis match the ground-truth diagnosis' },
    correct_term_if_wrong: { type: 'string', description: 'the adj-lang diagnosis() term that SHOULD have won (empty if top1_correct)' },
    failure_class: { type: 'string', enum: ['wrong_top1', 'saturated_overconfident', 'incoherent_differential', 'inert_uncertainty', 'missing_residual_mass', 'none'] },
    primary_lever: { type: 'string', enum: ['H1_correlation', 'H2_open_question', 'H3_residual_mass', 'other', 'none'], description: 'the single lever that would most help this case' },
    secondary_lever: { type: 'string', enum: ['H1_correlation', 'H2_open_question', 'H3_residual_mass', 'other', 'none'] },
    mechanism_directives_used: { type: 'number', description: 'count of "% mechanism" lines in the rulebook' },
    independent_same_sign_contributes_to_top: { type: 'number', description: 'how many SEPARATE positive contributes fed the winning conclusion (the Naive-Bayes double-count indicator)' },
    open_uncertainty_present: { type: 'boolean', description: 'is there an uncertain{} marker bearing on the conclusion' },
    recommends_confirmatory_test: { type: 'boolean', description: 'does the answer recommend a confirmatory test while asserting high confidence' },
    posterior_top: { type: 'number' },
    evidence: { type: 'string', description: 'specific quotes from the rulebook / engine output that demonstrate the failure mechanism' },
    what_fix_needed: { type: 'string', description: 'concise: which lever, applied how, and why it would fix THIS case without changing the top-1 ranking on the correct cases' },
  },
}

const prompt = (t) => `You are root-causing ONE case from a Bayesian diagnostic engine to decide which CALIBRATION FIX it needs. Be precise and evidence-based; do NOT propose vague fixes.

CASE: ${t.id}  (this case was judged: framework_correct=${t.framework_correct}, expected_label=${t.expected_label})

STEPS (use your tools):
1. Read the held-aside context (ground truth, engine result, blind-judge rationale):
   ${ROOT}/cases/${t.id}/_rootcause-context.json
2. Read the rulebook:  ${ROOT}/cases/${t.id}/rulebook.adj
3. Read the program:   ${ROOT}/cases/${t.id}/program.adj
4. Re-run the engine to get the exact deterministic trace (Bash):
   ADJ52_RULEBOOK=cases/${t.id}/rulebook.adj ADJ52_PROGRAM=cases/${t.id}/program.adj cargo run --quiet --manifest-path "${MANIFEST}" --bin adj52 .
5. Compare the engine's ranked posteriors + fired clauses against the GROUND TRUTH and the blind JUDGE'S rationale from the context file.

NOW DIAGNOSE THE CALIBRATION FAILURE. The three candidate levers are:
- H1_correlation: the posterior saturated because several CORRELATED findings (manifestations of one underlying mechanism) each fired an INDEPENDENT positive contributes, double-counting. The fix is to group them under one "% mechanism" directive so they fire once. Evidence = count the independent same-sign contributes to the winning conclusion; check whether the rulebook used "% mechanism" at all.
- H2_open_question: the engine reports near-certainty WHILE a decision-relevant confirmatory test is still unresolved (an uncertain{} marker bears on the conclusion, or the answer recommends a confirmatory test). The fix is to HOLD residual probability / cap the posterior until that test is observed. Evidence = an open uncertainty + a saturated posterior + a recommended confirmatory test.
- H3_residual_mass: the engine committed hard to ONE named candidate (sometimes the WRONG sibling) because there is no "none-of-the-above / other" hypothesis to absorb probability when the evidence is ambiguous or weak. The fix is an explicit residual hypothesis. Evidence = a wrong or over-narrow top-1 winning at high posterior while the true answer was a sibling or unlisted.

Determine: is top-1 correct? What is the failure_class? Which is the PRIMARY lever (the one that most helps THIS case), and a secondary if any? Fill every field. In what_fix_needed, be concrete about why the fix helps this case WITHOUT reordering the top-1 on cases the engine already gets right.`

const results = await pipeline(
  targets,
  (t) => agent(prompt(t), { phase: 'RootCause', label: `rootcause:${t.id}`, agentType: 'general-purpose', schema: SCHEMA }).catch(() => null),
)
const done = results.filter(Boolean)

// Tally lever demand across the wrong cases.
const wrong = done.filter((r) => !r.top1_correct || r.failure_class !== 'none')
const leverCount = {}
for (const r of done) { leverCount[r.primary_lever] = (leverCount[r.primary_lever] || 0) + 1 }
log(`root-caused ${done.length}; primary-lever demand: ${JSON.stringify(leverCount)}`)
return { lever_demand: leverCount, per_case: done }
