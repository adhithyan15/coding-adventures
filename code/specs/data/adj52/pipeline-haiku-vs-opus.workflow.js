export const meta = {
  name: 'adj52-haiku-fw-vs-opus',
  description: 'Defensibility test: HAIKU + framework discipline (IR -> cited rulebook -> grounded cited conclusion, NO engine) vs PLAIN OPUS (blind), on 100 perturbed clinical cases. The blind judge ranks on DEFENSIBILITY (can a reviewer trace the reasoning, locate where any error entered, and correct it) as the PRIMARY axis; correctness is secondary/tiebreak. Tests the hypothesis: Haiku+framework is as defensible or more than plain Opus.',
  phases: [
    { title: 'Prepare' },
    { title: 'OpusBlind', model: 'opus' },
    { title: 'HaikuIR', model: 'haiku' },
    { title: 'HaikuDerive', model: 'haiku' },
    { title: 'HaikuConclude', model: 'haiku' },
    { title: 'Judge' },
  ],
}

const SPECIALTIES = [
  'cardiology', 'pulmonology', 'nephrology', 'endocrinology', 'infectious disease',
  'hematology', 'medical oncology', 'rheumatology', 'neurology', 'gastroenterology',
  'dermatology', 'hepatology', 'clinical immunology', 'emergency medicine',
  'geriatric medicine', 'vascular medicine', 'otolaryngology', 'urology',
  'allergy and immunology', 'general internal medicine',
]
const ANGLES = [
  'where the initial working diagnosis turned out to be wrong',
  'where the presentation mimicked a far more common condition',
  'with an unexpected final diagnosis after an initially misleading workup',
  'initially misattributed to a benign or unrelated cause',
  'where a rare disease masqueraded as a common one',
]
const SEEDS = []
for (const s of SPECIALTIES) for (const a of ANGLES) SEEDS.push(`an adult clinical case report in ${s} ${a}`)
// Full run: all 100 seeds (20 specialties x 5 angles). Smoke used SEEDS.slice(0, 3).
const seeds = (args && args.seeds && args.seeds.length) ? args.seeds : SEEDS
if (!seeds.length) { log('no seeds'); return { error: 'no seeds' } }
log(`haiku-fw vs opus over ${seeds.length} seed(s)`)

const PREPARE_SCHEMA = {
  type: 'object',
  required: ['skipped', 'ground_truth', 'prose', 'perturbations', 'diagnosis_unchanged'],
  properties: {
    skipped: { type: 'boolean' }, source_url: { type: 'string' },
    ground_truth: { type: 'string' }, prose: { type: 'string' },
    perturbations: { type: 'array', items: { type: 'string' } },
    diagnosis_unchanged: { type: 'boolean' },
  },
}
const BLIND_SCHEMA = { type: 'object', required: ['answer_text'], properties: { answer_text: { type: 'string' } } }
const IR_SCHEMA = {
  type: 'object', required: ['facts', 'queries'],
  properties: { inferred_domain: { type: 'string' }, facts: { type: 'array', items: { type: 'object' } }, uncertainties: { type: 'array', items: { type: 'object' } }, queries: { type: 'array', items: { type: 'object' } }, discarded: { type: 'array', items: { type: 'object' } } },
}
const RULEBOOK_SCHEMA = { type: 'object', required: ['rules'], properties: { rules: { type: 'array', items: { type: 'object' } } } }
const CONCLUDE_SCHEMA = {
  type: 'object', required: ['top_diagnosis', 'recommended_next_step', 'confidence', 'conclusion_text', 'citations'],
  properties: { top_diagnosis: { type: 'string' }, recommended_next_step: { type: 'string' }, confidence: { type: 'string' }, conclusion_text: { type: 'string' }, citations: { type: 'array', items: { type: 'object' } } },
}
const VERDICT_SCHEMA = {
  type: 'object',
  required: ['winner', 'a_defensibility', 'b_defensibility', 'a_correct', 'b_correct', 'rationale'],
  properties: {
    winner: { type: 'string', enum: ['A', 'B', 'tie'] },
    a_defensibility: { type: 'string', enum: ['high', 'medium', 'low'] },
    b_defensibility: { type: 'string', enum: ['high', 'medium', 'low'] },
    a_correct: { type: 'string', enum: ['correct', 'partial', 'incorrect'] },
    b_correct: { type: 'string', enum: ['correct', 'partial', 'incorrect'] },
    rationale: { type: 'string' },
  },
}

const preparePrompt = (seed) => `Find and prepare a published clinical case for a blinded experiment. SEED: ${seed}.
1. WebSearch/WebFetch to FIND ONE real published case report matching the seed (prefer open-access PMC) with a CLEARLY DOCUMENTED final/confirmed diagnosis. Favor initially-misleading presentations. AVOID famous textbook cases. If none, skipped=true.
2. Extract GROUND TRUTH (final dx + how confirmed + correct disposition) + source_url. Held aside.
3. PERTURB + SANITISE: remove the dx/confirmatory result/discussion; change every diagnosis-irrelevant detail (age in dx-preserving range, sex/ethnicity if irrelevant, all lab numbers shifted but same side of reference + magnitude, anecdotes, timeline, doses, institution, ordering, phrasing); PRESERVE every load-bearing finding so the dx is UNCHANGED.
Return skipped, source_url, ground_truth, prose, perturbations, diagnosis_unchanged.`

const blindPrompt = (prose) => `Read this clinical case and answer as you normally would. Do NOT look up the specific published case; do not read local files. Give the most likely diagnosis, the recommended next action, your confidence, and your reasoning.

=== CASE ===
${prose}
=== END ===`

const irPrompt = (prose) => `Decompose this clinical case into a human-readable IR. Read EVERY byte: each span is a typed fact OR discarded with a reason. Ambiguity becomes an uncertainty, never a guess. Infer the domain. Raise the queries. Do NOT solve it; do NOT look up the outcome.
Return inferred_domain, facts [{id, term, source_span}], uncertainties [{id, about, domain}], queries [{id, asks}], discarded [{source_span, reason}].

=== CASE ===
${prose}
=== END ===`

const derivePrompt = (irJson) => `Given this ingested IR (facts + queries), derive the rulebook needed to answer the queries — clinical rules linking findings to candidate diagnoses. Use WebSearch/WebFetch for REAL citations. Recurse into subtypes. No program, no probabilities — a human-readable cited rulebook only.
Return rules: [{id, rule, bears_on, direction, source}].

=== IR ===
${irJson}`

const concludePrompt = (prose, irJson, rbJson) => `You have a case, its IR, and a derived cited rulebook. WITHOUT writing any program or running any engine, reason to a conclusion. Use ONLY facts in the IR and rules in the rulebook; for every key claim CITE the supporting IR fact id(s) and rule id(s); give an honest confidence; name the recommended next step.
Return top_diagnosis, recommended_next_step, confidence, conclusion_text (full answer with inline citations to fact/rule ids), citations [{claim, input_facts:[ids], rules:[ids]}].

=== CASE ===
${prose}
=== IR ===
${irJson}
=== RULEBOOK ===
${rbJson}`

const judgePrompt = (gt, a, b) => `You are an impartial judge. Two responses (OUTPUT A, OUTPUT B) from hidden systems, scored against the ground truth.
PRIMARY criterion is DEFENSIBILITY: can a reviewer TRACE the reasoning, LOCATE where any error entered, and CORRECT it at a specific step? — i.e. reasoning grounded in explicitly stated facts/rules with citations and an inspectable line of thinking, versus an opaque assertion you must take on trust. An answer that is auditable and whose error (if any) is localizable is MORE defensible, even if less polished.
SECONDARY: correctness vs ground truth (use only to break a defensibility tie).
Score each output's defensibility (high/medium/low) and correctness (correct/partial/incorrect). Pick the winner on DEFENSIBILITY.

=== GROUND TRUTH ===
${gt}

=== OUTPUT A ===
${a}

=== OUTPUT B ===
${b}`

const results = await pipeline(
  seeds,
  (seed, _o, idx) => agent(preparePrompt(seed), { phase: 'Prepare', label: `prepare:case-${idx + 1}`, agentType: 'general-purpose', schema: PREPARE_SCHEMA })
    .then((p) => {
      if (p.skipped || !p.prose) { throw new Error(`no case for seed ${idx + 1}`) }
      return { id: `case-${idx + 1}`, source_url: p.source_url, ground_truth: p.ground_truth, prose: p.prose, perturbations: p.perturbations, diagnosis_unchanged: p.diagnosis_unchanged }
    }),
  (o) => agent(blindPrompt(o.prose), { phase: 'OpusBlind', label: `opus:${o.id}`, model: 'opus', agentType: 'general-purpose', schema: BLIND_SCHEMA })
    .then((b) => ({ ...o, opus_answer: b.answer_text })),
  (o) => agent(irPrompt(o.prose), { phase: 'HaikuIR', label: `ir:${o.id}`, model: 'haiku', agentType: 'general-purpose', schema: IR_SCHEMA })
    .then((ir) => ({ ...o, ir })),
  (o) => agent(derivePrompt(JSON.stringify(o.ir)), { phase: 'HaikuDerive', label: `derive:${o.id}`, model: 'haiku', agentType: 'general-purpose', schema: RULEBOOK_SCHEMA })
    .then((rb) => ({ ...o, rulebook: rb.rules })),
  (o) => agent(concludePrompt(o.prose, JSON.stringify(o.ir), JSON.stringify(o.rulebook)), { phase: 'HaikuConclude', label: `conclude:${o.id}`, model: 'haiku', agentType: 'general-purpose', schema: CONCLUDE_SCHEMA })
    .then((c) => ({ ...o, framework: c })),
  (o, _o2, idx) => {
    const fwIsA = (idx % 2) === 0
    const A = fwIsA ? o.framework.conclusion_text : o.opus_answer
    const B = fwIsA ? o.opus_answer : o.framework.conclusion_text
    return agent(judgePrompt(o.ground_truth, A, B), { phase: 'Judge', label: `judge:${o.id}`, agentType: 'general-purpose', schema: VERDICT_SCHEMA })
      .then((v) => ({
        id: o.id, source_url: o.source_url, diagnosis_unchanged: o.diagnosis_unchanged,
        prose: o.prose, ground_truth: o.ground_truth, ir: o.ir, rulebook: o.rulebook,
        opus_answer: o.opus_answer, framework_conclusion: o.framework.conclusion_text,
        framework_top: o.framework.top_diagnosis, framework_citations: o.framework.citations,
        framework_is: fwIsA ? 'A' : 'B',
        winner: v.winner,
        framework_defensibility: fwIsA ? v.a_defensibility : v.b_defensibility,
        opus_defensibility: fwIsA ? v.b_defensibility : v.a_defensibility,
        framework_correct: fwIsA ? v.a_correct : v.b_correct,
        opus_correct: fwIsA ? v.b_correct : v.a_correct,
        framework_won: v.winner === (fwIsA ? 'A' : 'B'),
        opus_won: v.winner === (fwIsA ? 'B' : 'A'),
        rationale: v.rationale,
      }))
  }
)

const done = results.filter(Boolean)
const hi = (v) => v === 'high'
const c = (v) => v === 'correct'
const tally = {
  seeds_attempted: seeds.length, cases_completed: done.length, skipped_or_failed: seeds.length - done.length,
  framework_won_defensibility: done.filter((r) => r.framework_won).length,
  opus_won_defensibility: done.filter((r) => r.opus_won).length,
  tie: done.filter((r) => r.winner === 'tie').length,
  framework_defensibility_high: done.filter((r) => hi(r.framework_defensibility)).length,
  opus_defensibility_high: done.filter((r) => hi(r.opus_defensibility)).length,
  framework_as_or_more_defensible: done.filter((r) => r.framework_won || r.winner === 'tie').length,
  framework_correct: done.filter((r) => c(r.framework_correct)).length,
  opus_correct: done.filter((r) => c(r.opus_correct)).length,
}
log(`HAIKU+FW vs OPUS: ${tally.cases_completed}/${tally.seeds_attempted}; defensibility wins framework ${tally.framework_won_defensibility} / opus ${tally.opus_won_defensibility} / tie ${tally.tie} (framework as-or-more-defensible: ${tally.framework_as_or_more_defensible}); defensibility-high framework ${tally.framework_defensibility_high} vs opus ${tally.opus_defensibility_high}; correctness framework ${tally.framework_correct} vs opus ${tally.opus_correct}`)
return { tally, per_case: done }
