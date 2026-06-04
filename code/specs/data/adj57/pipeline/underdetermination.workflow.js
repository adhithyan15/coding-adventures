export const meta = {
  name: 'adj64-underdetermination',
  description: 'ADJ64 — the underdetermination gate. Take the byte-grounded leading conclusion from the ADJ63 axle run and ask: what rival explanations fit the SAME bytes, and for each, is the discriminating observation PRESENT (cited verbatim) or ABSENT? If any rival cannot be ruled out because its deciding datum is missing, the conclusion is underdetermined — soften it to a disjunction and emit the missing data as named provenance holes (queries the spider/CAS can fetch). Demonstrates catching over-attribution-under-missing-evidence, the failure the byte-anchor alone does not catch.',
  phases: [{ title: 'Rivals' }, { title: 'Soften' }],
}

// The axle case + the ADJ63 leading conclusion (byte-identical to bidirectional-results.json).
const CASE_TEXT = `During the development of a prototype PMDD electric locomotive, the newly designed axles exhibited premature failures during wheel-seat–axle fatigue testing, failing to meet validation requirements. After 3 million fatigue tests, axle 1# broke at the transition corner. After 3.5 million fatigue tests, it was found that magnetic marks gathered on both sides of the transition corners of axle 2#. After 1.6 million tests, accumulated magnetic marks were also found on both sides of the # 3 axle's transition angle. The identified dual fatigue initiation sites on surfaces A and B at the transition's rounded corners exhibit characteristic bending fatigue patterns. Beach marks are present at both the top and bottom regions of the fracture section. The final fracture exhibits mixed modes: ductile shear lips at edges and cleavage-dominated brittle fracture in the central zone. Parallel distributions of rough machining textures can be observed on the side of the crack source, and local corrosion pits exist. The width of the blade marks on the surface is about 0.95 mm, and there are parallel processing textures at the bottom. The height difference between the highest point and the lowest point near the transition zone is 342 μm. There are small V-shaped notches in this section. The chemical composition test results are in line with the standard requirements of EA1N in UIC 811-1. The microstructures at the crack source are the same as those at the core, all of which are flake pearlite + ferrite. The grain size is grade 7, and no characteristics of oxidation, decarburization, and overheated microstructure are found. No defects, such as porosity, slag inclusion, and repair welding, were found in this area. The surface of the arc region is manifested as tensile stress in both circumferential and axial directions. The arc transition zone exhibits residual tensile stress values of 72–99 MPa in the circumferential direction and 71–92 MPa in the axial direction.`
const LEADING_ANSWER = `The axle failures are most plausibly a case of bending-fatigue cracking initiated at the machined transition fillet, where the leading hypothesis is that the failure was driven by manufacturing/surface condition rather than material defect: rough machining textures, blade marks, surface notches/roughness, corrosion pits, and detrimental residual tensile stress at the fillet acted together as stress concentrators that nucleated dual fatigue cracks, while chemical composition and microstructure were normal and conforming.`

const RIVALS_SCHEMA = {
  type: 'object',
  required: ['rivals'],
  properties: {
    rivals: {
      type: 'array',
      description: 'alternative explanations that fit the SAME bytes as the leading conclusion — the ones a careful reviewer would not be able to rule out from this text alone.',
      items: {
        type: 'object', required: ['hypothesis', 'discriminating_observation', 'present', 'citation'],
        properties: {
          hypothesis: { type: 'string', description: 'the rival explanation (e.g. a different root cause)' },
          discriminating_observation: { type: 'string', description: 'the SINGLE observation/measurement that would let you choose between the leading conclusion and this rival' },
          present: { type: 'boolean', description: 'is that discriminating observation actually present in the CASE TEXT?' },
          citation: { type: 'string', description: 'if present: the span copied VERBATIM from the case text that supplies it. if absent: empty string.' },
        },
      },
    },
  },
}

const SOFTEN_SCHEMA = {
  type: 'object',
  required: ['corrected_answer'],
  properties: {
    corrected_answer: { type: 'string', description: 'the honest answer: grounded findings kept; the cause stated as a disjunction over the rivals that cannot be ruled out; the missing discriminating data named as what must be retrieved to decide.' },
  },
}

const rivalsPrompt = `Here is a leading conclusion and the case text it was drawn from. Your job is adversarial: find the rival explanations that fit the SAME bytes equally well, so the leading conclusion may be OVER-ATTRIBUTING.

CASE TEXT:
${CASE_TEXT}

LEADING CONCLUSION:
${LEADING_ANSWER}

List the rival hypotheses a careful failure analyst could NOT rule out from this text alone (e.g. a different root cause that the same fractography is consistent with). For EACH rival give:
  - the SINGLE discriminating observation that would settle leading-vs-rival (the one measurement/test whose result picks a side);
  - present = whether that discriminating observation is actually IN the case text;
  - citation = if present, the verbatim span; if absent, "".
Be ruthless about "present": if the deciding measurement (e.g. a comparison of operating stress to the material's fatigue limit) is NOT written in the case text, mark present=false. Do not credit the text with data it does not contain.`

const softenPrompt = (open, holes) => `The leading conclusion is UNDERDETERMINED: these rival explanations fit the same bytes and cannot be ruled out, because the deciding observation is missing from the input:
${open.map((o) => `  - rival: ${o.hypothesis}\n      would need: ${o.discriminating_observation} (NOT in the case text)`).join('\n')}

LEADING CONCLUSION (do not just repeat it — it over-attributes):
${LEADING_ANSWER}

Rewrite it honestly: (1) keep the byte-grounded findings (bending fatigue at the transition fillet; surface stress concentrators present; material/chemistry clean); (2) state the ROOT CAUSE as a disjunction over the rivals that cannot be ruled out, NOT a single cause; (3) explicitly name the missing discriminating data — ${JSON.stringify(holes)} — as what must be retrieved to decide. Do NOT invent any of the missing data.`

// ---- the gate (mirror of underdetermination.py) ----
const isResolved = (r) => !!r.present && !!r.citation && CASE_TEXT.includes(r.citation)

// ---- run ----
const out = await agent(rivalsPrompt, { phase: 'Rivals', label: 'rivals', agentType: 'general-purpose', schema: RIVALS_SCHEMA })
const graded = out.rivals.map((r) => {
  const resolved = isResolved(r)
  return { ...r, resolved, why: resolved ? '' : (!r.present ? 'discriminating observation ABSENT from the input' : 'claimed present but citation not verbatim — treated as absent') }
})
const open = graded.filter((g) => !g.resolved)
const holes = open.map((o) => o.discriminating_observation).filter(Boolean)
const determined = open.length === 0
log(`rivals=${graded.length}; open=${open.length}; determined=${determined}`)

let corrected_answer = LEADING_ANSWER
if (!determined) {
  corrected_answer = (await agent(softenPrompt(open, holes), { phase: 'Soften', label: 'soften', agentType: 'general-purpose', schema: SOFTEN_SCHEMA })).corrected_answer
}

return {
  case_text: CASE_TEXT,
  leading_answer: LEADING_ANSWER,
  rivals: graded,
  determined,
  holes,
  corrected_answer,
}
