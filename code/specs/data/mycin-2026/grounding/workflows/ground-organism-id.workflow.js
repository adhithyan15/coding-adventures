export const meta = {
  name: 'ground-organism-id',
  description: 'Spider-ground MYCIN organism-identification priors + gram-stain morphology against primary sources, with independent re-extraction (adversarial)',
  phases: [
    { title: 'Ground', detail: 'one agent per claim: WebSearch/WebFetch a primary source, extract a verbatim byte-quote + value + verdict' },
    { title: 'Verify', detail: 'independent agent re-fetches the cited URL, re-extracts, attempts to refute (byte-stability)' },
  ],
}

const CLAIMS = [
  { id: 'prior_s_pneumoniae', kind: 'prior', target: 's_pneumoniae', hint: 0.50,
    claim: 'Streptococcus pneumoniae is the most common cause of community-acquired bacterial meningitis in adults; quantify its PROPORTION of cases.' },
  { id: 'prior_n_meningitidis', kind: 'prior', target: 'n_meningitidis', hint: 0.15,
    claim: 'Proportion of community-acquired bacterial meningitis caused by Neisseria meningitidis.' },
  { id: 'prior_listeria', kind: 'prior', target: 'listeria', hint: 0.05,
    claim: 'Proportion of bacterial meningitis caused by Listeria monocytogenes (esp. age >50 / immunocompromised).' },
  { id: 'prior_h_influenzae', kind: 'prior', target: 'h_influenzae', hint: 0.05,
    claim: 'Proportion of bacterial meningitis caused by Haemophilus influenzae in the post-Hib-vaccine era.' },
  { id: 'prior_gram_negative_bacilli', kind: 'prior', target: 'gram_negative_bacilli', hint: 0.03,
    claim: 'Proportion of bacterial meningitis caused by aerobic gram-negative bacilli (E. coli/Klebsiella).' },
  { id: 'prior_group_b_strep', kind: 'prior', target: 'group_b_strep', hint: 0.02,
    claim: 'Proportion of bacterial meningitis caused by group B Streptococcus (Streptococcus agalactiae).' },
  { id: 'prior_s_aureus', kind: 'prior', target: 's_aureus', hint: 0.01,
    claim: 'Proportion of bacterial meningitis caused by Staphylococcus aureus (healthcare-associated).' },
  { id: 'morph_gpd_pneumococcus', kind: 'morphology', target: 's_pneumoniae',
    claim: 'On CSF Gram stain, lancet-shaped gram-positive diplococci indicate Streptococcus pneumoniae.' },
  { id: 'morph_gnd_meningococcus', kind: 'morphology', target: 'n_meningitidis',
    claim: 'On CSF Gram stain, gram-negative diplococci indicate Neisseria meningitidis.' },
  { id: 'morph_gpb_listeria', kind: 'morphology', target: 'listeria',
    claim: 'On CSF Gram stain, gram-positive bacilli/coccobacilli indicate Listeria monocytogenes.' },
  { id: 'morph_gncocco_hflu', kind: 'morphology', target: 'h_influenzae',
    claim: 'On CSF Gram stain, pleomorphic gram-negative coccobacilli indicate Haemophilus influenzae.' },
  { id: 'morph_gnb_enteric', kind: 'morphology', target: 'gram_negative_bacilli',
    claim: 'On CSF Gram stain, gram-negative bacilli (rods) indicate enteric gram-negative bacilli.' },
  { id: 'morph_gpcc_saureus', kind: 'morphology', target: 's_aureus',
    claim: 'On CSF Gram stain, gram-positive cocci in clusters indicate staphylococci (S. aureus).' },
]

const GROUND_SCHEMA = {
  type: 'object',
  required: ['id', 'resolved_url', 'source_title', 'byte_quote', 'value_found', 'direction_correct', 'verdict', 'discards', 'note'],
  properties: {
    id: { type: 'string' },
    resolved_url: { type: 'string' },
    source_title: { type: 'string' },
    byte_quote: { type: 'string', description: 'VERBATIM sentence(s) copied from the fetched page — never paraphrased or fabricated' },
    value_found: { type: 'string' },
    direction_correct: { type: 'boolean' },
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
    `Ground this clinical claim against a PRIMARY source. Use WebSearch then WebFetch to actually READ a primary source (a peer-reviewed study, an IDSA guideline, or an authoritative clinical-microbiology reference — NOT a secondary blog). ` +
    `CLAIM [${c.id}]: ${c.claim}` +
    (c.hint != null ? ` (a rough prior estimate is ~${c.hint}, but DERIVE the real value from the source — do not trust the hint).` : '') +
    ` Return the resolved_url you fetched, source_title, a VERBATIM byte_quote from that page (never fabricate — if you cannot fetch a page with a supporting quote, set verdict "ungrounded"), value_found, direction_correct, a verdict, the sources/spans you DISCARDED (with why), and a note justifying whether the quote ENTAILS the claim or you made a LEAP.`,
    { schema: GROUND_SCHEMA, label: `ground:${c.id}`, phase: 'Ground' }
  ).then((g) => g ? { claim: c, grounded: g } : null),
  (r) => {
    if (!r || !r.grounded) return null
    const g = r.grounded
    if (g.verdict === 'ungrounded') return { ...r, verify: { id: g.id, byte_stable: false, reextracted_value: '', refute_attempt: 'n/a', final_verdict: 'ungrounded' } }
    return agent(
      `Independently VERIFY a grounding. WebFetch this exact URL and confirm the byte_quote really appears there and supports the claim. ` +
      `CLAIM [${g.id}]: ${r.claim.claim}\nURL: ${g.resolved_url}\nbyte_quote to confirm (verbatim): "${g.byte_quote}"\nclaimed value: ${g.value_found}\n` +
      `Set byte_stable=true ONLY if the quote appears verbatim on the page you fetch. Re-extract the value yourself. Then make the STRONGEST refutation you can. Give your final_verdict.`,
      { schema: VERIFY_SCHEMA, label: `verify:${g.id}`, phase: 'Verify' }
    ).then((v) => ({ ...r, verify: v }))
  }
)

return records.filter(Boolean).map((r) => ({
  id: r.claim.id, kind: r.claim.kind, target: r.claim.target, claim: r.claim.claim,
  authored_hint: r.claim.hint ?? null,
  grounded: r.grounded,
  verify: r.verify,
  spider_status: (r.verify && r.verify.byte_stable && r.grounded.verdict === 'grounded' && r.verify.final_verdict === 'grounded')
    ? 'grounded'
    : (r.grounded.verdict === 'refuted' || (r.verify && r.verify.final_verdict === 'refuted')) ? 'refuted'
    : (r.grounded.verdict === 'ungrounded') ? 'ungrounded'
    : 'direction_only',
}))