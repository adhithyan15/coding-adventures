# VMCOND00 — VM Condition System

**Status:** Draft  
**Series:** VMCOND (VM Condition System)  
**Depends on:** LANG02 (vm-core), LANG01 (interpreter-ir), LANG05 (backend-protocol)

---

## 1. Motivation and Design Philosophy

Every production language eventually needs a way to say "something unusual happened
here — what should we do about it?" Most bytecode VMs bolt on a single answer: unwind
the stack, find a matching catch block, run it. The JVM, CLR, and Python all do this.
It works, but it hard-codes a policy (stack unwinding) into the mechanism (error
signaling), which makes it impossible to build the richer control structures that
languages like Common Lisp, Dylan, and Racket provide.

Common Lisp's condition system is the most carefully designed answer in the history of
programming languages. Its key insight is that **signaling an unusual situation and
deciding what to do about it are two separate acts**, and they should happen
independently:

1. **Signal** — announce that something unusual occurred. The call stack is left
   completely intact. Nothing has been decided yet.
2. **Handle** — a handler runs, with the full live stack beneath it. The handler
   inspects the situation and chooses a course of action.
3. **Restart** — a named "way to continue" that was established by the code that
   called the signaling code. The handler can invoke one, which may or may not unwind
   the stack.

The value: a handler in high-level application code can decide how low-level library
code recovers, without the library having to anticipate every recovery strategy in
advance. The library offers restarts; the application picks one.

This VM implements that model as a set of opt-in opcodes. A language that wants
traps-only pays nothing. A language that wants the full condition system declares its
intent in its module header and emits the appropriate opcodes. Every layer is a strict
superset of the one below; no layer removes capabilities added by a lower one.

---

## 2. Capability Layers

The VM defines six capability layers, numbered 0–5. A module declares the highest layer
it uses. The VM allocates the runtime infrastructure that layer requires and enables
verification of the corresponding opcodes.

| Layer | Name | Key Opcodes | Runtime Cost Added |
|-------|------|-------------|-------------------|
| 0 | **Traps** | *(none)* | None — failure aborts |
| 1 | **Result values** | `SYSCALL_CHECKED`, `BRANCH_ERR` | One error register per frame |
| 2 | **Unwind exceptions** | `THROW` + static exception table | Exception table in module header |
| 3 | **Dynamic handlers** | `PUSH_HANDLER`, `POP_HANDLER`, `SIGNAL`, `ERROR`, `WARN` | Handler chain (per-thread linked list) |
| 4 | **Restarts** | `PUSH_RESTART`, `POP_RESTART`, `FIND_RESTART`, `INVOKE_RESTART`, `COMPUTE_RESTARTS` | Restart chain (per-thread linked list) |
| 5 | **Non-local exits** | `ESTABLISH_EXIT`, `EXIT_TO` | Exit-point chain (per-thread linked list) |

A module that declares Layer 4 automatically has access to Layers 0–4. It does not
automatically gain Layer 5; non-local exits must be declared explicitly (the penalty
is a third chain to maintain).

In practice:
- Simple calculators and brainfuck-style languages: Layer 0.
- Go-style languages with typed errors: Layer 1.
- Java/Python-style languages with try/catch: Layer 2.
- Scheme-style languages with `with-exception-handler`: Layer 3.
- Languages with full condition/restart systems (Dylan, Common Lisp): Layers 3–5.

---

## 3. Runtime Chains

Layers 3–5 require the VM to maintain **three independent runtime chains**, each
maintained as a singly-linked list anchored in the current thread's execution context.
They are independent — a SIGNAL search walks the handler chain, not the call stack;
a FIND_RESTART search walks the restart chain, not the call stack.

```
Thread execution context
├── call_stack          — activation records (always present)
├── handler_chain       — pushed by PUSH_HANDLER, popped by POP_HANDLER (Layer 3+)
├── restart_chain       — pushed by PUSH_RESTART, popped by POP_RESTART (Layer 4+)
└── exit_point_chain    — pushed by ESTABLISH_EXIT, popped on EXIT_TO or frame exit (Layer 5+)
```

Each chain node records the stack depth at the time of its push, so that EXIT_TO and
INVOKE_RESTART can unwind to the correct frame when needed.

### 3.1 Handler Chain Node

```
HandlerNode {
    condition_type : ConditionTypeRef,  // what kind of condition this handles
    handler_fn     : FnRef,             // callable to invoke (non-unwinding)
    stack_depth    : usize,             // call stack depth when handler was pushed
    prev           : *HandlerNode,      // linked list pointer
}
```

`condition_type` may be a specific type, a parent type (handler matches all subtypes),
or the sentinel `ALL` which matches every condition.

### 3.2 Restart Chain Node

```
RestartNode {
    name           : Symbol,            // name used by FIND_RESTART
    restart_fn     : FnRef,             // the restart implementation
    stack_depth    : usize,             // call stack depth when restart was pushed
    prev           : *RestartNode,
}
```

Restarts are named so that handlers can find them by name without holding a direct
reference. The name is a VM Symbol value, compared by identity (interned).

### 3.3 Exit Point Chain Node

```
ExitPointNode {
    tag            : Symbol,            // matched by EXIT_TO
    resume_ip      : InstrPtr,          // where execution continues after EXIT_TO
    frame_depth    : usize,             // call stack depth to unwind to
    result_reg     : RegIdx,            // register to receive the exit value
    prev           : *ExitPointNode,
}
```

---

## 4. Condition Objects

A condition is a first-class VM value — not an error code, not a string. It is a
structured object with a type and fields, similar to a record or class instance.

The VM defines a minimal condition protocol. Each language built on top can extend it.

```
ConditionObject {
    type_id   : ConditionTypeRef,   // VM-managed type identifier
    fields    : Vec<LangValue>,     // type-specific payload
    message   : Option<Symbol>,     // optional human-readable description
    origin_ip : InstrPtr,           // instruction pointer where signaling occurred
    origin_fn : FnRef,              // function where signaling occurred
}
```

The VM type hierarchy for built-in condition types:

```
Condition
├── Warning                         ; WARN — unhandled is continue
├── Error                           ; ERROR — unhandled is abort
│   ├── TypeMismatch
│   ├── UnboundVariable
│   ├── ArityError
│   ├── IOCondition                 ; defined in SYSCALL01
│   │   ├── WriteError
│   │   ├── ReadError
│   │   └── EOFCondition
│   └── ExitRequest                 ; defined in SYSCALL01
└── SimpleCondition                 ; SIGNAL — unhandled is continue
```

Languages can define additional subtypes by registering them in the module header. The
VM provides subtype checking (`is_subtype(a, b)`) used during handler chain traversal.

---

## 5. Opcode Reference

Opcodes are grouped by the layer that introduces them. Each opcode's full effect on VM
state is specified. Operands are described using these conventions:

- `<reg>` — a register index in the current frame
- `<imm>` — an inline immediate value in the instruction stream
- `<label>` — a branch target (instruction pointer offset)
- `<sym>` — a symbol reference (into the module's symbol table)
- `<fn>` — a callable reference (function pointer or closure)

---

### Layer 1 Opcodes

#### `SYSCALL_CHECKED <n:imm> <arg:reg> → <val:reg> <err:reg>`

Execute host syscall number `n` with argument from `<arg>`. Unlike `SYSCALL`, this
variant does not trap on failure. Instead it writes two results:
- `<val>` — the return value (meaningful only if `<err>` is zero)
- `<err>` — zero on success; a platform-specific error code on failure

The mapping from `n` to host operations is defined in SYSCALL00. The error codes are
defined in SYSCALL01.

```
effect:
  (val, err) = host_syscall_checked(n, registers[arg])
  registers[val_reg] = val
  registers[err_reg] = err
```

#### `BRANCH_ERR <err:reg> <label>`

Branch to `<label>` if `<err>` contains a non-zero error code; fall through otherwise.

```
effect:
  if registers[err] != 0:
      ip = label
```

---

### Layer 2 Opcodes

#### `THROW <val:reg>`

Unwind the call stack, searching the **static exception table** in the current module's
header for the innermost handler whose type covers `typeof(registers[val])`. If found,
unwind to that handler's frame and jump to its instruction pointer, placing the
condition value into the handler's designated register.

If no handler is found in the current module, propagate the throw to the caller's frame
and repeat. If propagation reaches the top of the thread without finding a handler,
abort the thread.

The static exception table entry format:

```
ExceptionTableEntry {
    from_ip   : u32,   // inclusive start of guarded range
    to_ip     : u32,   // exclusive end of guarded range
    handler_ip: u32,   // where to jump on match
    type_id   : u32,   // condition type to match (ALL_TYPES = 0xFFFF_FFFF)
    val_reg   : u8,    // register to receive the condition object
}
```

```
effect:
  condition = registers[val]
  for entry in current_module.exception_table (innermost first):
      if from_ip <= ip < to_ip and is_subtype(typeof(condition), entry.type_id):
          unwind_to(entry.handler_ip)
          registers[entry.val_reg] = condition
          ip = entry.handler_ip
          return
  propagate_to_caller(condition)
```

---

### Layer 3 Opcodes

#### `PUSH_HANDLER <type:imm> <fn:reg>`

Push a handler onto the thread-local handler chain. The handler covers conditions of
type `type` (and all subtypes). `<fn>` is a callable in `<reg>` that will be invoked
non-unwinding when a matching condition is signaled.

The handler remains active until `POP_HANDLER` is executed. `PUSH_HANDLER` and
`POP_HANDLER` must be paired within the same function (verified at load time).

```
effect:
  node = HandlerNode {
      condition_type: type,
      handler_fn:     registers[fn],
      stack_depth:    current_stack_depth(),
      prev:           thread.handler_chain,
  }
  thread.handler_chain = node
```

#### `POP_HANDLER`

Pop the most recently pushed handler from the handler chain.

```
effect:
  assert thread.handler_chain != null   // verification error if violated
  thread.handler_chain = thread.handler_chain.prev
```

#### `SIGNAL <val:reg>`

Walk the handler chain looking for the first node whose `condition_type` covers
`typeof(registers[val])`. If found, invoke the handler function **without unwinding the
call stack** (see Section 6 for the non-unwinding call protocol). If no handler is
found, continue execution — `SIGNAL` with no handler is a no-op.

```
effect:
  condition = registers[val]
  node = thread.handler_chain
  while node != null:
      if is_subtype(typeof(condition), node.condition_type):
          invoke_nonunwinding(node.handler_fn, [condition])
          return  // resume here after handler returns normally
      node = node.prev
  // no handler found — continue
```

#### `ERROR <val:reg>`

Identical to `SIGNAL` except: if no handler is found in the handler chain, abort the
current thread with the condition as the termination reason. If the condition is also
covered by a Layer 2 static exception table entry that is in scope, that entry takes
priority over the handler chain.

```
effect:
  condition = registers[val]
  // First check Layer 2 static exception table
  if current_ip_in_guarded_range() and table_covers(condition):
      execute_throw(condition)
      return
  // Then walk handler chain
  node = thread.handler_chain
  while node != null:
      if is_subtype(typeof(condition), node.condition_type):
          invoke_nonunwinding(node.handler_fn, [condition])
          return
      node = node.prev
  // No handler — abort thread
  abort_thread(condition)
```

#### `WARN <val:reg>`

Identical to `SIGNAL` except: if no handler is found, the VM emits the condition's
message to stderr (if available) and continues execution. `WARN` never aborts.

```
effect:
  condition = registers[val]
  node = thread.handler_chain
  while node != null:
      if is_subtype(typeof(condition), node.condition_type):
          invoke_nonunwinding(node.handler_fn, [condition])
          return
      node = node.prev
  // No handler — print warning, continue
  emit_warning(condition)
```

---

### Layer 4 Opcodes

#### `PUSH_RESTART <name:sym> <fn:reg>`

Push a named restart onto the thread-local restart chain. The restart is callable by
name via `FIND_RESTART`. Restarts are searched in push order (most recently pushed
first), so inner restarts with the same name shadow outer ones.

```
effect:
  node = RestartNode {
      name:        sym,
      restart_fn:  registers[fn],
      stack_depth: current_stack_depth(),
      prev:        thread.restart_chain,
  }
  thread.restart_chain = node
```

#### `POP_RESTART`

Pop the most recently pushed restart from the restart chain.

```
effect:
  assert thread.restart_chain != null
  thread.restart_chain = thread.restart_chain.prev
```

#### `FIND_RESTART <name:sym> → <result:reg>`

Search the restart chain for a restart named `sym`. Write the restart handle into
`<result>`, or `NIL` if not found. Does not invoke the restart.

```
effect:
  node = thread.restart_chain
  while node != null:
      if node.name == sym:
          registers[result] = RestartHandle { node }
          return
      node = node.prev
  registers[result] = NIL
```

#### `INVOKE_RESTART <handle:reg> <arg:reg>`

Invoke the restart referenced by `<handle>`, passing `<arg>` as its argument. This
may or may not unwind the stack depending on the restart's implementation:

- If the restart function calls `EXIT_TO` (Layer 5), the stack will unwind.
- If the restart function returns normally, execution resumes after `INVOKE_RESTART`
  with the restart's return value in `<handle>`.

This is the only Layer 4 opcode that requires Layer 5 opcodes inside the restart
body if unwinding is desired. A restart can be purely non-unwinding (returns a
substitute value) or unwinding (calls EXIT_TO).

```
effect:
  handle = registers[handle]
  arg    = registers[arg]
  assert handle != NIL
  result = call(handle.restart_fn, [arg])
  registers[handle_reg] = result   // if restart returned normally
```

#### `COMPUTE_RESTARTS → <result:reg>`

Collect all currently visible restarts into a list and write it into `<result>`. Useful
for presenting restart choices to the user (e.g., in a debugger or REPL).

```
effect:
  list = []
  node = thread.restart_chain
  while node != null:
      list.push(RestartHandle { node })
      node = node.prev
  registers[result] = lang_list(list)
```

---

### Layer 5 Opcodes

#### `ESTABLISH_EXIT <tag:sym> <result:reg> <after:label>`

Push an exit point onto the exit-point chain. Code inside the dynamic extent of this
instruction can call `EXIT_TO <tag>` to jump to `<after>` with a value delivered into
`<result>`. If no `EXIT_TO` fires, execution falls through to `<after>` naturally with
`<result>` unchanged.

```
effect:
  node = ExitPointNode {
      tag:         sym,
      resume_ip:   after,
      frame_depth: current_stack_depth(),
      result_reg:  result,
      prev:        thread.exit_point_chain,
  }
  thread.exit_point_chain = node
  // execution continues at the next instruction
  // when the dynamic extent ends (EXIT_TO fires or normal fallthrough):
  //   thread.exit_point_chain = node.prev
```

#### `EXIT_TO <tag:sym> <val:reg>`

Search the exit-point chain for a node with `tag == sym`. Unwind the call stack to
the depth recorded in that node, pop all handler chain nodes that were pushed after
that depth, pop all restart chain nodes pushed after that depth, remove the exit-point
node from the chain, write `registers[val]` into the node's `result_reg`, and jump to
the node's `resume_ip`.

This is the unwinding primitive that `INVOKE_RESTART` uses when a restart needs to
transfer control non-locally.

```
effect:
  val  = registers[val]
  node = thread.exit_point_chain
  while node != null:
      if node.tag == sym:
          unwind_stacks_to_depth(node.frame_depth)
          thread.exit_point_chain = node.prev
          registers[node.result_reg] = val
          ip = node.resume_ip
          return
      node = node.prev
  // tag not found — abort: EXIT_TO with no matching ESTABLISH_EXIT
  abort_thread(make_condition(UnboundExitTag, sym))
```

---

## 6. Non-Unwinding Handler Invocation Protocol

This section specifies exactly how `SIGNAL`, `ERROR`, and `WARN` call a handler
function without disturbing the call stack beneath it. This is the heart of what
distinguishes Layer 3 from Layer 2.

When a matching handler is found, the VM performs:

1. **Save resume state.** Record the current `ip` (the instruction *after* `SIGNAL`)
   and the current call stack depth.

2. **Push a handler invocation frame** onto the call stack. This is a special frame
   type with an extra field:

   ```
   HandlerInvocationFrame {
       ..normal_frame_fields..,
       resume_ip    : InstrPtr,   // where to return after handler completes normally
       signal_depth : usize,      // stack depth at the time of signaling
       is_handler   : bool = true,
   }
   ```

3. **Call the handler function** with the condition object as its single argument.
   The call looks like a normal function call from the handler's perspective.

4. **On normal return** from the handler: detect the `HandlerInvocationFrame`,
   pop it, restore `ip` to `resume_ip`, and continue.

5. **On `EXIT_TO` inside the handler**: `EXIT_TO` unwinds past the
   `HandlerInvocationFrame` as it would any other frame, finding the matching
   `ExitPointNode` in the chain. The resume-from-handler path is abandoned.

6. **On `INVOKE_RESTART` inside the handler**: if the restart returns normally, the
   handler frame is still live and the handler can examine the result. If the restart
   calls `EXIT_TO`, the unwind proceeds as above.

The invariant the protocol maintains: **the call stack between the signaling frame and
the handler invocation frame is never touched**. No locals are invalidated, no frame
is popped. The handler can introspect the live stack using `COMPUTE_RESTARTS`
(which was established by code in those frames).

---

## 7. Module Capability Declaration

Each module's header includes a capability word:

```
CapabilityFlags : u8 {
    LAYER_1 = 0x01,   // result values
    LAYER_2 = 0x02,   // unwind exceptions + exception table
    LAYER_3 = 0x04,   // dynamic handlers
    LAYER_4 = 0x08,   // restarts
    LAYER_5 = 0x10,   // non-local exits
}
```

Higher layers imply lower ones for validation purposes (a module with `LAYER_4` may
also emit `PUSH_HANDLER`), but the VM only allocates infrastructure for the layers the
module explicitly declares. A module declaring `LAYER_3 | LAYER_4` gets handler and
restart chains but not the exit-point chain.

If a module emits an opcode from a layer it did not declare, the VM raises a
`VerificationError` at module load time — not at runtime.

Custom condition types are registered in the module header's condition type table:

```
ConditionTypeEntry {
    type_id    : u32,           // locally unique within this module
    name       : SymbolRef,     // for debugging / FIND_RESTART by name
    parent_id  : u32,           // 0 = root Condition type
    field_count: u8,
}
```

The VM computes a global type ID by combining the module ID and the local type ID.
`is_subtype` walks the registered parent chain.

---

## 8. Penalty Accounting

These are the runtime overheads incurred by each layer, to help language implementors
make informed decisions.

**Layer 0 — Zero overhead.** No additional instructions, no additional state.

**Layer 1 — One register per frame.** The error register is a slot in every activation
record for modules that declare Layer 1. Cost: one extra word of memory per frame.
`SYSCALL_CHECKED` costs slightly more than `SYSCALL` (one additional write).

**Layer 2 — Module table + THROW search.** The exception table is a static structure;
its cost is memory, not time. On the happy path (no throw), zero overhead.
`THROW` costs O(exception table entries × stack depth) in the worst case, but since
throws are exceptional this is acceptable.

**Layer 3 — Handler chain push/pop on every `PUSH_HANDLER`.** If a function pushes
no handlers, it pays nothing. If it pushes `k` handlers, it pays `2k` pointer writes
(push + pop). `SIGNAL` costs O(h) where `h` is the number of active handlers — in
practice 2–5. The non-unwinding invocation frame adds one extra frame to the call stack
per handler invocation.

**Layer 4 — Restart chain push/pop.** Same asymptotic structure as Layer 3. Functions
that establish restarts pay for them; functions that do not establish restarts pay
nothing. `FIND_RESTART` is O(r) where `r` is the number of active restarts.
`COMPUTE_RESTARTS` allocates a list — it is expected to be called rarely (debugger,
REPL).

**Layer 5 — Exit point chain push/pop.** Same structure. `EXIT_TO` walks the exit
point chain (O(e)) then unwinds the stacks — the unwind cost is proportional to the
number of frames between the `EXIT_TO` call site and the `ESTABLISH_EXIT` site. This
is the most expensive opcode in the system when it fires, but it only fires on
non-local transfers.

---

## 9. Interaction with Existing IR

The opcodes defined here are added to the compiler IR (`IrOp`) and the interpreter IR
(`IIRInstr`) as new variants. No existing opcode is modified. The Rust implementations
of the IrOp and IIRInstr types gain new variants:

```rust
// Compiler IR additions (compiler-ir crate)
pub enum IrOp {
    // ... existing ...

    // Layer 1
    SyscallChecked,     // operands: IrImmediate(n), IrRegister(arg), IrRegister(val_out), IrRegister(err_out)
    BranchErr,          // operands: IrRegister(err), IrLabel(target)

    // Layer 2
    Throw,              // operands: IrRegister(condition)

    // Layer 3
    PushHandler,        // operands: IrImmediate(type_id), IrRegister(fn)
    PopHandler,
    Signal,             // operands: IrRegister(condition)
    Error,              // operands: IrRegister(condition)
    Warn,               // operands: IrRegister(condition)

    // Layer 4
    PushRestart,        // operands: IrLabel(name_sym), IrRegister(fn)
    PopRestart,
    FindRestart,        // operands: IrLabel(name_sym), IrRegister(out)
    InvokeRestart,      // operands: IrRegister(handle), IrRegister(arg)
    ComputeRestarts,    // operands: IrRegister(out)

    // Layer 5
    EstablishExit,      // operands: IrLabel(tag_sym), IrRegister(result_out), IrLabel(after)
    ExitTo,             // operands: IrLabel(tag_sym), IrRegister(val)
}
```

Backends (JVM, CLR, BEAM) are responsible for lowering these opcodes to their
platform's equivalent. Lowering guidance per backend is out of scope for this spec and
will be covered in platform-specific backend specs as the features are implemented.

---

## 10. Verification Rules

The VM verifier checks the following at module load time (before any execution):

1. **Layer consistency.** Every opcode used must be from a layer declared in the
   module capability flags.
2. **PUSH/POP pairing.** Every `PUSH_HANDLER` must be dominated by exactly one
   `POP_HANDLER` on every control flow path that exits the region. Same for
   `PUSH_RESTART` / `POP_RESTART` and `ESTABLISH_EXIT` (exit point is automatically
   cleaned up on `EXIT_TO` or fallthrough).
3. **Exception table coverage.** Every `THROW` must be either inside a guarded range
   in the exception table or in a function whose caller's stack is covered. (Uncovered
   throws are legal; they propagate to the thread root.)
4. **FIND_RESTART targets.** `FIND_RESTART` with a given symbol must have a
   corresponding `PUSH_RESTART` that dominates it on the relevant paths — this is a
   best-effort warning, not a hard error, since restarts can be established by callers.
5. **EXIT_TO tags.** Every `EXIT_TO` must have a corresponding `ESTABLISH_EXIT` with
   the same tag that dominates it in the dynamic extent — again best-effort, since
   exit points can be established by callers.

---

## 11. Phase Plan

### Phase 1 — Layers 0 and 1 (Traps + Result Values)
- `SYSCALL_CHECKED` and `BRANCH_ERR` in compiler IR and interpreter IR
- Rust vm-core updated to carry an error register per frame for Layer 1 modules
- Acceptance: `(host/read-byte-checked) → (val, err)` where `err` is non-zero on EOF,
  without aborting the VM

### Phase 2 — Layer 2 (Unwind Exceptions)
- Static exception table in module header
- `THROW` opcode in interpreter VM and compiler IR
- Acceptance: `(try (/ x 0) (catch DivisionByZero e 0))` compiles to bytecode with
  an exception table entry; the VM catches the thrown condition and resumes

### Phase 3 — Layer 3 (Dynamic Handlers)
- Handler chain in vm-core thread context
- `PUSH_HANDLER`, `POP_HANDLER`, `SIGNAL`, `ERROR`, `WARN`
- Non-unwinding invocation protocol (Section 6)
- Acceptance: a handler registered above a call can observe a condition signaled
  inside that call without the call stack unwinding

### Phase 4 — Layer 4 + 5 (Restarts + Non-Local Exits)
- Restart chain and exit-point chain in vm-core thread context
- Full opcode set
- Acceptance: a restart named `use-value` established by outer code can be found by a
  handler in inner code, invoked with a substitute value, and execution resumes in the
  outer code at the post-ESTABLISH_EXIT instruction
