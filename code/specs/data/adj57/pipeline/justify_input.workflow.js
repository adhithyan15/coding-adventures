export const meta = {
  name: 'adj62-input-justification',
  description: 'ADJ62 — apply the justification gate to the INPUT side. After decomposing, ask the agent: what facts did you EXTRACT or INFER from this, which bytes do they come from, and JUSTIFY why those bytes prove the extraction. Coverage (every byte used-or-discarded) proves nothing was dropped; this proves nothing was MIS-extracted. Two layers, same as ADJ61: (1) byte-anchor — every cited span verbatim; (2) justification — an adversarial verifier confirms the cited bytes prove (extracted) or warrant (inferred) the fact. Re-uses the neurobrucellosis bytes.',
  phases: [{ title: 'Decompose' }, { title: 'Extract' }, { title: 'Verify' }, { title: 'Kickback' }],
}

const CASE_TEXT = `A forty-year Indian, working in pharmaceutical company, had gone on a business trip to Africa traveling through Uganda, Tanzania and Kenya. While he was in Africa, he developed a swelling over the left ankle joint after an insect bite. This swelling was painful and followed by high grade fever with chills and rigors after 3-4 days. There was no history of any joint pain. There was no other clue on history to localize the cause of fever. There was no history of high risk behavior. Patient had past history of essential tremors for last 6 years, without any functional disability. There was no history of hypertension, diabetes, stroke, or tuberculosis in the past. He took empirical antibiotics in Africa which resulted minor relief in his symptoms. On examination, he was febrile with oral temperature of 102 F. He had tachycardia with pulse rate of 116/min, blood pressure was 124/80 mm of Hg. There was an erythematous swelling over the left ankle, with peri-lesional erythematous rashes. There was no other skin rash elsewhere. There was no pallor, icterus, or peripheral lymphnode enlargement. Examination of respiratory and cardiovascular system was normal. Abdomen examination revealed hepatomegaly of 3 cm and splenomegaly of 3 cm below costal margin. Central nervous system examination revealed coarse tremors in bilateral hands. Blood smear for malaria parasite was negative. Cultures from blood and urine were sterile. Widal test showed normal titers. Testing for viral markers were negative. Cerebrospinal fluid examination was acellular with normal glucose. The proteins were raised. Bone biopsy showed single ill defined granuloma. Magnetic resonance imaging of brain showed mild meningeal enhancement. Few small lesions noticed in pons, which were hyperintense on T2-weighted and FLAIR images.`

const DECOMPOSE_SCHEMA = {
  type: 'object',
  required: ['segments'],
  properties: {
    segments: {
      type: 'array',
      description: 'ordered partition of the case text; concatenating segment.text reproduces it character-for-character (the COVERAGE invariant — nothing dropped).',
      items: {
        type: 'object', required: ['text', 'kind'],
        properties: {
          text: { type: 'string' }, kind: { type: 'string', enum: ['fact', 'discard'] },
          reason: { type: 'string', description: 'for a discard: why this span carries no fact' },
        },
      },
    },
  },
}

// the NEW input-side question: justify every fact against the bytes it came from.
const FACTS_SCHEMA = {
  type: 'object',
  required: ['facts'],
  properties: {
    facts: {
      type: 'array',
      description: 'every fact you extracted or inferred from the case text.',
      items: {
        type: 'object', required: ['fact', 'kind', 'grounded_by', 'justification'],
        properties: {
          fact: { type: 'string', description: 'one atomic fact' },
          kind: { type: 'string', enum: ['extracted', 'inferred'], description: 'extracted = the bytes STATE it directly. inferred = you DERIVED it from the bytes (a reading/interpretation).' },
          grounded_by: { type: 'array', items: { type: 'string' }, description: 'the bytes this fact comes from — span(s) copied VERBATIM from the case text (combine several if the fact is built from several)' },
          justification: { type: 'string', description: 'why those exact bytes PROVE this fact (extracted) or WARRANT it (inferred)' },
        },
      },
    },
  },
}

const VERIFY_SCHEMA = {
  type: 'object',
  required: ['verdicts'],
  properties: {
    verdicts: {
      type: 'array',
      description: 'one verdict per fact, same order',
      items: {
        type: 'object', required: ['idx', 'justified', 'note'],
        properties: {
          idx: { type: 'integer' },
          justified: { type: 'boolean', description: 'true iff the CITED bytes prove (extracted) or warrant (inferred) the fact at its stated strength' },
          note: { type: 'string' },
        },
      },
    },
  },
}

const decomposePrompt = `Decompose this case text into an ordered partition of segments. Concatenating every segment.text MUST reproduce the case text character-for-character (every space, every period). Each segment is a fact (carries information) or a discard (filler/connective with a reason). This is the COVERAGE step — nothing may be dropped.

CASE TEXT:
${CASE_TEXT}`

const extractPrompt = `You have decomposed the case below. Now account for what you took from it.

CASE TEXT:
${CASE_TEXT}

List EVERY fact you extracted or inferred. For each: the fact; its kind — "extracted" (the bytes STATE it directly) or "inferred" (you DERIVED it, a reading/interpretation); grounded_by = the byte span(s) copied VERBATIM from the case text it comes from (combine several spans if the fact is built from several); and a justification of why those exact bytes PROVE the fact (if extracted) or WARRANT it (if inferred).

Be honest about kind: if a fact is your interpretation (e.g. calling a pulse of 116 "tachycardia", or "minor relief" a "partial response"), it is INFERRED, not extracted — say so and justify the inference. Do NOT claim to have extracted a fact the bytes do not state.`

const verifyPrompt = (facts) => `You are an adversarial extraction verifier. For EACH fact, decide whether the CITED bytes — and only those — justify it at its stated kind. Try to REFUTE each; default justified=false when the cited bytes do not carry it.

THE CASE TEXT (the only ground truth):
${CASE_TEXT}

Rules:
  - extracted: the cited bytes must STATE or unambiguously contain the fact. If the fact adds anything the bytes do not say (an interpretation, a label, a clinical reading), it is NOT "extracted" — justified=false (it should have been kind=inferred).
  - inferred: the cited bytes, combined, must WARRANT the fact as a reasonable reading, and it must be a defensible interpretation (not a leap). If warranted, justified=true even though the bytes do not state it verbatim.

FACTS:
${facts.map((f, i) => `  [${i}] kind=${f.kind} :: "${f.fact}"\n       cites: ${JSON.stringify(f.grounded_by)}\n       claim-justification: ${f.justification}`).join('\n')}`

const kickbackPrompt = (rejected) => `The INPUT justification gate REJECTED these fact extractions:
${rejected.map((r) => `  - [${r.kind}] "${r.fact}"\n      reason: ${r.reason}`).join('\n')}

Fix EACH by ONE of:
  (a) cite the exact byte span(s) that prove it (copy verbatim; combine several if needed);
  (b) if you labelled an interpretation as "extracted", re-file it as kind="inferred" with a justification of the inference;
  (c) if the bytes do not support it at all, REMOVE the fact.
Re-emit the FULL corrected facts list. Every fact must be byte-anchored and justified at its stated kind.`

// ---- the two-layer gate (mirror of justify_gate.py) ----
const anchored = (f) => {
  const spans = (f.grounded_by || []).filter(Boolean)
  return spans.length > 0 && spans.every((s) => CASE_TEXT.includes(s))
}

async function verify(facts) {
  const v = await agent(verifyPrompt(facts), { phase: 'Verify', label: 'verify', agentType: 'general-purpose', schema: VERIFY_SCHEMA })
  const byIdx = new Map((v.verdicts || []).map((x) => [x.idx, x]))
  return facts.map((f, i) => {
    const verdict = byIdx.get(i) || { justified: false, note: 'no verdict returned' }
    const isAnchored = anchored(f)
    const grounded = isAnchored && !!verdict.justified
    const reason = grounded ? ''
      : (!isAnchored ? 'byte-anchor FAIL — a cited span is not verbatim in the case text (or no citation)'
        : `justification FAIL — ${verdict.note}`)
    return { ...f, anchored: isAnchored, justified: !!verdict.justified, verifier_note: verdict.note, grounded, reason }
  })
}

// ---- run ----
// (1) COVERAGE — decompose into a byte-covering partition.
const decomp = await agent(decomposePrompt, { phase: 'Decompose', label: 'decompose', agentType: 'general-purpose', schema: DECOMPOSE_SCHEMA })
const concat = decomp.segments.map((s) => s.text).join('')
const coverage_ok = concat === CASE_TEXT
log(`coverage: segments ${coverage_ok ? 'reproduce' : 'DO NOT reproduce'} the case text (${concat.length}/${CASE_TEXT.length} bytes)`)

// (2) EXTRACTION JUSTIFICATION — what did you extract/infer, from which bytes, why.
let extracted = await agent(extractPrompt, { phase: 'Extract', label: 'extract', agentType: 'general-purpose', schema: FACTS_SCHEMA })
let graded = await verify(extracted.facts)
const attempts = [{ attempt: 1, n_facts: graded.length, n_rejected: graded.filter((g) => !g.grounded).length }]

for (let i = 2; i <= 4; i++) {
  const rejected = graded.filter((g) => !g.grounded)
  if (rejected.length === 0) break
  log(`input justification gate: ${rejected.length} fact(s) rejected -> kicking back (attempt ${i})`)
  extracted = await agent(kickbackPrompt(rejected), { phase: 'Kickback', label: `re-extract:attempt-${i}`, agentType: 'general-purpose', schema: FACTS_SCHEMA })
  graded = await verify(extracted.facts)
  attempts.push({ attempt: i, n_facts: graded.length, n_rejected: graded.filter((g) => !g.grounded).length })
}

return {
  case_text: CASE_TEXT,
  segments: decomp.segments,
  coverage_ok,
  graded_facts: graded,
  attempts,
  final_rejected: graded.filter((g) => !g.grounded).length,
}
