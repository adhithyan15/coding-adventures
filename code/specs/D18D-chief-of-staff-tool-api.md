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
