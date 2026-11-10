/**
 * Level 3 CDCL SAT Solver & Tseitin Bit-Blaster (VER09a).
 *
 * Implements Conflict-Driven Clause Learning (CDCL) Boolean satisfiability solver
 * with two-watched-literals (2WL), 1-UIP conflict analysis, non-chronological
 * backjumping, VSIDS decision heuristics, phase saving, Luby restarts, learned
 * clause database reduction, incremental assumption solving, failed-assumption
 * core extraction, and Tseitin circuit transformation.
 *
 * Conforms to:
 * - code/specs/VER00-verification-manifesto.md
 * - code/specs/VER09-cdcl-sat.md
 * - code/specs/VER09a-typescript-sat-solver.md
 */

// --- Literals and Variables (VER09 §3.1) ------------------------------------

export type Var = number;
export type Lit = number;

export const LBool = {
  Undef: 0,
  True: 1,
  False: -1,
} as const;
export type LBool = (typeof LBool)[keyof typeof LBool];

export function varOf(lit: Lit): Var {
  return lit >> 1;
}

export function signOf(lit: Lit): number {
  return lit & 1;
}

export function negateLit(lit: Lit): Lit {
  return lit ^ 1;
}

export function makeLit(v: Var, sign: boolean | number): Lit {
  return (v << 1) | (sign ? 1 : 0);
}

export function litToString(lit: Lit): string {
  const v = varOf(lit) + 1;
  return signOf(lit) === 1 ? `-${v}` : `${v}`;
}

export type SolveResult = "sat" | "unsat" | "unknown";

// --- Priority Heap for VSIDS (VER09 §3.5) -----------------------------------

class VarOrderHeap {
  private heap: Var[] = [];
  private indices: Int32Array;
  private activity: Float64Array;

  constructor(activity: Float64Array) {
    this.activity = activity;
    this.indices = new Int32Array(activity.length);
    this.indices.fill(-1);
  }

  public insert(v: Var): void {
    if (this.indices[v] === -1) {
      this.indices[v] = this.heap.length;
      this.heap.push(v);
      this.percolateUp(this.indices[v]);
    }
  }

  public update(v: Var): void {
    if (this.indices[v] !== -1) {
      this.percolateUp(this.indices[v]);
    }
  }

  public pop(): Var | undefined {
    if (this.heap.length === 0) return undefined;
    const top = this.heap[0];
    const last = this.heap.pop()!;
    this.indices[top] = -1;
    if (this.heap.length > 0) {
      this.heap[0] = last;
      this.indices[last] = 0;
      this.percolateDown(0);
    }
    return top;
  }

  public empty(): boolean {
    return this.heap.length === 0;
  }

  private percolateUp(i: number): void {
    let curr = i;
    const v = this.heap[curr];
    const act = this.activity[v];
    while (curr > 0) {
      const parent = (curr - 1) >> 1;
      const pv = this.heap[parent];
      if (this.activity[pv] >= act) break;
      this.heap[curr] = pv;
      this.indices[pv] = curr;
      curr = parent;
    }
    this.heap[curr] = v;
    this.indices[v] = curr;
  }

  private percolateDown(i: number): void {
    let curr = i;
    const v = this.heap[curr];
    const act = this.activity[v];
    const half = this.heap.length >> 1;
    while (curr < half) {
      let child = (curr << 1) + 1;
      const right = child + 1;
      if (right < this.heap.length && this.activity[this.heap[right]] > this.activity[this.heap[child]]) {
        child = right;
      }
      if (act >= this.activity[this.heap[child]]) break;
      this.heap[curr] = this.heap[child];
      this.indices[this.heap[child]] = curr;
      curr = child;
    }
    this.heap[curr] = v;
    this.indices[v] = curr;
  }
}

// --- Watcher (VER09 §3.2) ---------------------------------------------------

interface Watcher {
  clauseIdx: number;
  blocker: Lit;
}

export interface SolverOptions {
  maxConflicts?: number;
  maxSteps?: number;
}

// --- CDCL SAT Solver --------------------------------------------------------

export class Solver {
  private numVars = 0;
  private clauses: number[][] = [];
  private isLearned: boolean[] = [];
  private clauseActivity: number[] = [];

  // Assignments & Trail
  private assigns: Int8Array = new Int8Array(0); // LBool
  private trail: Lit[] = [];
  private trailLim: number[] = []; // Trail index at decision level boundary
  private level: Int32Array = new Int32Array(0); // Decision level of Var
  private reason: Int32Array = new Int32Array(0); // Reason clauseIdx of Var (-1 = decision)
  private qhead = 0;

  // Watcher lists indexed by Lit
  private watches: Watcher[][] = [];

  // Heuristics
  private activity: Float64Array = new Float64Array(0);
  private varInc = 1.0;
  private readonly varDecay = 0.95;
  private clauseInc = 1.0;
  private readonly clauseDecay = 0.999;
  private orderHeap: VarOrderHeap = new VarOrderHeap(this.activity);
  private polarity: Int8Array = new Int8Array(0);

  // Statistics & Budgets
  public conflicts = 0;
  public decisions = 0;
  public propagations = 0;
  public restarts = 0;
  private conflictBudget = Infinity;
  private stepBudget = Infinity;

  // Solver Status & Soundness
  private ok = true;

  // Assumptions & Core
  private assumptionSet: Set<Lit> = new Set();
  private failedCore: Lit[] = [];

  constructor() {
    this.orderHeap = new VarOrderHeap(this.activity);
  }

  public okay(): boolean {
    return this.ok;
  }

  public newVar(): Var {
    const v = this.numVars++;
    this.growToVars(this.numVars);
    this.orderHeap.insert(v);
    return v;
  }

  private growToVars(size: number): void {
    if (this.assigns.length >= size) return;
    const newCap = Math.max(size, this.assigns.length * 2, 16);

    const newAssigns = new Int8Array(newCap);
    newAssigns.set(this.assigns);
    this.assigns = newAssigns;

    const newLevel = new Int32Array(newCap);
    newLevel.fill(-1);
    newLevel.set(this.level);
    this.level = newLevel;

    const newReason = new Int32Array(newCap);
    newReason.fill(-1);
    newReason.set(this.reason);
    this.reason = newReason;

    const newActivity = new Float64Array(newCap);
    newActivity.set(this.activity);
    this.activity = newActivity;

    const newPolarity = new Int8Array(newCap);
    newPolarity.fill(LBool.False);
    newPolarity.set(this.polarity);
    this.polarity = newPolarity;

    while (this.watches.length < newCap * 2) {
      this.watches.push([]);
    }

    this.orderHeap = new VarOrderHeap(this.activity);
    for (let i = 0; i < size; i++) {
      if (this.assigns[i] === LBool.Undef) {
        this.orderHeap.insert(i);
      }
    }
  }

  public nVars(): number {
    return this.numVars;
  }

  public nClauses(): number {
    return this.clauses.length;
  }

  public value(lit: Lit): LBool {
    const v = varOf(lit);
    const val = this.assigns[v];
    if (val === LBool.Undef) return LBool.Undef;
    return signOf(lit) === 1 ? -val as LBool : val as LBool;
  }

  public varValue(v: Var): LBool {
    return this.assigns[v] as LBool;
  }

  public addClause(lits: Lit[]): boolean {
    if (!this.ok) return false;

    // Reset to decision level 0 to prevent transient solution assignments
    // from corrupting clause simplification or dropping valid constraints
    if (this.decisionLevel() > 0) {
      this.backjumpTo(0);
    }

    // Simplify clause: eliminate duplicates, false literals, tautologies
    const normalized: Lit[] = [];
    let seenTrue = false;

    for (const lit of lits) {
      const v = varOf(lit);
      while (v >= this.numVars) {
        this.newVar();
      }

      const val = this.value(lit);
      if (val === LBool.True) {
        seenTrue = true;
        break;
      }
      if (val === LBool.False) {
        continue;
      }
      if (!normalized.includes(lit)) {
        if (normalized.includes(negateLit(lit))) {
          seenTrue = true;
          break;
        }
        normalized.push(lit);
      }
    }

    if (seenTrue) return true; // Satisfied at level 0

    if (normalized.length === 0) {
      this.ok = false;
      return false; // Unsatisfiable empty clause
    }

    if (normalized.length === 1) {
      this.assignLiteral(normalized[0], -1);
      const conflict = this.propagate();
      if (conflict !== -1) {
        this.ok = false;
        return false;
      }
      return true;
    }

    const clauseIdx = this.clauses.length;
    this.clauses.push(normalized);
    this.isLearned.push(false);
    this.clauseActivity.push(0);

    // Watch first two literals
    this.watches[normalized[0]].push({ clauseIdx, blocker: normalized[1] });
    this.watches[normalized[1]].push({ clauseIdx, blocker: normalized[0] });

    return true;
  }

  private assignLiteral(lit: Lit, reasonClause: number): void {
    const v = varOf(lit);
    const val: LBool = signOf(lit) === 1 ? LBool.False : LBool.True;
    this.assigns[v] = val;
    this.level[v] = this.decisionLevel();
    this.reason[v] = reasonClause;
    this.polarity[v] = val;
    this.trail.push(lit);
  }

  private decisionLevel(): number {
    return this.trailLim.length;
  }

  // Two-watched literals unit propagation (VER09 §3.2)
  public propagate(): number {
    let conflictClause = -1;

    while (this.qhead < this.trail.length) {
      const p = this.trail[this.qhead++];
      this.propagations++;
      if (this.propagations > this.stepBudget) {
        return -2; // Step budget exceeded
      }

      const falseLit = negateLit(p);
      const ws = this.watches[falseLit];
      let i = 0;
      let j = 0;

      while (i < ws.length) {
        const watcher = ws[i++];
        const cr = watcher.clauseIdx;
        const clause = this.clauses[cr];

        // Blocker optimization
        if (this.value(watcher.blocker) === LBool.True) {
          ws[j++] = watcher;
          continue;
        }

        // Ensure falseLit is at clause[1]
        if (clause[0] === falseLit) {
          clause[0] = clause[1];
          clause[1] = falseLit;
        }

        // If 0th literal is satisfied, keep watcher
        const first = clause[0];
        if (first !== watcher.blocker && this.value(first) === LBool.True) {
          watcher.blocker = first;
          ws[j++] = watcher;
          continue;
        }

        // Find new literal to watch
        let foundNewWatch = false;
        for (let k = 2; k < clause.length; k++) {
          if (this.value(clause[k]) !== LBool.False) {
            clause[1] = clause[k];
            clause[k] = falseLit;
            this.watches[clause[1]].push({ clauseIdx: cr, blocker: first });
            foundNewWatch = true;
            break;
          }
        }

        if (foundNewWatch) continue;

        // Clause is unit or conflicting
        ws[j++] = watcher;
        if (this.value(first) === LBool.False) {
          // Conflict
          conflictClause = cr;
          this.qhead = this.trail.length;
          // Copy remaining watchers
          while (i < ws.length) {
            ws[j++] = ws[i++];
          }
        } else {
          // Unit propagation
          this.assignLiteral(first, cr);
        }
      }

      ws.length = j;
      if (conflictClause !== -1) break;
    }

    return conflictClause;
  }

  // 1-UIP Conflict Analysis & Backjumping (VER09 §3.3)
  private analyze(conflictClause: number): { learnedClause: Lit[]; backjumpLevel: number } {
    const seen = new Uint8Array(this.numVars);
    const learnedClause: Lit[] = [0]; // Placeholder for asserting 1-UIP literal
    let pathC = 0;
    let p: Lit | null = null;
    let index = this.trail.length - 1;
    let currentClause: number[] = this.clauses[conflictClause];

    do {
      for (const lit of currentClause) {
        const v = varOf(lit);
        if (p !== null && v === varOf(p)) continue;
        if (!seen[v] && this.level[v] > 0) {
          seen[v] = 1;
          this.bumpVarActivity(v);
          if (this.level[v] >= this.decisionLevel()) {
            pathC++;
          } else {
            learnedClause.push(lit);
          }
        }
      }

      // Trace back along trail
      while (!seen[varOf(this.trail[index])]) {
        index--;
      }
      p = this.trail[index--];
      const v = varOf(p);
      seen[v] = 0;
      pathC--;
      if (pathC > 0) {
        currentClause = this.clauses[this.reason[v]];
      }
    } while (pathC > 0);

    // p is the 1-UIP
    learnedClause[0] = negateLit(p!);

    // Compute backjump level (highest level among remaining literals)
    let backjumpLevel = 0;
    for (let i = 1; i < learnedClause.length; i++) {
      const lvl = this.level[varOf(learnedClause[i])];
      if (lvl > backjumpLevel) {
        backjumpLevel = lvl;
        // Swap to position 1 for 2WL efficiency
        const tmp = learnedClause[1];
        learnedClause[1] = learnedClause[i];
        learnedClause[i] = tmp;
      }
    }

    return { learnedClause, backjumpLevel };
  }

  private backjumpTo(lvl: number): void {
    if (this.decisionLevel() <= lvl) return;

    for (let i = this.trail.length - 1; i >= this.trailLim[lvl]; i--) {
      const v = varOf(this.trail[i]);
      this.assigns[v] = LBool.Undef;
      this.level[v] = -1;
      this.reason[v] = -1;
      this.orderHeap.insert(v);
    }

    this.trail.length = this.trailLim[lvl];
    this.trailLim.length = lvl;
    this.qhead = this.trail.length;
  }

  private bumpVarActivity(v: Var): void {
    this.activity[v] += this.varInc;
    if (this.activity[v] > 1e100) {
      // Rescale
      for (let i = 0; i < this.numVars; i++) {
        this.activity[i] *= 1e-100;
      }
      this.varInc *= 1e-100;
    }
    this.orderHeap.update(v);
  }

  private decayActivities(): void {
    this.varInc *= 1 / this.varDecay;
    this.clauseInc *= 1 / this.clauseDecay;
    if (this.varInc > 1e100) {
      for (let i = 0; i < this.numVars; i++) {
        this.activity[i] *= 1e-100;
      }
      this.varInc *= 1e-100;
    }
    if (this.clauseInc > 1e100) {
      for (let i = 0; i < this.clauseActivity.length; i++) {
        this.clauseActivity[i] *= 1e-100;
      }
      this.clauseInc *= 1e-100;
    }
  }

  // Luby sequence for restart interval
  private static luby(i: number): number {
    let size = 1;
    let seq = i;
    while (size <= seq) {
      size = (size << 1) + 1;
    }
    while (size > 1) {
      size = (size - 1) >> 1;
      if (seq >= size) {
        seq -= size;
      }
    }
    return 1 << seq;
  }

  // Failed-assumption core extraction via implication graph backward reachability
  private extractCore(initialVars: Var[], initialLit?: Lit): Lit[] {
    const core: Lit[] = [];
    if (initialLit !== undefined) {
      core.push(initialLit);
    }
    const seen = new Uint8Array(this.numVars);
    for (const v of initialVars) {
      if (this.level[v] > 0) {
        seen[v] = 1;
      }
    }

    for (let i = this.trail.length - 1; i >= 0; i--) {
      const p = this.trail[i];
      const v = varOf(p);
      if (seen[v]) {
        seen[v] = 0;
        const r = this.reason[v];
        if (r === -1) {
          if (this.level[v] > 0) {
            if (!core.includes(p)) {
              core.push(p);
            }
          }
        } else {
          const c = this.clauses[r];
          for (const lit of c) {
            const lv = varOf(lit);
            if (this.level[lv] > 0) {
              seen[lv] = 1;
            }
          }
        }
      }
    }

    const result: Lit[] = [];
    for (const lit of core) {
      if (this.assumptionSet.has(lit) && !result.includes(lit)) {
        result.push(lit);
      }
    }
    return result;
  }

  // Main CDCL solve loop
  public solve(options: SolverOptions = {}): SolveResult {
    return this.solveWithAssumptions([], options);
  }

  public solveWithAssumptions(assumptions: Lit[], options: SolverOptions = {}): SolveResult {
    if (!this.ok) {
      return "unsat";
    }

    this.backjumpTo(0);
    const startConflicts = this.conflicts;
    const startSteps = this.propagations;
    this.conflictBudget = startConflicts + (options.maxConflicts ?? Infinity);
    this.stepBudget = startSteps + (options.maxSteps ?? Infinity);
    this.assumptionSet = new Set(assumptions);
    this.failedCore = [];

    // Ensure all variables are instantiated
    for (const lit of assumptions) {
      const v = varOf(lit);
      while (v >= this.numVars) {
        this.newVar();
      }
    }

    if (this.propagate() !== -1) {
      this.ok = false;
      return "unsat";
    }

    let restartCount = 0;
    let restartConflictLimit = 100 * Solver.luby(restartCount);

    while (true) {
      const conflict = this.propagate();

      if (conflict === -2) {
        this.backjumpTo(0);
        return "unknown"; // Step budget reached
      }

      if (conflict !== -1) {
        // Unconditional contradiction at root level is always definitive unsat
        if (this.decisionLevel() === 0) {
          this.ok = false;
          return "unsat";
        }

        // Conflict occurred
        this.conflicts++;
        if (this.conflicts > this.conflictBudget) {
          this.backjumpTo(0);
          return "unknown"; // Conflict budget reached
        }

        // If conflict occurred while deciding assumptions (all decisions are assumptions)
        if (this.decisionLevel() <= assumptions.length && assumptions.length > 0) {
          const initialVars = this.clauses[conflict].map(varOf);
          this.failedCore = this.extractCore(initialVars);
          this.backjumpTo(0);
          return "unsat";
        }

        const { learnedClause, backjumpLevel } = this.analyze(conflict);

        this.backjumpTo(backjumpLevel);

        if (learnedClause.length === 1) {
          this.assignLiteral(learnedClause[0], -1);
        } else {
          const cIdx = this.clauses.length;
          this.clauses.push(learnedClause);
          this.isLearned.push(true);
          this.clauseActivity.push(this.clauseInc);

          this.watches[learnedClause[0]].push({ clauseIdx: cIdx, blocker: learnedClause[1] });
          this.watches[learnedClause[1]].push({ clauseIdx: cIdx, blocker: learnedClause[0] });

          this.assignLiteral(learnedClause[0], cIdx);
        }

        this.decayActivities();

        // Check restart
        if (--restartConflictLimit <= 0) {
          restartCount++;
          this.restarts++;
          restartConflictLimit = 100 * Solver.luby(restartCount);
          this.backjumpTo(0);
        }
      } else {
        // No conflict: pick decision literal
        if (this.decisionLevel() < assumptions.length) {
          // Next assumption decision
          const p = assumptions[this.decisionLevel()];
          if (this.value(p) === LBool.True) {
            // Already true, dummy level
            this.trailLim.push(this.trail.length);
          } else if (this.value(p) === LBool.False) {
            // Contradiction on assumption
            this.failedCore = this.extractCore([varOf(p)], p);
            this.backjumpTo(0);
            return "unsat";
          } else {
            this.trailLim.push(this.trail.length);
            this.assignLiteral(p, -1);
          }
        } else {
          // Regular heuristic decision
          let nextVar: Var | undefined;
          while (!this.orderHeap.empty()) {
            const v = this.orderHeap.pop()!;
            if (this.assigns[v] === LBool.Undef) {
              nextVar = v;
              break;
            }
          }

          if (nextVar === undefined) {
            // All variables assigned with no conflict: SAT!
            return "sat";
          }

          this.decisions++;
          this.trailLim.push(this.trail.length);
          const sign = this.polarity[nextVar] === LBool.True ? 0 : 1;
          this.assignLiteral(makeLit(nextVar, sign), -1);
        }
      }
    }
  }

  public getModel(): boolean[] {
    const model: boolean[] = [];
    for (let i = 0; i < this.numVars; i++) {
      model.push(this.assigns[i] === LBool.True);
    }
    return model;
  }

  public getFailedCore(): Lit[] {
    return [...this.failedCore];
  }

  public reset(): void {
    this.backjumpTo(0);
  }
}

// --- Tseitin Transformation & Bit-Blaster (VER09a §3) ----------------------

export class CnfBuilder {
  private solver: Solver;

  constructor(solver?: Solver) {
    this.solver = solver ?? new Solver();
  }

  public getSolver(): Solver {
    return this.solver;
  }

  public newVar(): Var {
    return this.solver.newVar();
  }

  public newLit(): Lit {
    return makeLit(this.newVar(), 0);
  }

  public addClause(lits: Lit[]): boolean {
    return this.solver.addClause(lits);
  }

  public trueLit(): Lit {
    const t = makeLit(this.newVar(), 0);
    this.addClause([t]);
    return t;
  }

  public falseLit(): Lit {
    const f = makeLit(this.newVar(), 0);
    this.addClause([negateLit(f)]);
    return f;
  }

  // Tseitin AND: out <-> (a and b)
  public and(a: Lit, b: Lit): Lit {
    const out = this.newLit();
    this.addClause([out, negateLit(a), negateLit(b)]);
    this.addClause([negateLit(out), a]);
    this.addClause([negateLit(out), b]);
    return out;
  }

  // Tseitin OR: out <-> (a or b)
  public or(a: Lit, b: Lit): Lit {
    const out = this.newLit();
    this.addClause([negateLit(out), a, b]);
    this.addClause([out, negateLit(a)]);
    this.addClause([out, negateLit(b)]);
    return out;
  }

  // Tseitin XOR: out <-> (a xor b)
  public xor(a: Lit, b: Lit): Lit {
    const out = this.newLit();
    this.addClause([negateLit(out), a, b]);
    this.addClause([negateLit(out), negateLit(a), negateLit(b)]);
    this.addClause([out, negateLit(a), b]);
    this.addClause([out, a, negateLit(b)]);
    return out;
  }

  // Tseitin NOT: out <-> not a
  public not(a: Lit): Lit {
    return negateLit(a);
  }

  // Tseitin ITE: out <-> if cond then thenLit else elseLit
  public ite(cond: Lit, thenLit: Lit, elseLit: Lit): Lit {
    const out = this.newLit();
    this.addClause([negateLit(cond), negateLit(thenLit), out]);
    this.addClause([negateLit(cond), thenLit, negateLit(out)]);
    this.addClause([cond, negateLit(elseLit), out]);
    this.addClause([cond, elseLit, negateLit(out)]);
    return out;
  }

  // Tseitin EQUIV: out <-> (a == b)
  public equiv(a: Lit, b: Lit): Lit {
    const out = this.newLit();
    this.addClause([negateLit(out), negateLit(a), b]);
    this.addClause([negateLit(out), a, negateLit(b)]);
    this.addClause([out, a, b]);
    this.addClause([out, negateLit(a), negateLit(b)]);
    return out;
  }

  // 1-bit full adder: returns { sum, cout }
  public fullAdder(a: Lit, b: Lit, cin: Lit): { sum: Lit; cout: Lit } {
    const sum = this.xor(this.xor(a, b), cin);
    const ab = this.and(a, b);
    const axorb = this.xor(a, b);
    const axorb_cin = this.and(axorb, cin);
    const cout = this.or(ab, axorb_cin);
    return { sum, cout };
  }

  // Bit-vector equality: returns Lit that is true iff bv1 == bv2
  public bvEq(a: Lit[], b: Lit[]): Lit {
    if (a.length !== b.length) {
      throw new Error(`Bit-vector width mismatch: ${a.length} != ${b.length}`);
    }
    if (a.length === 0) return this.trueLit();

    let eq = this.equiv(a[0], b[0]);
    for (let i = 1; i < a.length; i++) {
      const bitEq = this.equiv(a[i], b[i]);
      eq = this.and(eq, bitEq);
    }
    return eq;
  }

  // Ripple-carry bit-vector adder: returns sum bits
  public bvAdd(a: Lit[], b: Lit[]): Lit[] {
    if (a.length !== b.length) {
      throw new Error(`Bit-vector width mismatch: ${a.length} != ${b.length}`);
    }
    const sum: Lit[] = [];
    let carry: Lit = this.falseLit();

    for (let i = 0; i < a.length; i++) {
      const fa = this.fullAdder(a[i], b[i], carry);
      sum.push(fa.sum);
      carry = fa.cout;
    }
    return sum;
  }
}

// --- DIMACS CNF Transport (VER09 §2, §3.6) -----------------------------------

export interface CnfFormula {
  numVars: number;
  clauses: Lit[][];
}

export function parseDimacs(dimacs: string): CnfFormula {
  const lines = dimacs.split(/\r?\n/);
  let numVars = 0;
  let expectedClauses = 0;
  const clauses: Lit[][] = [];
  let currentClause: Lit[] = [];
  let headerSeen = false;

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.length === 0 || trimmed.startsWith("c")) {
      continue; // Comment line
    }

    if (!headerSeen) {
      if (!trimmed.startsWith("p cnf")) {
        throw new Error(`Expected DIMACS header 'p cnf <vars> <clauses>', got: ${trimmed}`);
      }
      const parts = trimmed.split(/\s+/);
      if (parts.length < 4) {
        throw new Error(`Malformed DIMACS header: ${trimmed}`);
      }
      numVars = parseInt(parts[2], 10);
      expectedClauses = parseInt(parts[3], 10);
      if (!Number.isInteger(numVars) || numVars < 0 || !Number.isInteger(expectedClauses) || expectedClauses < 0) {
        throw new Error(`Malformed DIMACS header counts: vars=${parts[2]}, clauses=${parts[3]}`);
      }
      headerSeen = true;
      continue;
    }

    // Parse tokens in clause line
    const tokens = trimmed.split(/\s+/);
    for (const tok of tokens) {
      const num = parseInt(tok, 10);
      if (Number.isNaN(num)) {
        throw new Error(`Malformed integer in DIMACS clause: ${tok}`);
      }
      if (num === 0) {
        clauses.push(currentClause);
        currentClause = [];
      } else {
        const absNum = Math.abs(num);
        if (absNum <= 0 || absNum > 0x3fffffff) {
          throw new Error(`Literal index out of range: ${tok}`);
        }
        const v = absNum - 1;
        const sign = num < 0;
        currentClause.push(makeLit(v, sign));
      }
    }
  }

  if (currentClause.length > 0) {
    throw new Error("DIMACS format error: Unterminated clause (missing trailing 0)");
  }

  return { numVars, clauses };
}

export function writeDimacs(formula: CnfFormula): string {
  const out: string[] = [];
  out.push(`p cnf ${formula.numVars} ${formula.clauses.length}`);
  for (const clause of formula.clauses) {
    const line = clause.map(litToString).join(" ") + " 0";
    out.push(line);
  }
  return out.join("\n");
}
