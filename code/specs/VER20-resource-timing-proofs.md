# VER20: Resource-Bound and Constant-Time Trace Proofs

**Status:** Normative Specification  
**Authority:** VER00 §7.4, §7.6.3, VER04 §7, VER19, VER25, VER26 §8.1  
**Target Weaknesses:** CWE-208 (Observable Timing Discrepancy), CWE-125/787 (Out-of-Bounds Memory), CWE-400 (Uncontrolled Resource Consumption)  

---

## 1. Scope and Mathematical Foundation

VER20 specifies formal verification of **2-safety hyperproperties** (constant-time execution) and **resource bounds** over Warrant Intermediate Representation (WIR).

A standard safety property (1-safety) asserts a property of individual execution traces (e.g. absence of crashes or functional input/output equivalence). Constant-time execution is a **2-safety hyperproperty**: it relates pairs of execution traces. Proving equal instruction counts alone is insufficient; VER20 requires formal equivalence of all compiler-observable events (branch decisions, memory access addresses, and total step counts) under 2-run self-composition.

---

## 2. Self-Composition Model

Let $P$ be a WIR program or function with formal parameters $X = X_{\text{pub}} \cup X_{\text{sec}}$, partitioned by security lattice labels:
- $X_{\text{pub}} = \{ x \in X \mid \text{label}(x) = \text{PUBLIC} \}$
- $X_{\text{sec}} = \{ x \in X \mid \text{label}(x) = \text{SECRET} \}$

### 2.1 Trace Equivalence ($\sim_{\text{obs}}$)

An observable event $e \in \text{ObsEvent}$ is one of:
1. `BranchDecision(cond)`: The boolean condition controlling a conditional branch (`BranchIf`).
2. `MemoryAccess(prov, offset, size)`: The memory location (provenance ID and byte offset) dereferenced during a `Load` or `Store`.
3. `StepCount(steps)`: The cumulative abstract cost incurred along an execution path.

Two execution traces $\tau_1 = [e_{1,1}, \dots, e_{1,m}]$ and $\tau_2 = [e_{2,1}, \dots, e_{2,n}]$ are observationally equivalent ($\tau_1 \sim_{\text{obs}} \tau_2$) if and only if:
$$m = n \quad \land \quad \forall i \in [1, m].\, e_{1,i} \equiv e_{2,i}$$

### 2.2 Constant-Time Formulation

A program $P$ is observationally constant-time if for all input valuations $\sigma_1, \sigma_2$:
$$(\forall x \in X_{\text{pub}}.\, \sigma_1(x) = \sigma_2(x)) \implies \text{Trace}(P, \sigma_1) \sim_{\text{obs}} \text{Trace}(P, \sigma_2)$$

### 2.3 The Self-Composition Miter

The constant-time property is compiled into a propositional miter formula:
$$\Phi_{\text{CT}} = PC(\sigma_1) \land PC(\sigma_2) \land \bigwedge_{x \in X_{\text{pub}}} (\sigma_1(x) = \sigma_2(x)) \land \neg(\text{Trace}(P, \sigma_1) \sim_{\text{obs}} \text{Trace}(P, \sigma_2))$$

- If $\Phi_{\text{CT}}$ is **UNSAT**, no pair of inputs agreeing on public data can produce differing observable behavior $\implies$ certified constant-time.
- If $\Phi_{\text{CT}}$ is **SAT**, the satisfying assignment yields a concrete pair of inputs $(\sigma_1, \sigma_2)$ that leak secret information $\implies$ counterexample with timing / cache-timing side-channel.

---

## 3. Vacuity Prevention (VER00 §7.6.3, VER04 §7)

A constant-time proof over a function with no secret inputs ($X_{\text{sec}} = \emptyset$) is vacuous: since $\sigma_1 = \sigma_2$ for all inputs, any deterministic program trivially satisfies $\tau_1 = \tau_2$.
- **Vacuity Rule:** The constant-time prover **MUST** reject verification attempts where $|X_{\text{sec}}| = 0$ with `ConstantTimeVerdict::VacuousProof`, preventing false claims of side-channel safety on unannotated code.

---

## 4. Resource Bounds and Ranking Functions

### 4.1 Step Bounds
A resource bound claim $\text{steps} \le B(n)$ for public input size $n$ is encoded by asserting that for all valid paths:
$$\bigwedge_{\text{paths } p} (PC_p \implies \text{Steps}(p) \le B(n))$$

The negation formula is checked for satisfiability:
$$\Phi_{\text{Bound}} = \bigvee_{\text{paths } p} (PC_p \land \text{Steps}(p) > B(n))$$
- **UNSAT** certifies that the step quota is respected along all admissible paths.
- **SAT** produces a concrete input exceeding the step bound (DoS vulnerability / algorithmic complexity attack, CWE-400).

### 4.2 Loop Unrolling and Honest Bounded Labels
When a loop depends on input-dependent bounds and loop unrolling is capped at bound $k$:
- If all paths exit before bound $k$, the proof is complete and unlabelled.
- If any path reaches the unrolling threshold $k$, the verdict MUST be explicitly labelled `Bounded(k)`.

---

## 5. Independent Proof Kernel Validation

All UNSAT verdicts emitted by the VER20 proof engine MUST be certified by independent proof checking:
1. CDCL SAT solver (`cdcl-sat`) generates a DRAT refutation trace during solving.
2. The verification engine passes the DIMACS CNF and the DRAT proof to `drat-check::check`.
3. If DRAT verification succeeds, the engine returns `ConstantTimeVerdict::Proved`.
4. If DRAT verification fails or cannot be produced, the engine returns `ConstantTimeVerdict::Unknown`, failing closed.

---

## 6. Implementation Architecture

The VER20 implementation in `warrant-ir` resides in `src/ct_proof.rs`:
- `ConstantTimeProver`: High-level self-composition driver managing dual symbolic executors and miter construction.
- `TraceComparator`: Circuit generator building event-by-event equivalence checks ($e_{1,i} == e_{2,i}$).
- `BoundProver`: Verification engine checking maximum step limits and termination bounds.
- Re-exported at crate level `coding_adventures_warrant_ir::ct_proof`.
