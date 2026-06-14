# ADJ07 — Audit Trail Schema: The IR Is the Audit Trail

## Overview

[`ADJ00`](ADJ00-adjudication-framework.md) stated that the audit trail
is materialised data, not post-hoc explanation. This spec defines
exactly what that trail looks like as a serializable artifact.

The framework's design claim about audit trails is:

> The IR is the audit trail. Every fact in the engine, every rule that
> fires, every clarification turn, every checker-pass decision, every
> proof step the engine takes — all of these chain back without gaps to
> the source bytes of the original document. The chain is data, not
> commentary.

ADJ07 is the JSON-shaped form of that chain. It is what gets persisted
when an adjudication completes (or terminates), what regulators can
audit, what downstream tooling can replay, and what makes the framework
defensible to anyone who challenges its conclusion.

## What's In the Trail

Six categories of artifact, all linked by stable ids:

1. **Documents.** The original input documents, their normalized text,
   and the byte-offset ranges (per `ADJ01`) that all spans reference.
2. **IR nodes.** Every node produced by extraction, including its kind,
   term, polarity, modality, source spans, confidence, and the
   `lowered_from` chain.
3. **Checker results.** Per-pass results: which nodes passed, which
   failed, the violation details for every failure.
4. **Clarification dialogue.** Every turn from `ADJ06`: question text,
   rung used, response source, response text, outcome.
5. **Engine artifacts.** The knowledge base used (which Facts and Rules
   from the IR fed the LP19 engine), the proof DAG returned, and, if
   probabilistic, the Boolean formula and weighted-model-counting
   result.
6. **Configuration and versioning.** Every model, prompt, tagger, and
   trigger taxonomy used, with version numbers.

These are linked by shared ids: an LP19 proof step cites an LP19 fact
id; an LP19 fact id is annotated with the ADJ IR node id it was lowered
from; an ADJ IR node id has a `source_spans` list that points into the
document. A reviewer follows the chain in either direction.

## Schema

The top-level artifact:

```text
AuditTrail := {
    adjudication_id:  AdjudicationId,           -- UUID, unique per run
    started_at:       ISO-8601,
    completed_at:     Option<ISO-8601>,         -- absent if terminated
    outcome:          AdjudicationOutcome,
    documents:        [Document],
    ir_nodes:         [IRNode],                 -- as per ADJ01
    checker_results:  [CheckerResult],
    dialogue:         [DialogueTurn],           -- as per ADJ06
    engine_artifacts: Option<EngineArtifacts>,  -- present iff engine ran
    configuration:    Configuration,
    schema_version:   "ADJ07-v1",
}
```

Each substructure is defined below.

### Documents

```text
Document := {
    id:              DocumentId,
    name:            string,                    -- user-supplied label
    received_at:     ISO-8601,
    normalized_text: string,                    -- the bytes spans reference
    normalization:   NormalizationRecord,       -- how raw -> normalized
    raw:             Optional<bytes>,           -- original bytes if retained
    appended_turns:  [AppendInfo],              -- clarification appends
}

NormalizationRecord := {
    pipeline:  string,                          -- "markdown-to-text-v1.2", etc.
    version:   string,
    options:   Map<string, Json>,
}

AppendInfo := {
    turn_id:        integer,                    -- the dialogue turn that appended
    start_offset:   integer,
    end_offset:     integer,
    appended_at:    ISO-8601,
}
```

Document storage is policy-dependent: some deployments keep raw bytes
indefinitely, others store only the normalized text. Either choice is
recorded in `NormalizationRecord.options`.

### IR Nodes

Stored exactly per [`ADJ01`](ADJ01-adjudication-ir-grammar.md). The
`lowered_from` chain is preserved end-to-end so a reviewer can replay
the refinement steps.

### Checker Results

```text
CheckerResult := {
    pass_name:    "ADJ02_coverage" | "ADJ03_polarity_modality"
                  | "ADJ04_round_trip" | "ADJ05_adversarial",
    pass_version: string,
    started_at:   ISO-8601,
    completed_at: ISO-8601,
    outcome:      Passed | Failed | Skipped,
    violations:   [Violation],
    telemetry:    Map<string, Json>,            -- per-pass extras (e.g.,
                                                   adversary-flag rates)
}

Violation := {
    node_id:       NodeId,
    pass_name:     string,                      -- redundant for ease of indexing
    kind:          ClarificationKind,           -- per ADJ06
    detail:        Json,                        -- pass-specific shape
    triggered_dialogue_turn: Option<TurnId>,
    resolved:      bool,
}
```

The `detail` schema varies by pass — coverage records uncovered spans,
polarity/modality records triggers, round-trip records NLI scores,
adversarial records the contradicting reading and judge response. The
pass-specific shapes are in each pass's spec.

### Dialogue

Per [`ADJ06`](ADJ06-clarification-dialogue.md). Stored verbatim; the
`response.source`, `response.text`, and `response.actor_id` fields are
the persisted record of who said what when.

### Engine Artifacts

```text
EngineArtifacts := {
    engine_version:  string,                    -- LP19 + sub-spec versions
    search_mode:     "FindFirst" | "EnumerateAll" | "AutoDetect",
    kb_summary:      KBSummary,
    proof_dag:       ProofDAG,                  -- as per LP19
    formula:         Option<BooleanFormula>,    -- present iff probabilistic
    wmc_result:      Option<WMCResult>,         -- present iff probabilistic
    answer:          Json,                      -- the engine's structured answer
}

KBSummary := {
    fact_count:      integer,
    rule_count:      integer,
    fact_ids:        [FactId],                  -- map to ir_nodes by metadata
    rule_ids:        [RuleId],
    all_certain:     bool,                      -- whether short-circuit fired
}

BooleanFormula := {
    encoding:  "d-DNNF" | "SDD" | "naive-enumeration",
    payload:   Json,                            -- format-specific representation
    fact_vars: Map<FactId, VarIndex>,
    rule_vars: Map<RuleId, VarIndex>,
}

WMCResult := {
    probability:   Real,
    method:        "enumeration" | "d-DNNF-eval" | ...,
    elapsed_ms:    integer,
}
```

The `proof_dag` field is verbatim what LP19 returns. The framework
adds, for each proof step that cited an LP19 fact or rule, the ADJ IR
node id those clauses were lowered from — via a separate `id_mapping`
table in the engine artifacts.

### Configuration

```text
Configuration := {
    tagger:                       VersionedComponent,
    trigger_taxonomy:             VersionedComponent,
    extractor_model:              VersionedComponent,
    renderer_model:               VersionedComponent,
    nli_model:                    VersionedComponent,
    adversary_model:              VersionedComponent,
    judge_model:                  VersionedComponent,
    rendering_function:           VersionedComponent,
    coverage_strictness:          string,
    polarity_modality_strictness: string,
    round_trip_strictness:        string,
    adversary_sample_rate:        Real in [0, 1],
    escalation_policy:            string,         -- e.g., "strict-cheap-first"
    schema_version:               string,
}

VersionedComponent := {
    name:    string,
    version: string,
    config:  Map<string, Json>,
}
```

Every model and rule corpus has a name and a version. *Reproducibility
requires this.* Running the same input through the same configuration
should produce the same output, modulo non-determinism in the LLM calls
(which is itself a configurable temperature setting recorded in
`config`).

### Outcome

```text
AdjudicationOutcome :=
    Resolved(answer:Json)               -- engine returned an answer
  | ClarificationExhausted(             -- couldn't resolve clarification
        unresolved: [Violation])
  | Aborted(reason: string)             -- system error
  | TimedOut
```

`ClarificationExhausted` is a *valid* outcome, not a failure. Per ADJ06,
"the framework's idea of 'I don't know' is the dialogue log explaining
why." Recording this as a structured outcome rather than an error lets
downstream consumers handle it explicitly.

## Storage and Persistence

A single adjudication's audit trail is a JSON document. For TSA-shape
adjudications (small inputs, few clauses), the document is small
enough to embed in a single response or commit. For clinical or
financial adjudications, the document can grow large (long notes, many
checker logs, deep dialogues).

Two persistence modes:

1. **Inline.** The trail is the response to the adjudication call.
   Suitable for synchronous, one-shot use.
2. **Append-only log.** The trail is built incrementally as the
   adjudication progresses. Each new IR node, checker result, dialogue
   turn, and engine artifact is appended to a log file or database
   table. Reads of the in-progress trail are valid at any time; the
   trail is "complete" when `completed_at` is set.

Append-only is the default for high-stakes deployments because it
permits live monitoring and partial-recovery after a crash.

## Cryptographic Integrity (Optional)

For deployments where audit-trail tampering is a real concern (medical
malpractice, financial fraud), each appendable record may be
content-addressed and chained:

```text
AppendedRecord := {
    record:      Json,
    prev_hash:   Option<Sha256Hex>,   -- the previous record's hash
    record_hash: Sha256Hex,           -- hash of this record's content
}
```

A reviewer can verify the chain is unbroken by re-hashing in order.
This is not the default — it adds overhead — but is supported.

## Replay

A complete audit trail is sufficient to **replay** the adjudication.
The replay tool (specified in `ADJ08`, a planned follow-up) reads the
trail and re-executes each step:

1. Load documents.
2. Run extraction against the same prompts and the same extractor
   model version.
3. Run each checker pass.
4. Re-execute the dialogue (this requires either replayable rung-2 / rung-3
   responses or a flag indicating the replay is "deterministic only").
5. Run the engine with the same KB.
6. Compare each artifact against the original.

Deviations between replay and original indicate a regression somewhere
in the pipeline. The replay tool reports the specific difference and
which component versions differ.

## Privacy Considerations

Audit trails contain the original document contents and every
intermediate model call. For clinical use, this means PHI. Three
controls:

1. **Storage encryption.** At-rest encryption is a deployment
   requirement; the framework does not specify the cipher but the
   `Configuration` records a `storage.encryption` entry.
2. **Selective redaction.** A redaction pass can be applied to a
   completed trail to strip identifying information while preserving
   the structural shape (node ids, polarities, decisions). The redacted
   trail is valid for downstream model training and aggregate analytics.
3. **Access control.** Each audit-trail record has an `access_policy`
   field referencing the deployment's policy registry. Tooling enforces.

None of these is the framework's job to *implement* — they are
deployment policies — but the schema must accommodate them.

## Worked Example

A complete-but-tiny TSA adjudication ends with an audit trail of the
form:

```json
{
  "adjudication_id": "01HG...",
  "started_at": "2026-05-11T08:01:23Z",
  "completed_at": "2026-05-11T08:01:31Z",
  "outcome": { "kind": "Resolved", "answer": { "verdict": "permitted_with_clarification", ... } },
  "documents": [
    { "id": "doc1", "name": "TSA-checkpoint-declaration",
      "normalized_text": "I'd like to bring a 4 oz tube of toothpaste...",
      ... }
  ],
  "ir_nodes": [ ... F1..F7 with full grammar ... ],
  "checker_results": [
    { "pass_name": "ADJ02_coverage", "outcome": "Passed", "violations": [], ... },
    { "pass_name": "ADJ03_polarity_modality", "outcome": "Passed", ... },
    { "pass_name": "ADJ04_round_trip", "outcome": "Failed",
      "violations": [ { "node_id": "F3", "kind": "RoundTripDrift", ... } ] },
    { "pass_name": "ADJ05_adversarial", "outcome": "Passed", ... }
  ],
  "dialogue": [
    { "turn_id": 1, "rung": "Rung0", "outcome": "Failed", ... },
    { "turn_id": 2, "rung": "Rung2",
      "response": { "source": "User", "text": "80 Wh each", ... },
      "outcome": "Resolved", ... }
  ],
  "engine_artifacts": {
    "engine_version": "logic-engine-0.1.0",
    "search_mode": "FindFirst",
    "kb_summary": { "all_certain": true, ... },
    "proof_dag": { ... },
    "answer": { ... }
  },
  "configuration": {
    "extractor_model": { "name": "anthropic/claude-opus-4-7", "version": "2026-05-10", ... },
    "renderer_model": { ... },
    ...
  },
  "schema_version": "ADJ07-v1"
}
```

A reviewer can follow any byte of the verdict back to a source byte of
the input, through the IR, the dialogue, and the engine, without
guesswork.

## Open Questions

1. **Schema evolution.** Audit trails persist for years; the framework
   evolves. Migration strategy for old trails (in-place, copy-on-read,
   read-only legacy) is `ADJ07a`.
2. **Compression.** Large dialogues and proof DAGs compress well. The
   schema is JSON; gzip is the obvious default. A binary format (CBOR,
   MessagePack) may be added for performance-sensitive deployments.
3. **Streaming export.** Long-running adjudications may need partial
   audit-trail export to remote storage during execution. Append-only
   storage handles this naturally; the export protocol does not.
4. **Multi-tenant deployments.** Deployments serving multiple
   organizations may share infrastructure but must isolate audit
   trails. Tenant-id is metadata; isolation is policy. Out of scope.

## Limitations

1. **The trail records what happened, not what should have happened.**
   It does not verify the *correctness* of the adjudication, only the
   *traceability*.
2. **Privacy and compliance are deployment concerns.** The schema
   accommodates redaction and encryption; the framework does not impose
   policy.
3. **Storage is not free.** A clinical adjudication's trail with
   adversarial-pass logs can be hundreds of KB. Long retention windows
   imply real storage cost.

## Status

Draft. Sufficient to implement the JSON serialization and the
append-only persistence directly. `ADJ08` (replay tool) builds on this.
`ADJ07a` (schema evolution) is a planned follow-up.
