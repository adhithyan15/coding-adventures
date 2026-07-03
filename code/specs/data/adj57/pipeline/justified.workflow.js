export const meta = {
  name: 'adj61-justification-gate',
  description: 'ADJ61 — refine ADJ60. A claim is grounded not by VERBATIM substring match but by JUSTIFICATION: combine multiple input bytes into one fact, and the combination must justify it. Two layers: (1) byte-anchor — every cited span verbatim in the input; (2) justification — an adversarial verifier confirms the cited bytes justify the claim. Claims are typed evidence (statement about the input — strict) vs conclusion (inference from the evidence — allowed as a hedged hypothesis). Re-runs the neurobrucellosis case to test whether the framework can now reach a JUSTIFIED conclusion instead of refusing to name it.',
  phases: [{ title: 'Derive' }, { title: 'Verify' }, { title: 'Kickback' }],
}

// ---- the fixed neurobrucellosis case (PMC2769393), reused from the ADJ60 run for a
//      direct before/after. case_text + facts are byte-identical to grounded-results.json.
const CASE_TEXT = `A forty-year Indian, working in pharmaceutical company, had gone on a business trip to Africa traveling through Uganda, Tanzania and Kenya. While he was in Africa, he developed a swelling over the left ankle joint after an insect bite. This swelling was painful and followed by high grade fever with chills and rigors after 3-4 days. There was no history of any joint pain. There was no other clue on history to localize the cause of fever. There was no history of high risk behavior. Patient had past history of essential tremors for last 6 years, without any functional disability. There was no history of hypertension, diabetes, stroke, or tuberculosis in the past. He took empirical antibiotics in Africa which resulted minor relief in his symptoms. On examination, he was febrile with oral temperature of 102 F. He had tachycardia with pulse rate of 116/min, blood pressure was 124/80 mm of Hg. There was an erythematous swelling over the left ankle, with peri-lesional erythematous rashes. There was no other skin rash elsewhere. There was no pallor, icterus, or peripheral lymphnode enlargement. Examination of respiratory and cardiovascular system was normal. Abdomen examination revealed hepatomegaly of 3 cm and splenomegaly of 3 cm below costal margin. Central nervous system examination revealed coarse tremors in bilateral hands. Blood smear for malaria parasite was negative. Cultures from blood and urine were sterile. Widal test showed normal titers. Testing for viral markers were negative. Cerebrospinal fluid examination was acellular with normal glucose. The proteins were raised. Bone biopsy showed single ill defined granuloma. Magnetic resonance imaging of brain showed mild meningeal enhancement. Few small lesions noticed in pons, which were hyperintense on T2-weighted and FLAIR images.`
const FACTS = ['travel_to_east_africa', 'ankle_swelling_after_insect_bite', 'high_grade_fever_with_chills_rigors', 'no_joint_pain', 'no_high_risk_behavior', 'chronic_essential_tremor', 'no_chronic_comorbidities', 'partial_response_to_empirical_antibiotics', 'febrile_102F', 'tachycardia_normotensive', 'ankle_erythematous_swelling', 'no_generalized_rash', 'no_pallor_icterus_lymphadenopathy', 'normal_resp_cvs_exam', 'hepatosplenomegaly', 'bilateral_hand_coarse_tremors', 'malaria_smear_negative', 'blood_urine_cultures_sterile', 'widal_negative', 'viral_markers_negative', 'csf_acellular_normal_glucose', 'csf_raised_protein', 'bone_granuloma', 'mri_meningeal_enhancement', 'pontine_t2_flair_lesions']

const DERIVE_SCHEMA = {
  type: 'object',
  required: ['leading_answer', 'claims'],
  properties: {
    leading_answer: { type: 'string', description: 'the conclusion, stated as a hedged inference (a leading hypothesis), e.g. "the most likely diagnosis is X, inferred from the findings below"' },
    claims: {
      type: 'array',
      description: 'the atomic assertions. Each is typed evidence or conclusion, and grounded_by verbatim input spans it draws on (you MAY combine several spans into one fact).',
      items: {
        type: 'object', required: ['claim', 'kind', 'grounded_by'],
        properties: {
          claim: { type: 'string' },
          kind: { type: 'string', enum: ['evidence', 'conclusion'], description: 'evidence = a statement ABOUT the input (must be supported by the cited bytes). conclusion = an INFERENCE from the evidence (a hedged hypothesis).' },
          grounded_by: { type: 'array', items: { type: 'string' }, description: 'one or more spans copied VERBATIM from the case text or a fact term that this claim draws on' },
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
      description: 'one verdict per claim, in the SAME ORDER as the claims given',
      items: {
        type: 'object', required: ['idx', 'justified', 'note'],
        properties: {
          idx: { type: 'integer', description: 'the 0-based index of the claim this verdict is for' },
          justified: { type: 'boolean', description: 'true iff the CITED input spans, taken together, justify the claim at its stated strength' },
          note: { type: 'string', description: 'one line: which cited bytes carry it, or why they do not' },
        },
      },
    },
  },
}

const derivePrompt = `Answer this case. You may COMBINE several input bytes into one fact, and you may state a CONCLUSION — but every claim must be JUSTIFIED by the input bytes it cites.

CASE TEXT:
${CASE_TEXT}

EXTRACTED FACTS (verbatim terms you may also cite):
${FACTS.map((f) => `  - ${f}`).join('\n')}

Emit leading_answer + claims. Type EACH claim:
  - kind="evidence": a statement ABOUT the input (an observation/finding). Its cited bytes must state or directly imply it. Do NOT assert a finding the case does not contain (e.g. a test result that was not reported) — that is invention.
  - kind="conclusion": an INFERENCE from the evidence (the diagnosis/identification, a mechanism, a unifying pattern). You ARE allowed to name it even if the name is not a byte — PROVIDED it rests only on the byte-grounded evidence above and you state it as a hedged inference (a leading hypothesis), not as a byte-fact.
Every claim's grounded_by must be spans copied VERBATIM from the case text or a fact term. Combining several spans into one fact is encouraged where that is what justifies it. The leading_answer must be a hedged conclusion that the claims justify.`

const verifyPrompt = (claims) => `You are an adversarial grounding verifier. For EACH claim below, decide whether the CITED input spans — and ONLY those spans, read together — JUSTIFY the claim at its stated strength. Try to REFUTE each one; default justified=false when the cited spans do not carry the claim.

THE INPUT (the only ground truth — nothing outside it counts as evidence):
${CASE_TEXT}

Rules by claim kind:
  - evidence: the cited spans must STATE or DIRECTLY IMPLY the claim. If the claim asserts a finding/result not present in the cited bytes, justified=false (it is invention — even if it is medically plausible).
  - conclusion: the cited spans, COMBINED, must make the claim the WARRANTED reading, AND the claim must be hedged as an inference (a leading/most-likely hypothesis), not asserted as a confirmed byte-fact. If the cited evidence genuinely points to it as the leading explanation, justified=true even though the name itself is not in the bytes. If it over-asserts confirmation the bytes lack, or a different explanation fits the same bytes equally, justified=false.

CLAIMS (verdict per claim, same order, by idx):
${claims.map((c, i) => `  [${i}] kind=${c.kind} :: "${c.claim}"\n       cites: ${JSON.stringify(c.grounded_by)}`).join('\n')}`

const kickbackPrompt = (prev, rejected) => `The JUSTIFICATION gate REJECTED these claims:
${rejected.map((r) => `  - [${r.kind}] "${r.claim}"\n      reason: ${r.reason}`).join('\n')}

Your previous leading_answer: "${prev.leading_answer}".

Fix EACH rejected claim by ONE of:
  (a) cite span(s) copied EXACTLY from the case text or a fact term that genuinely justify it (you may combine several);
  (b) if it is an over-asserted conclusion, SOFTEN it to a hedged inference (a leading hypothesis) that the cited evidence supports;
  (c) if it is an evidence claim the input does not contain, REMOVE it (and remove any specificity it lent the leading_answer).
Re-emit the FULL corrected leading_answer + claims (each typed evidence|conclusion, each grounded_by verbatim spans).`

// ---- the two-layer gate (mirror of justify_gate.py) ----
const HAY = CASE_TEXT + '\n' + FACTS.join('\n')
const anchored = (c) => {
  const spans = (c.grounded_by || []).filter(Boolean)
  return spans.length > 0 && spans.every((s) => HAY.includes(s))
}

async function verify(claims) {
  const v = await agent(verifyPrompt(claims), { phase: 'Verify', label: 'verify', agentType: 'general-purpose', schema: VERIFY_SCHEMA })
  const byIdx = new Map((v.verdicts || []).map((x) => [x.idx, x]))
  return claims.map((c, i) => {
    const verdict = byIdx.get(i) || { justified: false, note: 'no verdict returned' }
    const isAnchored = anchored(c)
    const grounded = isAnchored && !!verdict.justified
    let reason = ''
    if (!grounded) {
      reason = !isAnchored
        ? 'byte-anchor FAIL — a cited span is not verbatim in the input (or no citation)'
        : `justification FAIL — ${verdict.note}`
    }
    return { ...c, anchored: isAnchored, justified: !!verdict.justified, justification: verdict.note, grounded, reason }
  })
}

// ---- run ----
let derived = await agent(derivePrompt, { phase: 'Derive', label: 'derive:attempt-1', agentType: 'general-purpose', schema: DERIVE_SCHEMA })
let graded = await verify(derived.claims)
const attempts = [{ attempt: 1, leading_answer: derived.leading_answer, n_claims: graded.length, n_rejected: graded.filter((g) => !g.grounded).length }]

for (let i = 2; i <= 4; i++) {
  const rejected = graded.filter((g) => !g.grounded)
  if (rejected.length === 0) break
  log(`justification gate: ${rejected.length} claim(s) rejected -> kicking back (attempt ${i})`)
  derived = await agent(kickbackPrompt(derived, rejected), { phase: 'Kickback', label: `re-derive:attempt-${i}`, agentType: 'general-purpose', schema: DERIVE_SCHEMA })
  graded = await verify(derived.claims)
  attempts.push({ attempt: i, leading_answer: derived.leading_answer, n_claims: graded.length, n_rejected: graded.filter((g) => !g.grounded).length })
}

return {
  case_text: CASE_TEXT,
  facts: FACTS,
  leading_answer: derived.leading_answer,
  graded_claims: graded,
  attempts,
  final_rejected: graded.filter((g) => !g.grounded).length,
}
