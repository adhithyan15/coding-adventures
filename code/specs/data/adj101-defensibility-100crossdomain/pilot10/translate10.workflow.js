export const meta = {
  name: 'adj101-pilot10-translate',
  description: 'Run 10 computational items through the framework EXTRACTION+PROGRAM-EMISSION stage: each agent translates the messy source into typed/provenanced facts and emits a Python program (SymPy/RDKit) — it does NOT compute the answer itself',
  phases: [{ title: 'Translate', detail: 'one translator agent per item' }],
}

// args = [{id, source, question, quantity_spans}, ...]  OR  {model, items}
const parsed = Array.isArray(args) ? { items: args } : (typeof args === 'string' ? JSON.parse(args) : args)
const items = parsed.items || parsed
const MODEL = parsed.model // undefined -> inherit session model (Opus); 'haiku' for the weak-model arm

const FACT = {
  type: 'object',
  properties: {
    magnitude: { description: 'the typed value: a number, or a datum string like a SMILES/equation' },
    unit: { type: 'string' },
    polarity: { type: 'string', enum: ['affirmed', 'denied'] },
    type: { type: 'string', enum: ['stated', 'inferred'] },
    span: { type: 'string', description: 'verbatim source bytes (REQUIRED if stated)' },
    basis_span: { type: 'string', description: 'verbatim source bytes (REQUIRED if inferred)' },
    entailment: { type: 'string', enum: ['ENTAILED', 'LEAP'] },
  },
  required: ['type'],
}

const SCHEMA = {
  type: 'object',
  additionalProperties: false,
  properties: {
    facts: { type: 'object', additionalProperties: FACT,
             description: 'named typed facts extracted from the source, each with provenance' },
    discarded: { type: 'array', items: { type: 'object', additionalProperties: false,
                 properties: { span: { type: 'string' }, reason: { type: 'string' } },
                 required: ['span', 'reason'] } },
    program: { type: 'string',
               description: 'python that sets float RESULT using only facts[...] values + tools' },
  },
  required: ['facts', 'discarded', 'program'],
}

function contract(it) {
  return (
    `You are the EXTRACTION + PROGRAM-EMISSION stage of a byte-provenance reasoning framework. You do ` +
    `NOT solve the problem in your head (LLMs are unreliable at multi-step math/chemistry). You (1) ` +
    `translate the messy input into typed, provenance-tagged facts, and (2) emit a PYTHON PROGRAM that ` +
    `computes the answer using specialized tools. A deterministic executor runs your program.\n\n` +
    `SOURCE: ${JSON.stringify(it.source)}\nQUESTION: ${JSON.stringify(it.question)}\n\n` +
    `RULES (the executor enforces these):\n` +
    `1. Every quantity-bearing phrase in SOURCE must appear as a fact's span/basis_span OR in ` +
    `discarded(reason). Nothing silently dropped. The phrases to account for: ` +
    `${JSON.stringify(it.quantity_spans)}.\n` +
    `2. The program runs with a variable \`facts\` already set to your facts dict (JSON). It must set a ` +
    `float \`RESULT\`. It may import rdkit, sympy, numpy, scipy (all installed).\n` +
    `3. The program may use ONLY values pulled from facts[...] (e.g. facts['x']['magnitude']) and values ` +
    `COMPUTED by tools. It must NOT hard-code domain numbers (molecular weights, constants, etc.) as ` +
    `literals — get molecular weights from RDKit, etc. Only trivial structural constants (0,1,2,pi via ` +
    `the library) may appear. A bare quantity from the problem must come through facts, never a literal.\n` +
    `4. Mark facts stated(span) vs inferred(basis_span+entailment). A derived fact (e.g. a stoichiometric ` +
    `ratio) is inferred; justify it from the bytes.\n` +
    `5. Do NOT compute the answer anywhere yourself — the program does it.\n\n` +
    `Return the structured emission.`
  )
}

phase('Translate')
log(`Translating ${items.length} computational items into provenanced facts + emitted programs.`)
const emissions = await parallel(
  items.map((it) => () =>
    agent(contract(it), { label: `translate:${it.id}`, phase: 'Translate', schema: SCHEMA, ...(MODEL ? { model: MODEL } : {}) })
      .then((e) => (e ? { id: it.id, ...e } : { id: it.id, _error: true })))
)
return emissions
