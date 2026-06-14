// ground-host-factors.workflow.js — spider-ground the organism-id HOST-FACTOR LRs.
//
// G2: retire the 14 authored host-factor contributes in organism-id.adj (age band,
// immune status, exposures, device/neurosurgery, rash → organism). Same adversarial
// pattern as ground-organism-id: ground each ASSOCIATION + DIRECTION against a primary
// source (IDSA/Tunkel, van de Beek, StatPearls, CDC, Rouphael), then an independent
// agent re-fetches and tries to refute (byte-stability). The LR MAGNITUDE stays a
// structural risk-multiplier (like the morphology LRs); the spider grounds that the
// host→organism association is real and points the right way, with a verbatim quote.
//
// Output → grounding/host-factor-grounding.json, consumed by organism_id_ground.py.

export const meta = {
  name: 'ground-host-factors',
  description: 'Spider-ground MYCIN organism-id host-factor likelihood ratios (age/immune/exposure/device → organism) against primary sources, with independent adversarial re-extraction',
  phases: [
    { title: 'Ground', detail: 'one agent per host-factor association: WebSearch/WebFetch a primary source, verbatim byte-quote + direction + verdict' },
    { title: 'Verify', detail: 'independent agent re-fetches the cited URL, re-extracts, attempts to refute (byte-stability)' },
  ],
}

const CLAIMS = [
  { id: 'host_neonate_gbs', target: 'group_b_strep', evidence: 'age_band(neonate)',
    claim: 'In neonates (first ~1–3 months of life), group B Streptococcus (Streptococcus agalactiae) is a leading cause of bacterial meningitis.' },
  { id: 'host_neonate_gnb', target: 'gram_negative_bacilli', evidence: 'age_band(neonate)',
    claim: 'In neonates, aerobic gram-negative bacilli (especially Escherichia coli) are a leading cause of bacterial meningitis.' },
  { id: 'host_neonate_listeria', target: 'listeria', evidence: 'age_band(neonate)',
    claim: 'Neonates are an at-risk group for Listeria monocytogenes meningitis.' },
  { id: 'host_olderadult_listeria', target: 'listeria', evidence: 'age_band(older_adult)',
    claim: 'Adults older than ~50 years are at elevated risk of Listeria monocytogenes meningitis.' },
  { id: 'host_olderadult_gnb', target: 'gram_negative_bacilli', evidence: 'age_band(older_adult)',
    claim: 'Older adults are at elevated risk of aerobic gram-negative bacillary meningitis.' },
  { id: 'host_infantchild_nmen', target: 'n_meningitidis', evidence: 'age_band(infant_child)',
    claim: 'In children and adolescents, Neisseria meningitidis is a leading cause of bacterial meningitis.' },
  { id: 'host_infantchild_hflu', target: 'h_influenzae', evidence: 'age_band(infant_child)',
    claim: 'In young children, Haemophilus influenzae (type b, esp. if unvaccinated) is a cause of bacterial meningitis.' },
  { id: 'host_immuno_listeria', target: 'listeria', evidence: 'immunocompromised(present)',
    claim: 'Immunocompromised patients (impaired cell-mediated immunity) are at elevated risk of Listeria monocytogenes meningitis.' },
  { id: 'host_immuno_gnb', target: 'gram_negative_bacilli', evidence: 'immunocompromised(present)',
    claim: 'Immunocompromised patients are at elevated risk of aerobic gram-negative bacillary meningitis.' },
  { id: 'host_listeriaexp_listeria', target: 'listeria', evidence: 'listeria_exposure(present)',
    claim: 'Consumption of contaminated unpasteurized dairy products or deli meats is a recognized risk factor / exposure for Listeria monocytogenes infection.' },
  { id: 'host_neurosurg_saureus', target: 's_aureus', evidence: 'recent_neurosurgery_or_shunt(present)',
    claim: 'After neurosurgery or with a CSF shunt/device, staphylococci including Staphylococcus aureus are a leading cause of healthcare-associated meningitis/ventriculitis.' },
  { id: 'host_neurosurg_gnb', target: 'gram_negative_bacilli', evidence: 'recent_neurosurgery_or_shunt(present)',
    claim: 'After neurosurgery or with a CSF device, aerobic gram-negative bacilli are a common cause of healthcare-associated meningitis.' },
  { id: 'host_crowding_nmen', target: 'n_meningitidis', evidence: 'crowding_exposure(present)',
    claim: 'Crowded living conditions (dormitories, military barracks, the Hajj pilgrimage) increase the risk of meningococcal (Neisseria meningitidis) disease.' },
  { id: 'host_petechial_nmen', target: 'n_meningitidis', evidence: 'petechial_rash(present)',
    claim: 'A petechial or purpuric rash accompanying meningitis suggests meningococcal (Neisseria meningitidis) infection.' },
]

const GROUND_SCHEMA = {
  type: 'object',
  required: ['id', 'resolved_url', 'source_title', 'byte_quote', 'value_found', 'direction_correct', 'verdict', 'discards', 'note'],
  properties: {
    id: { type: 'string' },
    resolved_url: { type: 'string' },
    source_title: { type: 'string' },
    byte_quote: { type: 'string', description: 'VERBATIM sentence(s) copied from the fetched page — never paraphrased or fabricated' },
    value_found: { type: 'string', description: 'any odds ratio / relative risk / proportion the source gives for this association, else a short phrase' },
    direction_correct: { type: 'boolean', description: 'does the source support this host factor RAISING the probability of this organism?' },
    verdict: { type: 'string', enum: ['grounded', 'direction_only', 'refuted', 'ungrounded'] },
    discards: { type: 'array', items: { type: 'string' } },
    note: { type: 'string', description: 'ENTAILED (quote forces it) vs LEAP (inferred); explain' },
  },
}
const VERIFY_SCHEMA = {
  type: 'object',
  required: ['id', 'byte_stable', 'reextracted_value', 'refute_attempt', 'final_verdict'],
  properties: {
    id: { type: 'string' },
    byte_stable: { type: 'boolean' },
    reextracted_value: { type: 'string' },
    refute_attempt: { type: 'string' },
    final_verdict: { type: 'string', enum: ['grounded', 'direction_only', 'refuted', 'ungrounded'] },
  },
}

const records = await pipeline(
  CLAIMS,
  (c) => agent(
    `Ground this clinical RISK ASSOCIATION against a PRIMARY source. Use WebSearch then WebFetch to actually READ a primary source (a peer-reviewed study, the IDSA/Tunkel bacterial-meningitis guideline, van de Beek/Brouwer, CDC, or an authoritative clinical-microbiology reference — NOT a secondary blog). ` +
    `CLAIM [${c.id}]: ${c.claim} ` +
    `This is a host-factor → organism association: confirm the source supports that this host factor RAISES the likelihood of this organism (direction), and capture any odds ratio / relative risk / proportion it gives. ` +
    `Return the resolved_url you fetched, source_title, a VERBATIM byte_quote from that page (never fabricate — if you cannot fetch a page with a supporting quote, set verdict "ungrounded"), value_found, direction_correct, a verdict, the sources/spans you DISCARDED (with why), and a note justifying whether the quote ENTAILS the association or you made a LEAP.`,
    { schema: GROUND_SCHEMA, label: `ground:${c.id}`, phase: 'Ground' }
  ).then((g) => g ? { claim: c, grounded: g } : null),
  (r) => {
    if (!r || !r.grounded) return null
    const g = r.grounded
    if (g.verdict === 'ungrounded') return { ...r, verify: { id: g.id, byte_stable: false, reextracted_value: '', refute_attempt: 'n/a', final_verdict: 'ungrounded' } }
    return agent(
      `Independently VERIFY a grounding. WebFetch this exact URL and confirm the byte_quote really appears there and supports the host-factor→organism association. ` +
      `CLAIM [${g.id}]: ${r.claim.claim}\nURL: ${g.resolved_url}\nbyte_quote to confirm (verbatim): "${g.byte_quote}"\nclaimed value: ${g.value_found}\n` +
      `Set byte_stable=true ONLY if the quote appears verbatim on the page you fetch. Re-extract any value yourself. Then make the STRONGEST refutation you can (is the direction actually supported?). Give your final_verdict.`,
      { schema: VERIFY_SCHEMA, label: `verify:${g.id}`, phase: 'Verify' }
    ).then((v) => ({ ...r, verify: v }))
  }
)

return records.filter(Boolean).map((r) => ({
  id: r.claim.id, kind: 'host', target: r.claim.target, evidence: r.claim.evidence, claim: r.claim.claim,
  grounded: r.grounded,
  verify: r.verify,
  spider_status: (r.verify && r.verify.byte_stable && r.grounded.verdict === 'grounded' && r.verify.final_verdict === 'grounded')
    ? 'grounded'
    : (r.grounded.verdict === 'refuted' || (r.verify && r.verify.final_verdict === 'refuted')) ? 'refuted'
    : (r.grounded.verdict === 'ungrounded') ? 'ungrounded'
    : 'direction_only',
}))
