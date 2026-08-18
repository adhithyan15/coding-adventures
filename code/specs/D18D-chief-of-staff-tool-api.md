# D18D - Chief of Staff Tool API

## Overview

D18 already defines the low-level `host.*` boundary between a caged agent runtime and
its supervising host process. That boundary is necessary, but it is too low-level to
be the primary abstraction for a session kernel, a job runner, or a model gateway.

Chief of Staff also needs a **model-facing, provider-neutral, repository-owned tool
contract**.

This spec defines that contract.

The key distinction is:

- `host.*` is the **process-boundary API**
- the Tool API is the **kernel-facing and model-facing capability surface**

That separation lets the repository:

1. define tools once in a canonical schema
2. translate them into Anthropic/OpenAI/local-model tool formats at the edge
3. apply the same validation, policy, audit, approval, and persistence rules to every tool call
4. implement built-in tools over stores, jobs, vault, channels, and host services instead of coupling providers directly to those subsystems

---

## Design Principles

1. **The repository owns the tool schema.**
2. **Models never see provider-specific business logic.**
3. **Every tool call follows the same lifecycle.**
4. **Permission and approval checks are centralized, not ad hoc.**
5. **Tools compose over repository services, not raw backends.**
6. **Streaming and cancellation are first-class.**
7. **Built-ins should prefer durable stores over ambient filesystem state.**

---

## Where It Fits

```text
User / Job / Agent / SessionKernel
    |
    v
Tool Runtime (D18D)
  - registry
  - validation
  - invocation pipeline
  - event stream
  - audit + policy hooks
    |
    +--> built-in tools
    |      +--> ContextStore (D18A)
    |      +--> ArtifactStore (D18A)
    |      +--> SkillStore (D18A)
    |      +--> MemoryStore (D18A)
    |      +--> Job Runtime (D18C)
    |      +--> Vault / Channels / Host APIs (D18)
    |
    +--> ModelGateway / ModelDriver (future D18B)
    |      +--> translates ToolDefinition into provider-specific schemas
    |
    +--> Policy / Capability / Tier enforcement
           +--> Capability Cage (D21)
```

**Depends on:** D18 Chief of Staff, D18A Stores, D18C Job Framework, D21 Capability Cage.

**Used by:** future session-kernel work, model-gateway work, automation/jobs, agent runtimes,
CLI/desktop/mobile shells, and all built-in model-facing tools.

---

## Goals

### Primary goals

1. One canonical `ToolDefinition` format for the whole repository
2. One canonical invocation/result/event lifecycle
3. Centralized approval, tier, and policy checks
4. Clean translation into provider-specific tool/function schemas
5. A built-in catalog that works equally in interactive sessions, jobs, and sub-agents

### Non-goals

- Replacing `host.*`
- Replacing the Capability Cage taxonomy
- Embedding provider-specific schema quirks into the repository contract
- Giving every tool unrestricted access to raw OS resources

---

## Layers

```text
Tool author / built-in implementation
    |
    v
ToolDefinition + ToolHandler
    |
    v
ToolRuntime
  - register
  - resolve
  - validate
  - invoke
  - stream events
    |
    +--> ToolPolicyEngine
    +--> ToolAuditSink
    +--> ToolLocks / concurrency guards
    +--> Artifact persistence hooks
    +--> Approval / privilege tier hooks
    |
    v
Execution context
  - stores
  - host facade
  - model gateway
  - audit
  - policy
  - cancellation
```

---

## Tool Identity

Tool identifiers must be stable, globally unique within the repository, and readable in
logs.

### Naming rules

- dotted namespace form: `family.verb` or `family.object.verb`
- lowercase ASCII
- words separated with dots, not slashes
- examples:
  - `context.open_session`
  - `artifact.write_revision`
  - `job.run_now`
  - `vault.request_lease`
  - `agent.spawn`

### Stability rule

Once a tool id is published in a released runtime, that id must remain stable. Semantic
changes that break callers require either:

- additive schema evolution, or
- a new tool id

---

## ToolDefinition

`ToolDefinition` is the canonical repository-owned shape.

```text
ToolDefinition
|-- tool_id
|-- display_name
|-- description
|-- input_schema
|-- output_schema?
|-- side_effects
|-- idempotency
|-- concurrency
|-- streaming
|-- required_tier
|-- required_capabilities[]
|-- preferred_lock_scope?
|-- timeout_seconds?
|-- tags[]
|-- stability            experimental | stable | deprecated
```

### Field meanings

#### `tool_id`

Stable repository identifier. See naming rules above.

#### `display_name`

Human-facing label for UIs, logs, and traces.

#### `description`

A short provider-neutral explanation of what the tool does. This text should be suitable
for model prompts and for human inspection.

#### `input_schema`

Repository-owned JSON-schema-like shape describing the arguments. The schema must be
strict enough that the runtime can reject malformed calls before handler execution.

#### `output_schema`

Optional schema describing a successful terminal result. Streaming tools may also emit
intermediate events before the terminal result.

#### `side_effects`

```text
none      read-only, no persistent or external mutation
read      reads protected resources but does not mutate them
write     mutates repository-managed state
external  affects the outside world (network, process, device, user account, etc.)
```

This field drives approvals, audit emphasis, and planning heuristics.

#### `idempotency`

```text
always       repeating the same call is safe
conditional  safe only under caller-controlled keys or revisions
never        repeating may produce a new side effect each time
```

#### `concurrency`

```text
safe         may run in parallel with same-tool/same-target calls
serialized   runtime must acquire a lock before invocation
```

#### `streaming`

```text
none         only terminal result
events       produces structured progress/output events
```

#### `required_tier`

Privilege tier required to execute the tool, using D18's trust model. Tools that touch
vault material or external side effects often require a higher tier than pure store
reads.

#### `required_capabilities[]`

Repository policy capability scopes that explain what services the handler may touch,
such as:

- `memory:read`
- `memory:write`
- `artifacts:create`
- `jobs:install`
- `vault:lease`

These are **not** raw D21 OS-capability grants for the model. They are runtime policy
metadata used for visibility, invocation checks, approval routing, and audit.

If a tool ultimately performs OS work through a host or another infrastructure package,
that lower layer still carries and enforces its own D21 `category:action:target`
manifest. In other words:

- D18D `required_capabilities[]` answers "which repository service scopes does this tool need?"
- D21 manifests answer "which OS operations may the implementing package perform?"

#### `preferred_lock_scope`

Optional logical lock key, such as:

- `context:<session_id>`
- `artifact:<artifact_id>`
- `job:<job_id>`

This is a runtime hint for serialized tools.

#### `timeout_seconds`

Optional default timeout for the handler. Callers may request a shorter timeout but not a
longer one unless policy explicitly allows it.

#### `stability`

```text
experimental   shape may change
stable         supported contract
deprecated     available but scheduled for removal
```

---

## ToolCall Model

Tool invocations are tracked explicitly.

```text
ToolInvocationRequest
|-- call_id
|-- tool_id
|-- arguments
|-- requested_by        user | session | job | agent | system
|-- session_id?
|-- job_id?
|-- agent_id?
|-- user_id?
|-- requested_at
|-- deadline_at?
|-- idempotency_key?
```

```text
ToolCallRecord
|-- call_id
|-- tool_id
|-- status              queued | validating | awaiting_approval | running | completed | failed | cancelled
|-- started_at?
|-- completed_at?
|-- lock_scope?
|-- approval_state
|-- metrics
```

### `requested_by`

This tells the runtime which subsystem initiated the call. Examples:

- interactive session turn
- recurring job
- delegated sub-agent
- system maintenance routine

This matters for policy, default approvals, and audit routing.

### `idempotency_key`

Optional caller-supplied key for conditional deduplication. Useful for tools such as:

- `artifact.create`
- `job.install`
- `context.append_entry`

The runtime owns dedupe semantics. Handlers should not reinvent them.

---

## Tool Events

Streaming is repository-owned rather than provider-owned.

```text
ToolEvent
|-- call_id
|-- sequence
|-- at
|-- kind                started | progress | output | artifact | memory | warning | completed | failed | cancelled
|-- payload
```

### Event rules

1. Event sequences are strictly ordered per call.
2. Every streamed call must emit exactly one terminal event:
   - `completed`
   - `failed`
   - or `cancelled`
3. Terminal events must agree with the returned `ToolResult`.
4. Event payloads must be structured JSON-like values, not provider-specific deltas.

### Recommended event kinds

- `started`
  - handler accepted and execution began
- `progress`
  - bounded progress information, phase changes, counts
- `output`
  - intermediate structured data
- `artifact`
  - emitted artifact reference
- `memory`
  - emitted memory reference
- `warning`
  - non-fatal warning
- `completed`
  - terminal success
- `failed`
  - terminal failure
- `cancelled`
  - terminal cancellation

---

## ToolResult

Terminal result shape:

```text
ToolResult
|-- call_id
|-- ok
|-- output?
|-- error?
|-- artifact_refs[]
|-- memory_refs[]
|-- metrics
```

### Metrics

```text
ToolMetrics
|-- queued_ms
|-- run_ms
|-- validation_ms
|-- approval_ms?
|-- bytes_in?
|-- bytes_out?
```

Handlers should return domain output, not provider-specific wrapping. The runtime wraps
that into the canonical `ToolResult`.

---

## ToolExecutionContext

Handlers execute against an explicit context rather than ambient global state.

```text
ToolExecutionContext
|-- session_id?
|-- job_id?
|-- agent_id?
|-- user_id?
|-- cancellation_token
|-- stores
|-- host
|-- model_gateway
|-- audit
|-- policy
|-- clock
```

### `stores`

Logical access to:

- `context_store`
- `artifact_store`
- `skill_store`
- `memory_store`

### `host`

A repository-owned facade over D18 host functionality. The handler should not speak raw
JSON-RPC or provider-specific process protocols directly.

### `model_gateway`

Interface for invoking named model profiles, embeddings, or structured generation. This
is future D18B territory, but the context reserves the boundary now.

### `audit`

Structured sink for recording the start, progress, and outcome of tool calls.

### `policy`

Capability, tier, approval, and rate/lock checks. Handlers may inspect policy decisions
but must not bypass them.

### `cancellation_token`

Cancellation must be cooperative and explicit. Long-running handlers must check for
cancellation between major steps.

---

## Runtime Interface

Every language should expose the same conceptual API.

```typescript
type ToolRuntime = {
  register(definition: ToolDefinition, handler: ToolHandler): Promise<void>;
  get(toolId: string): Promise<ToolDefinition | null>;
  list(): Promise<ToolDefinition[]>;
  validate(request: ToolInvocationRequest): Promise<ToolValidationReport>;
  invoke(request: ToolInvocationRequest): Promise<ToolResult>;
  stream(request: ToolInvocationRequest): AsyncIterable<ToolEvent>;
};

type ToolHandler = (
  args: JsonValue,
  context: ToolExecutionContext
) => Promise<ToolHandlerOutput>;
```

`ToolHandlerOutput` is the domain result before runtime wrapping. The runtime turns it
into the canonical `ToolResult`.

Equivalent APIs in Rust, Go, Python, Ruby, TypeScript, and other repository languages
must preserve the same semantics even if the syntax varies.

---

## Validation

Validation happens before policy and before handler execution.

```text
ToolValidationReport
|-- ok
|-- normalized_arguments?
|-- errors[]
|-- warnings[]
```

Validation checks:

1. tool id exists
2. input schema matches
3. required fields are present
4. unknown fields are rejected unless the schema allows them
5. enum values and scalar bounds are enforced
6. repository-specific invariants are enforced before the handler runs

Examples:

- `memory.search.limit` must be bounded
- `job.install.spec` must pass D18C/D18E validation
- `artifact.write_revision.parent_revision_id` must be structurally valid

---

## Invocation Pipeline

Every invocation follows the same pipeline:

1. `resolve`
   - load `ToolDefinition`
2. `validate`
   - schema + repository invariants
3. `policy check`
   - capability, caller class, rate/budget, and tool availability checks
4. `tier / approval check`
   - privilege tier gate, human approval if required
5. `lock acquisition`
   - for serialized tools
6. `audit start`
7. `handler execution`
8. `artifact / memory persistence hooks`
9. `audit completion`
10. `terminal result emission`

Tools must not implement bespoke approval or audit side channels. If a tool needs
approval, it declares metadata and the runtime performs the approval flow.

### What a rejected call may publish

A handler returns four caller-observable things: `output`, `artifact_refs`,
`memory_refs`, and `events`. Only `output` is validated, against the
definition's `output_schema`.

**A call that fails output validation must publish nothing the handler chose.**
Handler events are assembled during step 7 but must not reach the event stream
unless step 10 is reached with a passing result. Publishing them on the
rejection path is the worst of both worlds: the runtime declares the call
invalid and forwards the invalid call's side effects anyway, so a handler whose
output was refused can still say whatever it likes to every event sink.

The runtime's own framing events remain. Be precise about why, because the
obvious justification is wrong: it is *not* that they describe the call rather
than its result. The started event does, but the terminal event describes the
result — that is its purpose. It stays because a caller has to learn the
outcome, and it is safe to keep only because its payload is bounded to the error
kind and a message the runtime chose.

**The terminal event must not carry validation `details`.** On an
output-validation failure those details hold one path per offending field, and
each path is built from the handler's own output object — so a handler could
broadcast arbitrary text, one string per field it invents, through the very
rejection that refused it. `details` reaches the immediate caller on the
result; it must not reach the event stream, which is a broadcast.

### Built-in definitions are canonical

A tool id in the built-in catalog names one definition. Registering a *different*
definition under a built-in id is rejected.

Without this, the schema a tool is validated against is a property of whichever
`ToolDefinition` value reached `register_handler`, not of the tool id. Anything
holding a registry could register `vault.request_direct` with
`output_schema: None` *ahead of* the real binding — duplicate ids were already
refused, so the hole was getting in first rather than replacing — and silently
disable the output validation that the vault binding's V1 depends on — and nothing about the resulting registry would look
wrong. Pinning the id to its catalog entry makes "this tool has these
guarantees" a fact about the id.

This constrains only ids the catalog claims. Tools outside it — smart-home
device tools, host-local tools — are unaffected.

### The three unvalidated fields

`artifact_refs`, `memory_refs`, and `events` are passed through unchecked on the
success path, and deliberately so: a tool that creates an artifact is supposed to
reference it. There is no general rule making them empty.

For tools that handle secrets there *is* such a rule, and because it cannot come
from the runtime it must come from the binding. A binding whose handlers must
not use those channels wraps them at registration with the runtime's
`forbidding_side_channels` combinator, which fails any call whose handler
populated one of the three. The wrapper is applied once; the check runs on every
call. See section 7.1 V1.

---

## Policy, Approval, and Tier Integration

### Policy engine responsibilities

- decide whether a caller may see or invoke a tool
- check capability and target restrictions
- apply rate limits, budgets, and concurrency rules
- decide whether approval is required
- enforce trust-tier minimums

### Approval states

```text
ApprovalState
|-- not_required
|-- pending
|-- granted
|-- denied
|-- expired
```

### Approval guidance

Typically approval is required when:

- `side_effects = external`
- vault material is requested
- a tool writes sensitive durable state
- the caller is below the declared privilege tier

### Capability Cage integration

`ToolDefinition.required_capabilities[]` is checked against the runtime policy profile
and the caller's allowed tool set.

The model never directly receives D21 manifests. Instead, D18D policy scopes and D21
OS capabilities form two layers:

1. D18D decides whether the caller may use a repository tool such as `vault.request_lease`
   or `memory.search`.
2. D21 decides what the underlying implementation package is allowed to do at the OS
   boundary.

This keeps the model-facing tool surface stable even when the underlying host/package
implementation changes.

---

## Concurrency and Locking

Tools must declare whether same-target calls can run concurrently.

### Rules

- `concurrency = safe`
  - runtime may run invocations in parallel
- `concurrency = serialized`
  - runtime must serialize by lock scope

### Typical serialized tools

- `context.append_entry`
- `context.compact`
- `artifact.write_revision`
- `job.install`
- `job.uninstall`

### Typical safe tools

- `memory.search`
- `skill.read_asset`
- `artifact.read`
- `model.embed`

---

## Streaming and Cancellation

### Streaming rules

- `invoke()` must work for both non-streaming and streaming tools
- `stream()` is the preferred path when the caller wants events
- the terminal `ToolResult` must always be reconstructible from the stream

### Cancellation rules

1. The runtime may request cancellation at any time.
2. Handlers must check the cancellation token between major steps.
3. A cancelled call emits `cancelled` and returns `ToolCancelled`.
4. Partial side effects must be reflected honestly:
   - either by rollback, or
   - by reporting produced artifacts/mutations before cancellation

---

## Error Model

The runtime owns the canonical error taxonomy.

```text
ToolNotFound
ToolValidationError
ToolPermissionDenied
ToolTierDenied
ToolApprovalDenied
ToolConflict
ToolTimeout
ToolCancelled
ToolExecutionError
```

### Rules

- handlers should translate domain failures into repository-owned errors
- raw backend exceptions should not leak through directly
- a failed terminal event and `ToolResult.error` must agree on the error class

---

## Built-in Tool Families

Built-ins should be implemented over repository services, not by bypassing them.

### 1. Context tools

- `context.open_session`
- `context.append_entry`
- `context.read_entries`
- `context.create_snapshot`
- `context.compact`
- `context.archive_session`

Backed by `ContextStore`.

### 2. Artifact tools

- `artifact.create`
- `artifact.write_revision`
- `artifact.read`
- `artifact.read_revision`
- `artifact.list`
- `artifact.tag`
- `artifact.mark_retention`

Backed by `ArtifactStore`.

### 3. Skill tools

- `skill.list`
- `skill.read_manifest`
- `skill.read_asset`
- `skill.install`
- `skill.activate`
- `skill.deactivate`
- `skill.uninstall`

Backed by `SkillStore`.

### 4. Memory tools

- `memory.remember`
- `memory.search`
- `memory.list_by_class`
- `memory.list_by_tag`
- `memory.supersede`
- `memory.expire`
- `memory.tombstone`

Backed by `MemoryStore`.

### 5. Job tools

- `job.install`
- `job.uninstall`
- `job.run_now`
- `job.list`
- `job.status`
- `job.validate`

Backed by D18C job runtime and D18E portability validator.

### 6. Channel tools

- `channel.read`
- `channel.write`
- `channel.ack`

These wrap repository channel services rather than exposing raw log internals.

### 7. Vault tools

- `vault.request_lease`
- `vault.request_direct`

These always participate in approval/tier checks.

#### 7.1 Host dispatch binding

The two definitions above are declarations. This subsection is the normative
contract for the handlers that *implement* them at the host boundary, so that a
reviewer can check an implementation against a written rule rather than against
the author's intent.

A conforming binding registers exactly these two handlers against a vault
runtime that owns the secret material, and it obeys four invariants.

**V1 — no secret return channel.** Neither handler may place secret bytes, or
any value derived from secret bytes, into `ToolHandlerOutput.output`,
`artifact_refs`, `memory_refs`, `events`, or into any field of a returned
`ToolCallError`. `vault.request_lease` returns exactly `{ vault_ref,
expires_at_ms }`. `vault.request_direct` returns JSON `null`, and moves the
payload into the trusted delivery adapter rather than receiving the bytes back
to forward.

How much of this the runtime enforces, precisely — stated exactly, because an
invariant a reviewer trusts too far is worse than one they do not trust at all:

| Field | Enforced by | Notes |
|---|---|---|
| `output` | the runtime — *shape* only | validated against `output_schema` after the handler returns; content is excluded only where that schema is `null` |
| `artifact_refs` | the handler | copied to the result unchecked |
| `memory_refs` | the handler | copied to the result unchecked |
| `events` | the runtime, then the handler | published only when output validation passes; the handler chooses their content |
| `ToolCallError` | the handler | the runtime adds only a path and a JSON type name, never the offending value |

So for `vault.request_direct`, `output` genuinely cannot carry bytes — the
declared schema is `null` and a non-null output is rejected.

The other three fields the runtime does not inspect on the success path. A
binding whose handlers must not use them wraps them with
`forbidding_side_channels` at registration, which fails any call whose handler
populated one; see "The three unvalidated fields" above. Returning them empty
and testing that is necessary but not sufficient, because a test proves what the
handler does today and the wrapper proves what it can do at all.

One remaining limit on the `output` guarantee, and one that used to exist and no
longer does. Still true: a rejected output is discarded rather than echoed, so
the rejection path is not itself a channel — and since the terminal event no
longer carries validation `details`, that holds for the broadcast too. No longer
true: the guarantee used to be a property of the `ToolDefinition` passed to
`register_handler` rather than of the tool id, so a definition registered with
`output_schema: None` skipped validation. Built-in ids are now pinned to their
catalog entry, so that registration is rejected outright. Ids the catalog does
not claim remain the registrant's responsibility.

Finally, note what `vault_ref` is. It is not secret material, but it *is* a
bearer capability: whoever holds it can redeem it until it is consumed or
expires. `VaultLeaseReceipt` redacts it in `Debug` for that reason, but a
successful `vault.request_lease` necessarily puts it into the tool output, from
where the runtime copies it into the terminal event payload and the execution
journal. Any sink that persists or prints journals is therefore persisting live
capabilities and must be written knowing that. Redacting `vault.*` outputs at
the sink, or shortening lease TTLs on the agent-facing path, are both reasonable
responses; pretending the receipt is inert is not.

**V2 — bounded, secret-free errors.** Handler errors carry a fixed
`ToolErrorKind` and one of a closed set of static messages. A handler must not
interpolate the requested secret name, the payload, a `VaultRef`, or any
adapter-supplied string into the message or `details`. `details` stays JSON
`null` unless it carries a value already safe to show the caller.

This bounds what an error may *contain*; it does not claim the error set carries
no information. It does not: a caller can distinguish "secret not registered"
from success, and can distinguish the three delivery failures from each other,
so it can probe both the set of registered names and the adapter's consumer
table. Both are accepted deliberately. The caller supplied the name, and — until
per-secret authorization exists (see below) — name enumeration is strictly the
lesser problem, while collapsing the delivery failures would make a
misconfigured adapter undebuggable. A binding that needs those distinctions
hidden must suppress them explicitly; nothing here does it automatically.

**V3 — validate arguments independently of the registry.** A handler may be
invoked directly, without the registry's schema validation in front of it, so
each handler re-checks argument presence and type and returns
`ToolValidationError` on any mismatch. Missing, null, wrong-typed, and
out-of-range arguments are all validation failures, never panics: no `unwrap`,
no indexing, no arithmetic that can overflow on caller-controlled input.

**V4 — the registration path is the tier gate.** Handlers are attached with
`register_handler`, which enforces `allows_tool`, the `required_tier` ceiling
and the `required_capabilities` of the definition being registered. A binding
must not bypass that path by inserting into the handler map directly, and must
not register a vault tool against a host profile whose `max_tier` is below
`Tier2`. Registration failure is fatal to host startup rather than degraded
into an unregistered tool, because a missing vault handler is observationally
identical to a denied one from the agent's side.

Definitions must come from the built-in catalog rather than being reconstructed
by the binding, so a binding cannot register a vault tool under a weaker tier,
a smaller capability set, or a laxer `output_schema` than the rest of the system
validates against.

Registering the pair is all-or-nothing: pre-flight every definition before
attaching any handler. A host left holding one vault tool because the second was
refused is worse than one holding neither, since the tool that did register
looks healthy and the failure resurfaces later and elsewhere.

That rule is about *partial failure*, not about which tools a deployment
chooses to offer. A binding may deliberately register only `vault.request_lease`
— and must, where no trusted delivery adapter exists, because `request_direct`
without one has nowhere to deliver. Registering it anyway against a stub that
accepts everything would be strictly worse than leaving it unregistered: it
would present a working direct-delivery tool to an agent while the secret went
nowhere, or worse, somewhere unaudited.

The distinction to preserve is that a deliberate subset is chosen up front and
is complete in itself, whereas a partial registration is the residue of
something going wrong halfway. A conforming binding therefore exposes the
lease-only case as its own named operation rather than by catching an error
from the pair, so the two are distinguishable in the code that calls it.

Note what this gate does *not* do. It runs once, at registration, per host — it
is not a per-call check. Per-call authorization is the policy engine's job, and
the default policy admits everything, so a binding that wants per-call control
must install a policy rather than assume one.

#### 7.2 What this binding does *not* establish

`vault.request_direct`'s `consumer_agent_id` names a consumer the caller asserts
is authorized. The handler does not perform that authorization; it forwards to
the trusted adapter, which is the component entitled to accept or refuse. A
binding that treats a caller-supplied `consumer_agent_id` as proof of
authorization is non-conforming.

For the adapter to be able to refuse for a *reason*, it has to know what it is
being asked. A binding must therefore forward the requesting agent, the session,
and the secret name alongside the destination. Given only `(consumer, payload)`,
the strongest rule an adapter can express is a global destination allowlist —
and under that rule a caller cleared to send one secret to a consumer is equally
cleared to send every secret to it, because nothing in the chain can tell the
two requests apart. That is a confused deputy: the adapter holds the authority
but not the facts.

Forwarding those fields makes a decision *possible*. It does not make one. Two
gaps remain open, and are recorded here so nobody mistakes silence for safety:

1. **There is no per-secret policy.** The `privilege_tier`, `allowed_agents`,
   and `allowed_mode` metadata this system's vault requirements call for is not
   implemented. The tier and capability checks in V4 are per-*tool* and run once
   at registration, so they cannot express "this agent may read that secret" or
   "this secret is direct-delivery only". Until per-secret policy exists, any
   caller that clears the tool gate can request any registered secret by name,
   in either mode — including leasing a secret that was meant to be
   direct-delivery only, which hands the caller exactly the material direct
   delivery exists to withhold.

2. **The identity fields are only as good as their source.** This covers all
   three — `requesting_agent_id`, `requesting_user_id`, and `session_id`. Each
   is read from the execution context, which is populated from the tool
   invocation request. If a host fills those from caller-supplied values rather
   than from an attested identity, an adapter that authorizes on any of them is
   authorizing on the attacker's own claim. Any binding that relies on a field
   must first establish that its host attests *that* field; attesting one says
   nothing about the others.

### 8. Filesystem tools

- `fs.read`
- `fs.write`
- `fs.list`

These are intentionally lower-level and should be used more sparingly than durable
store-backed tools.

### 9. Network tools

- `network.fetch`

The first phase should prefer one typed fetch tool rather than a loose socket surface.

### 10. Model tools

- `model.generate`
- `model.embed`

These target named profiles through the ModelGateway rather than raw provider names.

### 11. Delegation tools

- `agent.spawn`
- `agent.send`
- `agent.await`

These are the high-level sub-agent tools used by the future session kernel.

---

## Example Definitions

### Example: `artifact.create`

```text
tool_id: artifact.create
side_effects: write
idempotency: conditional
concurrency: safe
streaming: none
required_tier: 0
required_capabilities:
  - artifacts:create
```

Input:

```json
{
  "collection": "plans",
  "name": "quarterly-digest-plan.md",
  "content_type": "text/markdown",
  "labels": ["draft"],
  "body_base64": "..."
}
```

Output:

```json
{
  "artifact_id": "art_123",
  "revision_id": "rev_001"
}
```

### Example: `memory.search`

```text
tool_id: memory.search
side_effects: read
idempotency: always
concurrency: safe
streaming: none
required_tier: 0
required_capabilities:
  - memory:read
```

Input:

```json
{
  "query": "user preferences for weekly summaries",
  "classes": ["profile", "procedure"],
  "limit": 10
}
```

Output:

```json
{
  "matches": [
    {
      "memory_id": "mem_001",
      "score": 0.91
    }
  ]
}
```

### Example: `job.install`

```text
tool_id: job.install
side_effects: write
idempotency: conditional
concurrency: serialized
streaming: events
required_tier: 1
```

This tool validates the `JobSpec`, runs portability checks, compiles an install plan,
and emits progress events as files/commands are staged.

---

## Provider Translation

The Tool API remains provider-neutral. Provider adapters are responsible for translation.

### Rules

- Anthropic tool schemas are generated by the Anthropic driver
- OpenAI function/tool schemas are generated by the OpenAI driver
- local model adapters may degrade gracefully when native tool calling is unavailable

### Important boundary

The rest of the repository must only depend on `ToolDefinition`, `ToolInvocationRequest`,
`ToolEvent`, and `ToolResult`. No provider-specific field names should leak into built-in
tools or session logic.

---

## Persistence and Audit

Tool runtimes should persist enough information to reconstruct what happened.

Minimum audit record:

```text
ToolAuditRecord
|-- call_id
|-- tool_id
|-- requested_by
|-- started_at
|-- completed_at?
|-- status
|-- approval_state
|-- lock_scope?
|-- result_summary
```

Suggested persistence:

- invocation record in durable session/job logs
- produced artifact refs
- produced memory refs
- approval decision trace

---

## Phase 1 Deliverables

The first implementation phase should deliver:

1. `D18D` spec
2. repository-owned `ToolDefinition`, `ToolInvocationRequest`, `ToolEvent`, and `ToolResult` types
3. a `ToolRuntime` with registration, validation, and invocation
4. built-ins for stores and jobs first
5. provider translation in the future model-gateway layer

### Recommended first built-ins

- `context.open_session`
- `context.append_entry`
- `artifact.create`
- `artifact.read`
- `memory.remember`
- `memory.search`
- `job.validate`
- `job.install`

These give the session kernel useful durable primitives without immediately exposing the
broadest host surfaces.
