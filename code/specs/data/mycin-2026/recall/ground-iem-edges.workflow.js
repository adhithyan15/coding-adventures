// ground-iem-edges.workflow.js — ground the inborn-errors-of-metabolism edges (REL-4).
//
// The IEM knowledge graph (recall/iem-edges.adj) was authored "trust consensus" from
// standard biochemistry. REL-4 retires that authored-debt through the cold path: one
// agent grounds each DISEASE→{enzyme,substrate,inheritance} edge against a PRIMARY
// source (OMIM, a peer-reviewed / authoritative biochemistry reference), capturing a
// VERBATIM byte-quote; an independent agent re-fetches the cited URL and tries to
// refute. Same per-claim ground→verify pattern as the bacteremia source-LRs (G5b).
// Output → recall/iem-edge-grounding.json, consumed by recall/iem_edge_ground.py,
// which regenerates iem-edges.adj — ACCEPTed edges flip to `trust authoritative` with
// the grounded byte-quote; the rest stay consensus + FLAG.
//
// Run it (opt-in, costs tokens + network): this is the expensive half of the loop;
// the gate + harness are the cheap, reusable on-ramp built first.

export const meta = {
  name: 'ground-iem-edges',
  description: 'Spider-ground inborn-errors-of-metabolism edges (disease→deficient enzyme / accumulated substrate / inheritance) against OMIM / primary biochemistry sources, with independent adversarial re-extraction',
  phases: [
    { title: 'Ground', detail: 'one agent per edge: WebSearch/WebFetch a primary source (OMIM/biochem ref), verbatim byte-quote + verdict' },
    { title: 'Verify', detail: 'independent agent re-fetches the cited URL, re-extracts, attempts to refute (byte-stability)' },
  ],
}

// id = "<relation>__<subject>" — must match iem_edge_ground.py's edge ids.
const EDGES = [
  { id: 'deficient_in__tay_sachs', rel: 'deficient_in', subj: 'tay_sachs', obj: 'hexosaminidase_a',
    claim: 'Tay-Sachs disease results from deficiency of the enzyme hexosaminidase A (HEXA).' },
  { id: 'accumulates__tay_sachs', rel: 'accumulates', subj: 'tay_sachs', obj: 'gm2_ganglioside',
    claim: 'GM2 ganglioside accumulates (in neurons) in Tay-Sachs disease.' },
  { id: 'inherited_as__tay_sachs', rel: 'inherited_as', subj: 'tay_sachs', obj: 'autosomal_recessive',
    claim: 'Tay-Sachs disease is inherited in an autosomal recessive manner.' },

  { id: 'deficient_in__gaucher', rel: 'deficient_in', subj: 'gaucher', obj: 'glucocerebrosidase',
    claim: 'Gaucher disease is caused by deficiency of glucocerebrosidase (acid beta-glucosidase / GBA).' },
  { id: 'accumulates__gaucher', rel: 'accumulates', subj: 'gaucher', obj: 'glucocerebroside',
    claim: 'Glucocerebroside (glucosylceramide) accumulates in macrophages in Gaucher disease.' },
  { id: 'inherited_as__gaucher', rel: 'inherited_as', subj: 'gaucher', obj: 'autosomal_recessive',
    claim: 'Gaucher disease is inherited in an autosomal recessive manner.' },

  { id: 'deficient_in__phenylketonuria', rel: 'deficient_in', subj: 'phenylketonuria', obj: 'phenylalanine_hydroxylase',
    claim: 'Phenylketonuria results from deficiency of phenylalanine hydroxylase (PAH).' },
  { id: 'accumulates__phenylketonuria', rel: 'accumulates', subj: 'phenylketonuria', obj: 'phenylalanine',
    claim: 'Phenylalanine accumulates in phenylketonuria.' },
  { id: 'inherited_as__phenylketonuria', rel: 'inherited_as', subj: 'phenylketonuria', obj: 'autosomal_recessive',
    claim: 'Phenylketonuria is inherited in an autosomal recessive manner.' },

  { id: 'deficient_in__pompe', rel: 'deficient_in', subj: 'pompe', obj: 'acid_alpha_glucosidase',
    claim: 'Pompe disease (GSD II) is caused by deficiency of acid alpha-glucosidase (acid maltase / GAA).' },
  { id: 'accumulates__pompe', rel: 'accumulates', subj: 'pompe', obj: 'lysosomal_glycogen',
    claim: 'Glycogen accumulates in lysosomes in Pompe disease.' },
  { id: 'inherited_as__pompe', rel: 'inherited_as', subj: 'pompe', obj: 'autosomal_recessive',
    claim: 'Pompe disease is inherited in an autosomal recessive manner.' },

  { id: 'deficient_in__lesch_nyhan', rel: 'deficient_in', subj: 'lesch_nyhan', obj: 'hgprt',
    claim: 'Lesch-Nyhan syndrome results from deficiency of hypoxanthine-guanine phosphoribosyltransferase (HGPRT/HPRT).' },
  { id: 'accumulates__lesch_nyhan', rel: 'accumulates', subj: 'lesch_nyhan', obj: 'uric_acid',
    claim: 'Uric acid accumulates (hyperuricemia) in Lesch-Nyhan syndrome.' },
  { id: 'inherited_as__lesch_nyhan', rel: 'inherited_as', subj: 'lesch_nyhan', obj: 'x_linked_recessive',
    claim: 'Lesch-Nyhan syndrome is inherited in an X-linked recessive manner.' },

  { id: 'deficient_in__von_gierke', rel: 'deficient_in', subj: 'von_gierke', obj: 'glucose_6_phosphatase',
    claim: 'Von Gierke disease (GSD type I) results from deficiency of glucose-6-phosphatase.' },
  { id: 'accumulates__von_gierke', rel: 'accumulates', subj: 'von_gierke', obj: 'glycogen',
    claim: 'Glycogen accumulates in liver and kidney in von Gierke disease.' },
  { id: 'inherited_as__von_gierke', rel: 'inherited_as', subj: 'von_gierke', obj: 'autosomal_recessive',
    claim: 'Von Gierke disease is inherited in an autosomal recessive manner.' },

  // REL-6 expansion — a second high-yield disease set (must match iem_edge_ground.py GROUPS).
  { id: 'deficient_in__fabry', rel: 'deficient_in', subj: 'fabry', obj: 'alpha_galactosidase_a',
    claim: 'Fabry disease results from deficiency of the enzyme alpha-galactosidase A.' },
  { id: 'accumulates__fabry', rel: 'accumulates', subj: 'fabry', obj: 'globotriaosylceramide',
    claim: 'Globotriaosylceramide (Gb3 / ceramide trihexoside) accumulates in Fabry disease.' },
  { id: 'inherited_as__fabry', rel: 'inherited_as', subj: 'fabry', obj: 'x_linked_recessive',
    claim: 'Fabry disease is inherited in an X-linked recessive manner.' },

  { id: 'deficient_in__niemann_pick', rel: 'deficient_in', subj: 'niemann_pick', obj: 'acid_sphingomyelinase',
    claim: 'Niemann-Pick disease types A and B result from deficiency of acid sphingomyelinase (ASM / SMPD1).' },
  { id: 'accumulates__niemann_pick', rel: 'accumulates', subj: 'niemann_pick', obj: 'sphingomyelin',
    claim: 'Sphingomyelin accumulates in Niemann-Pick disease (types A/B).' },
  { id: 'inherited_as__niemann_pick', rel: 'inherited_as', subj: 'niemann_pick', obj: 'autosomal_recessive',
    claim: 'Niemann-Pick disease is inherited in an autosomal recessive manner.' },

  { id: 'deficient_in__krabbe', rel: 'deficient_in', subj: 'krabbe', obj: 'galactocerebrosidase',
    claim: 'Krabbe disease results from deficiency of galactocerebrosidase (galactosylceramidase / GALC).' },
  { id: 'accumulates__krabbe', rel: 'accumulates', subj: 'krabbe', obj: 'psychosine',
    claim: 'Psychosine (galactosylsphingosine) accumulates in Krabbe disease.' },
  { id: 'inherited_as__krabbe', rel: 'inherited_as', subj: 'krabbe', obj: 'autosomal_recessive',
    claim: 'Krabbe disease is inherited in an autosomal recessive manner.' },

  { id: 'deficient_in__hurler', rel: 'deficient_in', subj: 'hurler', obj: 'alpha_l_iduronidase',
    claim: 'Hurler syndrome (mucopolysaccharidosis type I) results from deficiency of alpha-L-iduronidase (IDUA).' },
  { id: 'accumulates__hurler', rel: 'accumulates', subj: 'hurler', obj: 'glycosaminoglycans',
    claim: 'Glycosaminoglycans (dermatan and heparan sulfate) accumulate in Hurler syndrome.' },
  { id: 'inherited_as__hurler', rel: 'inherited_as', subj: 'hurler', obj: 'autosomal_recessive',
    claim: 'Hurler syndrome is inherited in an autosomal recessive manner.' },

  { id: 'deficient_in__msud', rel: 'deficient_in', subj: 'msud', obj: 'branched_chain_ketoacid_dehydrogenase',
    claim: 'Maple syrup urine disease results from deficiency of branched-chain alpha-ketoacid dehydrogenase (BCKDH).' },
  { id: 'accumulates__msud', rel: 'accumulates', subj: 'msud', obj: 'branched_chain_amino_acids',
    claim: 'Branched-chain amino acids (leucine, isoleucine, valine) accumulate in maple syrup urine disease.' },
  { id: 'inherited_as__msud', rel: 'inherited_as', subj: 'msud', obj: 'autosomal_recessive',
    claim: 'Maple syrup urine disease is inherited in an autosomal recessive manner.' },

  { id: 'deficient_in__galactosemia', rel: 'deficient_in', subj: 'galactosemia', obj: 'galactose_1_phosphate_uridyltransferase',
    claim: 'Classic galactosemia results from deficiency of galactose-1-phosphate uridyltransferase (GALT).' },
  { id: 'accumulates__galactosemia', rel: 'accumulates', subj: 'galactosemia', obj: 'galactose_1_phosphate',
    claim: 'Galactose-1-phosphate accumulates in classic galactosemia.' },
  { id: 'inherited_as__galactosemia', rel: 'inherited_as', subj: 'galactosemia', obj: 'autosomal_recessive',
    claim: 'Classic galactosemia is inherited in an autosomal recessive manner.' },
]

const GROUND_SCHEMA = {
  type: 'object',
  required: ['id', 'resolved_url', 'source_title', 'byte_quote', 'direction_correct', 'verdict', 'discards', 'note'],
  properties: {
    id: { type: 'string' },
    resolved_url: { type: 'string' },
    source_title: { type: 'string' },
    byte_quote: { type: 'string', description: 'VERBATIM sentence(s) copied from the fetched page — never paraphrased or fabricated' },
    direction_correct: { type: 'boolean', description: 'does the source state this edge (subject → object)?' },
    verdict: { type: 'string', enum: ['grounded', 'direction_only', 'refuted', 'ungrounded'] },
    discards: { type: 'array', items: { type: 'string' } },
    note: { type: 'string', description: 'ENTAILED (quote forces it) vs LEAP (inferred); explain' },
  },
}
const VERIFY_SCHEMA = {
  type: 'object',
  required: ['id', 'byte_stable', 'refute_attempt', 'final_verdict'],
  properties: {
    id: { type: 'string' },
    byte_stable: { type: 'boolean' },
    refute_attempt: { type: 'string' },
    final_verdict: { type: 'string', enum: ['grounded', 'direction_only', 'refuted', 'ungrounded'] },
  },
}

const records = await pipeline(
  EDGES,
  (e) => agent(
    `Ground this inborn-error-of-metabolism FACT against a PRIMARY source. Use WebSearch then WebFetch to actually READ a primary/authoritative source — OMIM (omim.org), a peer-reviewed reference, or an authoritative clinical-biochemistry text — NOT a secondary blog or a question bank. ` +
    `FACT [${e.id}]: ${e.claim} Confirm the source states this exact relation (${e.subj} → ${e.obj}). ` +
    `Return the resolved_url you fetched, source_title, a VERBATIM byte_quote from that page (never fabricate — if you cannot fetch a page with a supporting quote, set verdict "ungrounded"), direction_correct, a verdict, the sources/spans you DISCARDED (with why), and a note on whether the quote ENTAILS the fact or you made a LEAP.`,
    { schema: GROUND_SCHEMA, label: `ground:${e.id}`, phase: 'Ground' }
  ).then((g) => g ? { edge: e, grounded: g } : null),
  (r) => {
    if (!r || !r.grounded) return null
    const g = r.grounded
    if (g.verdict === 'ungrounded') return { ...r, verify: { id: g.id, byte_stable: false, refute_attempt: 'n/a', final_verdict: 'ungrounded' } }
    return agent(
      `Independently VERIFY a grounding. WebFetch this exact URL and confirm the byte_quote really appears there and states the edge. ` +
      `FACT [${g.id}]: ${r.edge.claim}\nURL: ${g.resolved_url}\nbyte_quote to confirm (verbatim): "${g.byte_quote}"\n` +
      `Set byte_stable=true ONLY if the quote appears verbatim on the page you fetch. Then make the STRONGEST refutation (is the relation actually stated?). Give your final_verdict.`,
      { schema: VERIFY_SCHEMA, label: `verify:${g.id}`, phase: 'Verify' }
    ).then((v) => ({ ...r, verify: v }))
  }
)

return {
  kind: 'iem-edge-grounding',
  records: records.filter(Boolean).map((r) => ({
    id: r.edge.id, relation: r.edge.rel, subject: r.edge.subj, object: r.edge.obj, claim: r.edge.claim,
    grounded: r.grounded,
    verify: r.verify,
    spider_status: (r.verify && r.verify.byte_stable && r.grounded.verdict === 'grounded' && r.verify.final_verdict === 'grounded')
      ? 'grounded'
      : (r.grounded.verdict === 'refuted' || (r.verify && r.verify.final_verdict === 'refuted')) ? 'refuted'
      : (r.grounded.verdict === 'ungrounded') ? 'ungrounded'
      : 'direction_only',
  })),
}
