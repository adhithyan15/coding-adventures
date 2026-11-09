//! Resource-bound and constant-time trace proofs via self-composition (VER20, CWE-208, CWE-400).
//!
//! Evaluates 2-safety hyperproperties over WIR observable traces by tying public inputs
//! and exploring all secret permutations to certify absence of timing and address side-channels.

use std::collections::HashMap;

use bit_blaster::aig::{Aig, Edge, Node};
use bit_blaster::bv::{self, BitVec};
use bit_blaster::tseitin::Tseitin;
use cdcl_sat::{Cnf, SolveResult, Solver};

use crate::ast::{Function, Label, ProvenanceId, VarId, WirType};
use crate::sym::{
    ObservableSymEvent, SymbolicConfig, SymbolicExecutor, SymMemory, SymVal,
};

/// Classification of observable trace divergence between two self-composed runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DivergenceKind {
    /// Conditional branch decision difference (secret-dependent control flow).
    BranchDecision { index: usize },
    /// Memory dereference address difference (secret-dependent cache/memory access).
    MemoryAddress {
        index: usize,
        prov_1: ProvenanceId,
        prov_2: ProvenanceId,
    },
    /// Step cost difference between runs.
    StepCount { steps_1: usize, steps_2: usize },
    /// Number of observable events differs between runs.
    TraceLength { len_1: usize, len_2: usize },
    /// Termination divergence: one path trapped/crashed while the other completed.
    TerminationTrap { index: usize },
    /// Observable event kind mismatch between two paths.
    EventMismatch {
        index: usize,
        event_1: String,
        event_2: String,
    },
}

/// Verification verdict for 2-safety constant-time proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CtProofVerdict {
    /// Mathematically proved constant-time across all secret inputs with certified UNSAT.
    Proved {
        max_steps: usize,
        events_count: usize,
    },
    /// Proved constant-time up to loop unrolling bound `k`.
    ProvedBounded {
        bound: usize,
        max_steps: usize,
        events_count: usize,
    },
    /// Counterexample found exhibiting timing or address divergence (CWE-208).
    Falsified {
        divergence_kind: DivergenceKind,
        counterexample_1: HashMap<VarId, u128>,
        counterexample_2: HashMap<VarId, u128>,
    },
    /// Verification failed vacuity check (no secret inputs found).
    VacuousProof { reason: String },
    /// Verification unknown or solver limit exceeded.
    Unknown(String),
}

/// Type alias for compatibility with module-namespaced references.
pub type ConstantTimeVerdict = CtProofVerdict;

/// Verification verdict for resource bound checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundVerdict {
    /// Proved that execution steps <= bound across all paths with certified UNSAT.
    Proved {
        bound: usize,
        max_path_steps: usize,
    },
    /// Proved that execution steps <= bound up to unrolling bound `k`.
    ProvedBounded {
        unroll_bound: usize,
        max_path_steps: usize,
    },
    /// Execution exceeds bound along at least one feasible path.
    Exceeded {
        bound: usize,
        actual_steps: usize,
        counterexample: HashMap<VarId, u128>,
    },
    /// Verification unknown or solver error.
    Unknown(String),
}

/// A refutation accepted by the DRAT checker, kept so it can be written to disk
/// and hashed (VER04h §7.1).
///
/// `trivial` is true when the divergence miter simplified structurally to
/// `FALSE`: both runs then produce identical observable traces by construction
/// and the "refutation" is the fixed two-clause formula `x and not-x`.  That is
/// a valid certificate of the structural fact, but not a non-trivial SAT proof,
/// and reports must say which kind they hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofCertificate {
    pub trivial: bool,
    pub dimacs: String,
    pub drat: String,
}

/// Formal verification engine for constant-time 2-safety hyperproperties.
pub struct ConstantTimeProver {
    /// Shared circuit graph for both symbolic execution runs.
    pub aig: Aig,
    /// Configuration for bounded exploration.
    pub config: SymbolicConfig,
    /// Every refutation the DRAT kernel accepted, in order (VER04h §7.1).
    pub certificates: Vec<ProofCertificate>,
}

impl Default for ConstantTimeProver {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstantTimeProver {
    /// Create a new constant-time prover with default parameters.
    pub fn new() -> Self {
        Self::with_config(SymbolicConfig::default())
    }

    /// Create a new constant-time prover with specified parameters.
    pub fn with_config(config: SymbolicConfig) -> Self {
        Self {
            aig: Aig::new(),
            config,
            certificates: Vec::new(),
        }
    }

    /// Prove constant-time execution of `func` using parameter security labels.
    ///
    /// Parameters labeled `Label::PUBLIC` are identical between Run 1 and Run 2.
    /// Parameters labeled `Label::SECRET` vary independently across their full domain.
    pub fn prove_function(&mut self, func: &Function) -> ConstantTimeVerdict {
        // 1. Vacuity check: ensure at least one secret parameter exists
        let has_secret = func.params.iter().any(|(_, _, label)| *label == Label::SECRET);
        if !has_secret {
            return ConstantTimeVerdict::VacuousProof {
                reason: format!("Function '{}' has no parameters labeled SECRET", func.name),
            };
        }

        // 2. Construct inputs for Run 1 and Run 2
        let mut args_1 = Vec::new();
        let mut args_2 = Vec::new();
        let mut mem_1 = SymMemory::new();
        let mut mem_2 = SymMemory::new();
        let mut tracked_inputs_1 = HashMap::new();
        let mut tracked_inputs_2 = HashMap::new();

        for (var, ty, label) in &func.params {
            let bit_w = ty.bit_width().unwrap_or(64) as usize;
            if matches!(ty, WirType::Ptr(_)) {
                let (prov1, off1) = mem_1.allocate_symbolic(64, &mut self.aig);
                let (prov2, off2) = if *label == Label::PUBLIC {
                    (prov1, off1.clone())
                } else {
                    mem_2.allocate_symbolic(64, &mut self.aig)
                };
                args_1.push(SymVal::Ptr(prov1, off1));
                args_2.push(SymVal::Ptr(prov2, off2));
            } else if *label == Label::PUBLIC {
                // Shared input for both runs
                if *ty == WirType::Bool {
                    let b = BitVec::input(&mut self.aig, 1);
                    args_1.push(SymVal::Bool(b.bits[0]));
                    args_2.push(SymVal::Bool(b.bits[0]));
                    tracked_inputs_1.insert(var.clone(), b.clone());
                    tracked_inputs_2.insert(var.clone(), b);
                } else {
                    let bv = BitVec::input(&mut self.aig, bit_w);
                    args_1.push(SymVal::Bits(bv.clone()));
                    args_2.push(SymVal::Bits(bv.clone()));
                    tracked_inputs_1.insert(var.clone(), bv.clone());
                    tracked_inputs_2.insert(var.clone(), bv);
                }
            } else {
                // Independent inputs for Run 1 and Run 2
                if *ty == WirType::Bool {
                    let b1 = BitVec::input(&mut self.aig, 1);
                    let b2 = BitVec::input(&mut self.aig, 1);
                    args_1.push(SymVal::Bool(b1.bits[0]));
                    args_2.push(SymVal::Bool(b2.bits[0]));
                    tracked_inputs_1.insert(var.clone(), b1);
                    tracked_inputs_2.insert(var.clone(), b2);
                } else {
                    let bv1 = BitVec::input(&mut self.aig, bit_w);
                    let bv2 = BitVec::input(&mut self.aig, bit_w);
                    args_1.push(SymVal::Bits(bv1.clone()));
                    args_2.push(SymVal::Bits(bv2.clone()));
                    tracked_inputs_1.insert(var.clone(), bv1);
                    tracked_inputs_2.insert(var.clone(), bv2);
                }
            }
        }

        self.prove_constant_time_internal(
            func,
            args_1,
            mem_1,
            args_2,
            mem_2,
            Vec::new(),
            tracked_inputs_1,
            tracked_inputs_2,
        )
    }

    /// Prove constant-time execution with explicit inputs and initial memories for both runs.
    #[allow(clippy::too_many_arguments)]
    pub fn prove_with_custom_inputs(
        &mut self,
        func: &Function,
        args_1: Vec<SymVal>,
        mem_1: SymMemory,
        args_2: Vec<SymVal>,
        mem_2: SymMemory,
        public_equalities: Vec<Edge>,
        tracked_inputs_1: HashMap<VarId, BitVec>,
        tracked_inputs_2: HashMap<VarId, BitVec>,
    ) -> ConstantTimeVerdict {
        if args_1.is_empty()
            && args_2.is_empty()
            && tracked_inputs_1.is_empty()
            && tracked_inputs_2.is_empty()
            && public_equalities.is_empty()
        {
            return ConstantTimeVerdict::VacuousProof {
                reason: "Custom inputs contain no arguments or tracked symbolic variables".to_string(),
            };
        }
        self.prove_constant_time_internal(
            func,
            args_1,
            mem_1,
            args_2,
            mem_2,
            public_equalities,
            tracked_inputs_1,
            tracked_inputs_2,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn prove_constant_time_internal(
        &mut self,
        func: &Function,
        args_1: Vec<SymVal>,
        mem_1: SymMemory,
        args_2: Vec<SymVal>,
        mem_2: SymMemory,
        public_equalities: Vec<Edge>,
        tracked_inputs_1: HashMap<VarId, BitVec>,
        tracked_inputs_2: HashMap<VarId, BitVec>,
    ) -> ConstantTimeVerdict {
        // Execute Run 1
        let mut exec_1 = SymbolicExecutor::with_config(self.config.clone());
        exec_1.aig = std::mem::take(&mut self.aig);
        let res_1 = exec_1.execute_function(func, args_1, mem_1);

        // Execute Run 2 on the same AIG graph
        let mut exec_2 = SymbolicExecutor::with_config(self.config.clone());
        exec_2.aig = exec_1.aig;
        let res_2 = exec_2.execute_function(func, args_2, mem_2);
        self.aig = exec_2.aig;

        // Check if one run trapped while the other completed (secret-dependent crash oracle, CWE-208)
        match (&res_1, &res_2) {
            (Ok(_), Err(crate::sym::SymbolicError::Trap(_)))
            | (Err(crate::sym::SymbolicError::Trap(_)), Ok(_)) => {
                return ConstantTimeVerdict::Falsified {
                    divergence_kind: DivergenceKind::TerminationTrap { index: 0 },
                    counterexample_1: HashMap::new(),
                    counterexample_2: HashMap::new(),
                };
            }
            (Err(e), _) => {
                return ConstantTimeVerdict::Unknown(format!("Run 1 execution error: {}", e));
            }
            (_, Err(e)) => {
                return ConstantTimeVerdict::Unknown(format!("Run 2 execution error: {}", e));
            }
            (Ok(_), Ok(_)) => {}
        }

        let paths_1 = res_1.unwrap();
        let paths_2 = res_2.unwrap();

        if paths_1.is_empty() || paths_2.is_empty() {
            return ConstantTimeVerdict::VacuousProof {
                reason: "No feasible execution paths explored".to_string(),
            };
        }

        let mut any_bounded = false;
        let mut max_steps = 0;
        let mut events_count = 0;
        let mut miter_clauses = Vec::new();
        let mut divergence_map: Vec<(Edge, DivergenceKind)> = Vec::new();

        let pub_eq_edge = self.aig.and_all(&public_equalities);
        if pub_eq_edge == Edge::FALSE {
            return ConstantTimeVerdict::VacuousProof {
                reason: "Public input equality constraints are contradictory".to_string(),
            };
        }

        // Cross-compare each path pair (p1, p2)
        for p1 in &paths_1 {
            let pc1 = self.aig.and_all(&p1.path_condition);
            if pc1 == Edge::FALSE {
                continue;
            }
            if p1.bounded {
                any_bounded = true;
            }
            max_steps = std::cmp::max(max_steps, p1.steps);
            events_count = std::cmp::max(events_count, p1.observable_events.len());

            for p2 in &paths_2 {
                let pc2 = self.aig.and_all(&p2.path_condition);
                if pc2 == Edge::FALSE {
                    continue;
                }
                if p2.bounded {
                    any_bounded = true;
                }

                let base_pc = self.aig.and(pc1, pc2);
                if base_pc == Edge::FALSE {
                    continue;
                }
                let joint_pc = self.aig.and(base_pc, pub_eq_edge);
                if joint_pc == Edge::FALSE {
                    continue;
                }

                // Check 1: Step count difference
                if p1.steps != p2.steps {
                    miter_clauses.push(joint_pc);
                    divergence_map.push((
                        joint_pc,
                        DivergenceKind::StepCount {
                            steps_1: p1.steps,
                            steps_2: p2.steps,
                        },
                    ));
                    continue;
                }

                // Check 2: Observable event count difference
                if p1.observable_events.len() != p2.observable_events.len() {
                    miter_clauses.push(joint_pc);
                    divergence_map.push((
                        joint_pc,
                        DivergenceKind::TraceLength {
                            len_1: p1.observable_events.len(),
                            len_2: p2.observable_events.len(),
                        },
                    ));
                    continue;
                }

                // Check 3: Observable event equivalence
                for (idx, (ev1, ev2)) in p1
                    .observable_events
                    .iter()
                    .zip(&p2.observable_events)
                    .enumerate()
                {
                    match (ev1, ev2) {
                        (
                            ObservableSymEvent::Branch {
                                cond: _c1,
                                taken: t1,
                            },
                            ObservableSymEvent::Branch {
                                cond: _c2,
                                taken: t2,
                            },
                        ) => {
                            if t1 != t2 {
                                // Branches took different directions!
                                miter_clauses.push(joint_pc);
                                divergence_map.push((
                                    joint_pc,
                                    DivergenceKind::BranchDecision { index: idx },
                                ));
                                break;
                            }
                        }
                        (
                            ObservableSymEvent::Load {
                                provenance: prov1,
                                offset: off1,
                                size: s1,
                            },
                            ObservableSymEvent::Load {
                                provenance: prov2,
                                offset: off2,
                                size: s2,
                            },
                        )
                        | (
                            ObservableSymEvent::Store {
                                provenance: prov1,
                                offset: off1,
                                size: s1,
                            },
                            ObservableSymEvent::Store {
                                provenance: prov2,
                                offset: off2,
                                size: s2,
                            },
                        ) => {
                            if prov1 != prov2 || s1 != s2 {
                                miter_clauses.push(joint_pc);
                                divergence_map.push((
                                    joint_pc,
                                    DivergenceKind::MemoryAddress {
                                        index: idx,
                                        prov_1: *prov1,
                                        prov_2: *prov2,
                                    },
                                ));
                                break;
                            } else {
                                // Addresses must be equivalent
                                let eq_addr = bv::eq(&mut self.aig, off1, off2);
                                let ne_addr = !eq_addr;
                                let addr_diff = self.aig.and(joint_pc, ne_addr);
                                if addr_diff != Edge::FALSE {
                                    miter_clauses.push(addr_diff);
                                    divergence_map.push((
                                        addr_diff,
                                        DivergenceKind::MemoryAddress {
                                            index: idx,
                                            prov_1: *prov1,
                                            prov_2: *prov2,
                                        },
                                    ));
                                }
                            }
                        }
                        _ => {
                            // Mismatched event kinds at same trace index
                            miter_clauses.push(joint_pc);
                            divergence_map.push((
                                joint_pc,
                                DivergenceKind::EventMismatch {
                                    index: idx,
                                    event_1: format!("{:?}", ev1),
                                    event_2: format!("{:?}", ev2),
                                },
                            ));
                            break;
                        }
                    }
                }
            }
        }

        let overall_miter = if miter_clauses.is_empty() {
            Edge::FALSE
        } else {
            self.aig.or_all(&miter_clauses)
        };

        // If no divergence formula exists or it collapses to FALSE, constant-time holds across all traces.
        // Formally certify via CDCL SAT and DRAT proof checker (VER20 §5).
        if overall_miter == Edge::FALSE {
            let mut cnf = Cnf::new(1);
            cnf.add_clause(&[cdcl_sat::Lit::from_dimacs(1).unwrap()]);
            cnf.add_clause(&[cdcl_sat::Lit::from_dimacs(-1).unwrap()]);
            let mut solver = Solver::from_cnf_with_proof(&cnf);
            if let Some(budget) = self.config.solver_conflict_budget {
                solver.set_conflict_budget(Some(budget));
            }
            match solver.solve() {
                SolveResult::Unsat => {
                    let dimacs = cdcl_sat::write_dimacs(&cnf);
                    let proof_text = match solver.proof() {
                        Some(p) => p.to_drat_text(),
                        None => {
                            return ConstantTimeVerdict::Unknown(
                                "Proof kernel produced no DRAT proof".to_string(),
                            );
                        }
                    };
                    if let Err(e) = drat_check::check(&dimacs, &proof_text) {
                        return ConstantTimeVerdict::Unknown(format!("DRAT proof rejected: {}", e));
                    }
                    self.certificates.push(ProofCertificate {
                        trivial: true,
                        dimacs: dimacs.clone(),
                        drat: proof_text.clone(),
                    });
                    return if any_bounded {
                        ConstantTimeVerdict::ProvedBounded {
                            bound: self.config.unroll_bound,
                            max_steps,
                            events_count,
                        }
                    } else {
                        ConstantTimeVerdict::Proved {
                            max_steps,
                            events_count,
                        }
                    };
                }
                SolveResult::Unknown => {
                    return ConstantTimeVerdict::Unknown(
                        "SAT solver resource budget exhausted (CWE-400)".to_string(),
                    );
                }
                _ => {
                    return ConstantTimeVerdict::Unknown(
                        "Proof kernel verification failed".to_string(),
                    );
                }
            }
        }

        // Encode to CNF via Tseitin
        let mut cnf = Cnf::new(0);
        let mut tseitin = Tseitin::new();
        tseitin.assert_true(&self.aig, overall_miter, &mut cnf);

        // Solve with cdcl-sat with DRAT proof logging
        let mut solver = Solver::from_cnf_with_proof(&cnf);
        if let Some(budget) = self.config.solver_conflict_budget {
            solver.set_conflict_budget(Some(budget));
        }

        match solver.solve() {
            SolveResult::Unsat => {
                // Certified UNSAT: validate with drat-check
                let dimacs = cdcl_sat::write_dimacs(&cnf);
                let proof_text = match solver.proof() {
                    Some(p) => p.to_drat_text(),
                    None => {
                        return ConstantTimeVerdict::Unknown(
                            "Solver reported UNSAT but no DRAT proof was produced".to_string(),
                        );
                    }
                };

                match drat_check::check(&dimacs, &proof_text) {
                    Ok(_) => {
                        self.certificates.push(ProofCertificate {
                            trivial: false,
                            dimacs: dimacs.clone(),
                            drat: proof_text.clone(),
                        });
                        if any_bounded {
                            ConstantTimeVerdict::ProvedBounded {
                                bound: self.config.unroll_bound,
                                max_steps,
                                events_count,
                            }
                        } else {
                            ConstantTimeVerdict::Proved {
                                max_steps,
                                events_count,
                            }
                        }
                    }
                    Err(e) => ConstantTimeVerdict::Unknown(format!("DRAT proof rejected: {}", e)),
                }
            }
            SolveResult::Sat => {
                // Falsified: reconstruct concrete counterexample from SAT model
                let model = solver.model();
                let mut ce_1 = HashMap::new();
                let mut ce_2 = HashMap::new();

                for (var, bv) in tracked_inputs_1 {
                    let mut val = 0u128;
                    for (i, edge) in bv.bits.iter().enumerate() {
                        if let Node::Input(input_idx) = self.aig.node(edge.node()) {
                            if let Some(b) = tseitin.input_value(&self.aig, input_idx, model) {
                                if b ^ edge.is_complemented() && i < 128 {
                                    val |= 1 << i;
                                }
                            }
                        }
                    }
                    ce_1.insert(var, val);
                }

                for (var, bv) in tracked_inputs_2 {
                    let mut val = 0u128;
                    for (i, edge) in bv.bits.iter().enumerate() {
                        if let Node::Input(input_idx) = self.aig.node(edge.node()) {
                            if let Some(b) = tseitin.input_value(&self.aig, input_idx, model) {
                                if b ^ edge.is_complemented() && i < 128 {
                                    val |= 1 << i;
                                }
                            }
                        }
                    }
                    ce_2.insert(var, val);
                }

                let input_assignments: Vec<bool> = (0..self.aig.num_inputs())
                    .map(|i| tseitin.input_value(&self.aig, i, model).unwrap_or(false))
                    .collect();

                let kind = divergence_map
                    .iter()
                    .find_map(|(edge, k)| {
                        if self.aig.eval(*edge, &input_assignments) {
                            Some(k.clone())
                        } else {
                            None
                        }
                    })
                    .or_else(|| divergence_map.first().map(|(_, k)| k.clone()))
                    .unwrap_or(DivergenceKind::BranchDecision { index: 0 });

                ConstantTimeVerdict::Falsified {
                    divergence_kind: kind,
                    counterexample_1: ce_1,
                    counterexample_2: ce_2,
                }
            }
            SolveResult::Unknown => {
                ConstantTimeVerdict::Unknown("SAT solver resource budget exhausted (CWE-400)".to_string())
            }
        }
    }
}

/// Prover for verifying abstract instruction step bounds.
pub struct BoundProver {
    pub executor: SymbolicExecutor,
    /// Every refutation the DRAT kernel accepted, in order (VER04h §7.1).
    pub certificates: Vec<ProofCertificate>,
}

impl Default for BoundProver {
    fn default() -> Self {
        Self::new()
    }
}

impl BoundProver {
    /// Create a new bound prover with default configuration.
    pub fn new() -> Self {
        Self::with_config(SymbolicConfig::default())
    }

    /// Create a new bound prover with specified configuration.
    pub fn with_config(config: SymbolicConfig) -> Self {
        Self {
            executor: SymbolicExecutor::with_config(config),
            certificates: Vec::new(),
        }
    }

    /// Prove that `func` executes in at most `bound` steps across all parameter inputs.
    pub fn prove_function_bound(&mut self, func: &Function, bound: usize) -> BoundVerdict {
        let mut args = Vec::new();
        for (var, ty, _) in &func.params {
            let bit_w = ty.bit_width().unwrap_or(64) as usize;
            let bv = self.executor.create_input_bits(var.0.as_str(), bit_w);
            args.push(SymVal::Bits(bv));
        }
        self.prove_step_bound(func, args, SymMemory::new(), bound)
    }

    /// Prove that `func` executes in at most `bound` steps along all paths.
    pub fn prove_step_bound(
        &mut self,
        func: &Function,
        args: Vec<SymVal>,
        initial_memory: SymMemory,
        bound: usize,
    ) -> BoundVerdict {
        let outcomes = match self.executor.execute_function(func, args, initial_memory) {
            Ok(o) => o,
            Err(crate::sym::SymbolicError::StepLimitExceeded(max_s)) => {
                if max_s >= bound {
                    return BoundVerdict::Exceeded {
                        bound,
                        actual_steps: max_s,
                        counterexample: HashMap::new(),
                    };
                } else {
                    return BoundVerdict::Unknown(format!("Symbolic execution step limit exceeded: {}", max_s));
                }
            }
            Err(e) => return BoundVerdict::Unknown(format!("Symbolic execution error: {}", e)),
        };

        let mut max_steps = 0;
        let mut any_bounded = false;
        let mut violation_clauses = Vec::new();
        let mut violation_steps = Vec::new();

        for outcome in outcomes {
            if outcome.bounded {
                any_bounded = true;
            }
            max_steps = std::cmp::max(max_steps, outcome.steps);

            if outcome.steps > bound {
                // Step limit exceeded along this path: record path condition
                let pc = self.executor.aig.and_all(&outcome.path_condition);
                violation_clauses.push(pc);
                violation_steps.push(outcome.steps);
            }
        }

        let miter = if violation_clauses.is_empty() {
            Edge::FALSE
        } else {
            self.executor.aig.or_all(&violation_clauses)
        };

        if miter == Edge::FALSE {
            let mut cnf = Cnf::new(1);
            cnf.add_clause(&[cdcl_sat::Lit::from_dimacs(1).unwrap()]);
            cnf.add_clause(&[cdcl_sat::Lit::from_dimacs(-1).unwrap()]);
            let mut solver = Solver::from_cnf_with_proof(&cnf);
            if let Some(budget) = self.executor.config.solver_conflict_budget {
                solver.set_conflict_budget(Some(budget));
            }
            match solver.solve() {
                SolveResult::Unsat => {
                    let dimacs = cdcl_sat::write_dimacs(&cnf);
                    let proof_text = match solver.proof() {
                        Some(p) => p.to_drat_text(),
                        None => {
                            return BoundVerdict::Unknown("No DRAT proof generated for UNSAT".to_string());
                        }
                    };
                    if let Err(e) = drat_check::check(&dimacs, &proof_text) {
                        return BoundVerdict::Unknown(format!("DRAT proof rejected: {}", e));
                    }
                    self.certificates.push(ProofCertificate {
                        trivial: true,
                        dimacs: dimacs.clone(),
                        drat: proof_text.clone(),
                    });
                    return if any_bounded {
                        BoundVerdict::ProvedBounded {
                            unroll_bound: self.executor.config.unroll_bound,
                            max_path_steps: max_steps,
                        }
                    } else {
                        BoundVerdict::Proved {
                            bound,
                            max_path_steps: max_steps,
                        }
                    };
                }
                SolveResult::Unknown => {
                    return BoundVerdict::Unknown("SAT solver resource budget exhausted (CWE-400)".to_string());
                }
                _ => return BoundVerdict::Unknown("Proof kernel verification failed".to_string()),
            }
        }

        let mut cnf = Cnf::new(0);
        let mut tseitin = Tseitin::new();
        tseitin.assert_true(&self.executor.aig, miter, &mut cnf);

        let mut solver = Solver::from_cnf_with_proof(&cnf);
        if let Some(budget) = self.executor.config.solver_conflict_budget {
            solver.set_conflict_budget(Some(budget));
        }

        match solver.solve() {
            SolveResult::Unsat => {
                let dimacs = cdcl_sat::write_dimacs(&cnf);
                let proof_text = match solver.proof() {
                    Some(p) => p.to_drat_text(),
                    None => {
                        return BoundVerdict::Unknown("No DRAT proof generated for UNSAT".to_string());
                    }
                };

                match drat_check::check(&dimacs, &proof_text) {
                    Ok(_) => {
                        self.certificates.push(ProofCertificate {
                            trivial: false,
                            dimacs: dimacs.clone(),
                            drat: proof_text.clone(),
                        });
                        if any_bounded {
                            BoundVerdict::ProvedBounded {
                                unroll_bound: self.executor.config.unroll_bound,
                                max_path_steps: max_steps,
                            }
                        } else {
                            BoundVerdict::Proved {
                                bound,
                                max_path_steps: max_steps,
                            }
                        }
                    }
                    Err(e) => BoundVerdict::Unknown(format!("DRAT proof rejected: {}", e)),
                }
            }
            SolveResult::Sat => {
                let model = solver.model();
                let mut ce = HashMap::new();
                for (var, bv) in &self.executor.input_vars {
                    let mut val = 0u128;
                    for (i, edge) in bv.bits.iter().enumerate() {
                        if let Node::Input(input_idx) = self.executor.aig.node(edge.node()) {
                            if let Some(b) = tseitin.input_value(&self.executor.aig, input_idx, model) {
                                if b ^ edge.is_complemented() && i < 128 {
                                    val |= 1 << i;
                                }
                            }
                        }
                    }
                    ce.insert(VarId::from(var.as_str()), val);
                }

                let input_assignments: Vec<bool> = (0..self.executor.aig.num_inputs())
                    .map(|i| tseitin.input_value(&self.executor.aig, i, model).unwrap_or(false))
                    .collect();

                let actual_steps = violation_clauses
                    .iter()
                    .zip(&violation_steps)
                    .find_map(|(edge, steps)| {
                        if self.executor.aig.eval(*edge, &input_assignments) {
                            Some(*steps)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(max_steps);

                BoundVerdict::Exceeded {
                    bound,
                    actual_steps,
                    counterexample: ce,
                }
            }
            SolveResult::Unknown => BoundVerdict::Unknown("SAT solver resource budget exhausted (CWE-400)".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;
    use crate::SymAlloc;

    #[test]
    fn test_constant_time_sat_solver_unsat_path() {
        // Proves constant time via SAT solver & DRAT check:
        // A secret-dependent load is located behind an impossible branch condition (sec > 10 && sec < 5).
        // The miter clause for the diverging path is sent to cdcl-sat, which proves it UNSAT with DRAT proof.
        let mut prover = ConstantTimeProver::new();

        let func = Function {
            name: "impossible_branch_leak".to_string(),
            params: vec![(VarId::from("sec"), WirType::U64, Label::SECRET)],
            return_type: WirType::Unit,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    instructions: vec![
                        WirInst::Alloc {
                            var: VarId::from("buf"),
                            size_expr: WirExpr::ConstInt { val: 64, ty: WirType::U64 },
                        },
                        WirInst::Let {
                            var: VarId::from("gt10"),
                            ty: WirType::Bool,
                            label: Label::SECRET,
                            expr: WirExpr::Binary {
                                op: BinaryOp::Gt,
                                lhs: Box::new(WirExpr::Var(VarId::from("sec"))),
                                rhs: Box::new(WirExpr::ConstInt { val: 10, ty: WirType::U64 }),
                            },
                        },
                        WirInst::Let {
                            var: VarId::from("lt5"),
                            ty: WirType::Bool,
                            label: Label::SECRET,
                            expr: WirExpr::Binary {
                                op: BinaryOp::Lt,
                                lhs: Box::new(WirExpr::Var(VarId::from("sec"))),
                                rhs: Box::new(WirExpr::ConstInt { val: 5, ty: WirType::U64 }),
                            },
                        },
                        WirInst::Let {
                            var: VarId::from("impossible"),
                            ty: WirType::Bool,
                            label: Label::SECRET,
                            expr: WirExpr::Binary {
                                op: BinaryOp::BitAnd,
                                lhs: Box::new(WirExpr::Var(VarId::from("gt10"))),
                                rhs: Box::new(WirExpr::Var(VarId::from("lt5"))),
                            },
                        },
                    ],
                    terminator: WirTerminator::Branch {
                        cond: WirExpr::Var(VarId::from("impossible")),
                        then_block: BlockId(1),
                        else_block: BlockId(2),
                    },
                },
                BasicBlock {
                    id: BlockId(1),
                    instructions: vec![
                        WirInst::Let {
                            var: VarId::from("ptr"),
                            ty: WirType::U64,
                            label: Label::SECRET,
                            expr: WirExpr::PtrOffset {
                                ptr: Box::new(WirExpr::Var(VarId::from("buf"))),
                                offset_bytes: Box::new(WirExpr::Var(VarId::from("sec"))),
                            },
                        },
                        WirInst::Load {
                            var: VarId::from("val"),
                            ptr_expr: WirExpr::Var(VarId::from("ptr")),
                            ty: WirType::U8,
                        },
                    ],
                    terminator: WirTerminator::Return(None),
                },
                BasicBlock {
                    id: BlockId(2),
                    instructions: vec![],
                    terminator: WirTerminator::Return(None),
                },
            ],
            ranking_metric: None,
        };

        let verdict = prover.prove_function(&func);
        match verdict {
            CtProofVerdict::Proved { max_steps, .. } => {
                assert!(max_steps > 0);
            }
            other => panic!("Expected Proved via SAT UNSAT, got {:?}", other),
        }
    }

    #[test]
    fn test_bound_prover_sat_solver_unsat_path() {
        // Step bound violation condition is unreachable due to contradictory condition (x > 10 && x < 5)
        let mut bound_prover = BoundProver::new();
        let x_bv = bound_prover.executor.create_input_bits("x", 64);

        let func = Function {
            name: "unreachable_heavy_path".to_string(),
            params: vec![(VarId::from("x"), WirType::U64, Label::PUBLIC)],
            return_type: WirType::Unit,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    instructions: vec![
                        WirInst::Let {
                            var: VarId::from("gt10"),
                            ty: WirType::Bool,
                            label: Label::PUBLIC,
                            expr: WirExpr::Binary {
                                op: BinaryOp::Gt,
                                lhs: Box::new(WirExpr::Var(VarId::from("x"))),
                                rhs: Box::new(WirExpr::ConstInt { val: 10, ty: WirType::U64 }),
                            },
                        },
                        WirInst::Let {
                            var: VarId::from("lt5"),
                            ty: WirType::Bool,
                            label: Label::PUBLIC,
                            expr: WirExpr::Binary {
                                op: BinaryOp::Lt,
                                lhs: Box::new(WirExpr::Var(VarId::from("x"))),
                                rhs: Box::new(WirExpr::ConstInt { val: 5, ty: WirType::U64 }),
                            },
                        },
                        WirInst::Let {
                            var: VarId::from("impossible"),
                            ty: WirType::Bool,
                            label: Label::PUBLIC,
                            expr: WirExpr::Binary {
                                op: BinaryOp::BitAnd,
                                lhs: Box::new(WirExpr::Var(VarId::from("gt10"))),
                                rhs: Box::new(WirExpr::Var(VarId::from("lt5"))),
                            },
                        },
                    ],
                    terminator: WirTerminator::Branch {
                        cond: WirExpr::Var(VarId::from("impossible")),
                        then_block: BlockId(1),
                        else_block: BlockId(2),
                    },
                },
                BasicBlock {
                    id: BlockId(1),
                    instructions: vec![
                        // 10 dummy instructions along impossible path
                        WirInst::Let { var: VarId::from("d1"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 1, ty: WirType::U64 } },
                        WirInst::Let { var: VarId::from("d2"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 2, ty: WirType::U64 } },
                        WirInst::Let { var: VarId::from("d3"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 3, ty: WirType::U64 } },
                        WirInst::Let { var: VarId::from("d4"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 4, ty: WirType::U64 } },
                    ],
                    terminator: WirTerminator::Return(None),
                },
                BasicBlock {
                    id: BlockId(2),
                    instructions: vec![],
                    terminator: WirTerminator::Return(None),
                },
            ],
            ranking_metric: None,
        };

        // Normal path takes 4 steps (3 lets + branch). Impossible path takes 8 steps.
        // Bound is 5. So impossible path violates bound (8 > 5), but solver proves it UNSAT!
        let verdict = bound_prover.prove_step_bound(&func, vec![SymVal::Bits(x_bv)], SymMemory::new(), 5);
        match verdict {
            BoundVerdict::Proved { bound, max_path_steps } => {
                assert_eq!(bound, 5);
                assert!(max_path_steps <= 8);
            }
            other => panic!("Expected Proved via SAT UNSAT, got {:?}", other),
        }
    }

    #[test]
    fn test_ct_prover_all_divergence_branches_and_errors() {
        let mut prover = ConstantTimeProver::default();

        // 1. Run 2 error (Run 1 succeeds, Run 2 fails)
        let r1_args = vec![SymVal::Bits(BitVec::constant(64, 1))];
        let r2_args = vec![SymVal::Tuple(vec![])]; // type mismatch when expecting int
        let r2_func = Function {
            name: "r2_func".to_string(),
            params: vec![(VarId::from("p"), WirType::U64, Label::SECRET)],
            return_type: WirType::U64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![WirInst::Let {
                    var: VarId::from("out"),
                    ty: WirType::U64,
                    label: Label::SECRET,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Add(OverflowMode::Wrap),
                        lhs: Box::new(WirExpr::Var(VarId::from("p"))),
                        rhs: Box::new(WirExpr::ConstInt { val: 1, ty: WirType::U64 }),
                    },
                }],
                terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("out")))),
            }],
            ranking_metric: None,
        };
        let r2_err = prover.prove_with_custom_inputs(
            &r2_func,
            r1_args,
            SymMemory::new(),
            r2_args,
            SymMemory::new(),
            Vec::new(),
            HashMap::new(),
            HashMap::new(),
        );
        assert!(matches!(r2_err, CtProofVerdict::Unknown(msg) if msg.contains("Run 2 execution error")));

        // 1b. Public int parameter and secret int parameter in prove_function
        let pub_int_func = Function {
            name: "pub_and_sec_int".to_string(),
            params: vec![
                (VarId::from("pub_i"), WirType::U64, Label::PUBLIC),
                (VarId::from("sec_i"), WirType::U64, Label::SECRET),
            ],
            return_type: WirType::U64,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![WirInst::Let {
                    var: VarId::from("res"),
                    ty: WirType::U64,
                    label: Label::SECRET,
                    expr: WirExpr::Binary {
                        op: BinaryOp::Add(OverflowMode::Wrap),
                        lhs: Box::new(WirExpr::Var(VarId::from("pub_i"))),
                        rhs: Box::new(WirExpr::Var(VarId::from("sec_i"))),
                    },
                }],
                terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("res")))),
            }],
            ranking_metric: None,
        };
        let pub_int_res = prover.prove_function(&pub_int_func);
        assert!(matches!(pub_int_res, CtProofVerdict::Proved { .. }));

        // 2. Memory provenance divergence
        let prov1 = ProvenanceId(10);
        let prov2 = ProvenanceId(20);
        let args1 = vec![SymVal::Ptr(prov1, BitVec::constant(64, 0))];
        let args2 = vec![SymVal::Ptr(prov2, BitVec::constant(64, 0))];
        let mut mem1 = SymMemory::new();
        mem1.allocations.insert(prov1, SymAlloc { base: prov1, size: 8, bytes: vec![BitVec::constant(8, 0); 8] });
        let mut mem2 = SymMemory::new();
        mem2.allocations.insert(prov2, SymAlloc { base: prov2, size: 8, bytes: vec![BitVec::constant(8, 0); 8] });

        let prov_div_func = Function {
            name: "prov_div".to_string(),
            params: vec![(VarId::from("ptr"), WirType::U64, Label::SECRET)],
            return_type: WirType::U8,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![WirInst::Load {
                    var: VarId::from("v"),
                    ptr_expr: WirExpr::Var(VarId::from("ptr")),
                    ty: WirType::U8,
                }],
                terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("v")))),
            }],
            ranking_metric: None,
        };

        let prov_verdict = prover.prove_with_custom_inputs(
            &prov_div_func,
            args1,
            mem1,
            args2,
            mem2,
            Vec::new(),
            HashMap::new(),
            HashMap::new(),
        );
        assert!(matches!(prov_verdict, CtProofVerdict::Falsified { divergence_kind: DivergenceKind::MemoryAddress { .. }, .. }));

        // 3. Trace length and mismatched event kinds coverage
        let bp_default = BoundProver::default();
        assert_eq!(bp_default.executor.config.max_paths, 256);

        let mut mem_mismatch1 = SymMemory::new();
        let (prov_m1, off_m1) = mem_mismatch1.allocate(8, Some(0));
        let mut mem_mismatch2 = SymMemory::new();
        let (prov_m2, off_m2) = mem_mismatch2.allocate(8, Some(0));

        let mismatch_func = Function {
            name: "mismatch_events".to_string(),
            params: vec![
                (VarId::from("is_branch"), WirType::Bool, Label::SECRET),
                (VarId::from("ptr"), WirType::U64, Label::PUBLIC),
            ],
            return_type: WirType::Unit,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    instructions: vec![],
                    terminator: WirTerminator::Branch {
                        cond: WirExpr::Var(VarId::from("is_branch")),
                        then_block: BlockId(1),
                        else_block: BlockId(2),
                    },
                },
                BasicBlock {
                    id: BlockId(1),
                    instructions: vec![WirInst::Load {
                        var: VarId::from("x"),
                        ptr_expr: WirExpr::Var(VarId::from("ptr")),
                        ty: WirType::U8,
                    }],
                    terminator: WirTerminator::Return(None),
                },
                BasicBlock {
                    id: BlockId(2),
                    instructions: vec![],
                    terminator: WirTerminator::Return(None),
                },
            ],
            ranking_metric: None,
        };

        let m_res = prover.prove_with_custom_inputs(
            &mismatch_func,
            vec![SymVal::Bool(Edge::TRUE), SymVal::Ptr(prov_m1, off_m1)],
            mem_mismatch1,
            vec![SymVal::Bool(Edge::FALSE), SymVal::Ptr(prov_m2, off_m2)],
            mem_mismatch2,
            Vec::new(),
            HashMap::new(),
            HashMap::new(),
        );
        assert!(matches!(m_res, CtProofVerdict::Falsified { .. }));

        // 4. Equal steps but different observable event count
        // Path 1: 1 load (1 step, 1 event)
        // Path 2: 1 let constant (1 step, 0 events)
        let eq_steps_diff_events = Function {
            name: "eq_steps_diff_events".to_string(),
            params: vec![
                (VarId::from("flag"), WirType::Bool, Label::SECRET),
                (VarId::from("ptr"), WirType::U64, Label::PUBLIC),
            ],
            return_type: WirType::U8,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    instructions: vec![],
                    terminator: WirTerminator::Branch {
                        cond: WirExpr::Var(VarId::from("flag")),
                        then_block: BlockId(1),
                        else_block: BlockId(2),
                    },
                },
                BasicBlock {
                    id: BlockId(1),
                    instructions: vec![WirInst::Load {
                        var: VarId::from("v"),
                        ptr_expr: WirExpr::Var(VarId::from("ptr")),
                        ty: WirType::U8,
                    }],
                    terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("v")))),
                },
                BasicBlock {
                    id: BlockId(2),
                    instructions: vec![WirInst::Let {
                        var: VarId::from("c"),
                        ty: WirType::U8,
                        label: Label::SECRET,
                        expr: WirExpr::ConstInt { val: 7, ty: WirType::U8 },
                    }],
                    terminator: WirTerminator::Return(Some(WirExpr::Var(VarId::from("c")))),
                },
            ],
            ranking_metric: None,
        };
        let mut mem_e1 = SymMemory::new();
        let (p_e1, o_e1) = mem_e1.allocate(4, Some(0));
        let mut mem_e2 = SymMemory::new();
        let (p_e2, o_e2) = mem_e2.allocate(4, Some(0));
        let diff_events_res = prover.prove_with_custom_inputs(
            &eq_steps_diff_events,
            vec![SymVal::Bool(Edge::TRUE), SymVal::Ptr(p_e1, o_e1)],
            mem_e1,
            vec![SymVal::Bool(Edge::FALSE), SymVal::Ptr(p_e2, o_e2)],
            mem_e2,
            Vec::new(),
            HashMap::new(),
            HashMap::new(),
        );
        assert!(matches!(diff_events_res, CtProofVerdict::Falsified { divergence_kind: DivergenceKind::TraceLength { .. }, .. }));

        // 5. Mismatched event kind at index 1: Path 1 has Load, Path 2 has Branch
        let mut mem_ev = SymMemory::new();
        let (p_ev, o_ev) = mem_ev.allocate(4, Some(0));
        let event_mismatch_func = Function {
            name: "event_mismatch".to_string(),
            params: vec![
                (VarId::from("b"), WirType::Bool, Label::SECRET),
                (VarId::from("ptr"), WirType::U64, Label::PUBLIC),
            ],
            return_type: WirType::Unit,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    instructions: vec![],
                    terminator: WirTerminator::Branch {
                        cond: WirExpr::Var(VarId::from("b")),
                        then_block: BlockId(1),
                        else_block: BlockId(2),
                    },
                },
                BasicBlock {
                    id: BlockId(1),
                    instructions: vec![WirInst::Load {
                        var: VarId::from("v"),
                        ptr_expr: WirExpr::Var(VarId::from("ptr")),
                        ty: WirType::U8,
                    }],
                    terminator: WirTerminator::Return(None),
                },
                BasicBlock {
                    id: BlockId(2),
                    instructions: vec![WirInst::Let {
                        var: VarId::from("dummy"),
                        ty: WirType::U8,
                        label: Label::PUBLIC,
                        expr: WirExpr::ConstInt { val: 0, ty: WirType::U8 },
                    }],
                    terminator: WirTerminator::Branch {
                        cond: WirExpr::ConstBool(true),
                        then_block: BlockId(3),
                        else_block: BlockId(3),
                    },
                },
                BasicBlock {
                    id: BlockId(3),
                    instructions: vec![],
                    terminator: WirTerminator::Return(None),
                },
            ],
            ranking_metric: None,
        };
        let em_res = prover.prove_with_custom_inputs(
            &event_mismatch_func,
            vec![SymVal::Bool(Edge::TRUE), SymVal::Ptr(p_ev, o_ev.clone())],
            mem_ev.clone(),
            vec![SymVal::Bool(Edge::FALSE), SymVal::Ptr(p_ev, o_ev)],
            mem_ev,
            Vec::new(),
            HashMap::new(),
            HashMap::new(),
        );
        assert!(matches!(em_res, CtProofVerdict::Falsified { .. }));

        // 6. ProvedBounded in SAT UNSAT branch
        let mut prover_b = ConstantTimeProver::with_config(SymbolicConfig {
            unroll_bound: 0,
            ..SymbolicConfig::default()
        });
        let infinite_loop_func = Function {
            name: "infinite_loop".to_string(),
            params: vec![(VarId::from("s"), WirType::U64, Label::SECRET)],
            return_type: WirType::Unit,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: WirTerminator::Jump(BlockId(0)),
            }],
            ranking_metric: None,
        };
        let bounded_unsat_verdict = prover_b.prove_function(&infinite_loop_func);
        assert!(matches!(bounded_unsat_verdict, CtProofVerdict::ProvedBounded { bound: 0, .. }));

        // 7. ProvedBounded in BoundProver SAT UNSAT branch
        let mut bp_b = BoundProver::with_config(SymbolicConfig {
            unroll_bound: 0,
            ..SymbolicConfig::default()
        });
        let x_b = bp_b.executor.create_input_bits("x", 64);
        let heavy_func = Function {
            name: "heavy".to_string(),
            params: vec![(VarId::from("x"), WirType::U64, Label::PUBLIC)],
            return_type: WirType::Unit,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    instructions: vec![
                        WirInst::Let { var: VarId::from("g"), ty: WirType::Bool, label: Label::PUBLIC, expr: WirExpr::Binary { op: BinaryOp::Gt, lhs: Box::new(WirExpr::Var(VarId::from("x"))), rhs: Box::new(WirExpr::ConstInt { val: 10, ty: WirType::U64 }) } },
                        WirInst::Let { var: VarId::from("l"), ty: WirType::Bool, label: Label::PUBLIC, expr: WirExpr::Binary { op: BinaryOp::Lt, lhs: Box::new(WirExpr::Var(VarId::from("x"))), rhs: Box::new(WirExpr::ConstInt { val: 5, ty: WirType::U64 }) } },
                        WirInst::Let { var: VarId::from("imp"), ty: WirType::Bool, label: Label::PUBLIC, expr: WirExpr::Binary { op: BinaryOp::BitAnd, lhs: Box::new(WirExpr::Var(VarId::from("g"))), rhs: Box::new(WirExpr::Var(VarId::from("l"))) } },
                    ],
                    terminator: WirTerminator::Branch {
                        cond: WirExpr::Var(VarId::from("imp")),
                        then_block: BlockId(1),
                        else_block: BlockId(2),
                    },
                },
                BasicBlock {
                    id: BlockId(1),
                    instructions: vec![
                        WirInst::Let { var: VarId::from("d1"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 1, ty: WirType::U64 } },
                        WirInst::Let { var: VarId::from("d2"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 2, ty: WirType::U64 } },
                        WirInst::Let { var: VarId::from("d3"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 3, ty: WirType::U64 } },
                        WirInst::Let { var: VarId::from("d4"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 4, ty: WirType::U64 } },
                    ],
                    terminator: WirTerminator::Return(None),
                },
                BasicBlock {
                    id: BlockId(2),
                    instructions: vec![],
                    terminator: WirTerminator::Return(None),
                },
            ],
            ranking_metric: None,
        };
        let bp_bounded_res = bp_b.prove_step_bound(&heavy_func, vec![SymVal::Bits(x_b)], SymMemory::new(), 5);
        assert!(matches!(bp_bounded_res, BoundVerdict::ProvedBounded { unroll_bound: 0, .. }));

        // 8. miter == Edge::FALSE paths in BoundProver and ConstantTimeProver
        let mut prover_false = ConstantTimeProver::new();
        let false_ct = prover_false.prove_constant_time_internal(
            &pub_int_func,
            vec![SymVal::Bits(BitVec::constant(64, 0)), SymVal::Bits(BitVec::constant(64, 0))],
            SymMemory::new(),
            vec![SymVal::Bits(BitVec::constant(64, 0)), SymVal::Bits(BitVec::constant(64, 0))],
            SymMemory::new(),
            vec![Edge::FALSE], // causes joint_pc to be Edge::FALSE, so miter_clauses has Edge::FALSE
            HashMap::new(),
            HashMap::new(),
        );
        assert!(matches!(false_ct, CtProofVerdict::VacuousProof { .. }));

        // 10. Step count divergence
        let step_div_func = Function {
            name: "step_div".to_string(),
            params: vec![(VarId::from("s"), WirType::Bool, Label::SECRET)],
            return_type: WirType::Unit,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    instructions: vec![],
                    terminator: WirTerminator::Branch {
                        cond: WirExpr::Var(VarId::from("s")),
                        then_block: BlockId(1),
                        else_block: BlockId(2),
                    },
                },
                BasicBlock {
                    id: BlockId(1),
                    instructions: vec![WirInst::Let {
                        var: VarId::from("x"),
                        ty: WirType::U64,
                        label: Label::PUBLIC,
                        expr: WirExpr::ConstInt { val: 1, ty: WirType::U64 },
                    }],
                    terminator: WirTerminator::Return(None),
                },
                BasicBlock {
                    id: BlockId(2),
                    instructions: vec![],
                    terminator: WirTerminator::Return(None),
                },
            ],
            ranking_metric: None,
        };
        let sc_res = prover.prove_function(&step_div_func);
        assert!(matches!(
            sc_res,
            CtProofVerdict::Falsified {
                divergence_kind: DivergenceKind::BranchDecision { .. } | DivergenceKind::StepCount { .. },
                ..
            }
        ));
        let mut bp_false = BoundProver::new();
        let false_bound_func = Function {
            name: "false_bound".to_string(),
            params: vec![],
            return_type: WirType::Unit,
            blocks: vec![
                BasicBlock {
                    id: BlockId(0),
                    instructions: vec![],
                    terminator: WirTerminator::Branch {
                        cond: WirExpr::ConstBool(false),
                        then_block: BlockId(1),
                        else_block: BlockId(2),
                    },
                },
                BasicBlock {
                    id: BlockId(1),
                    instructions: vec![
                        WirInst::Let { var: VarId::from("d1"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 1, ty: WirType::U64 } },
                        WirInst::Let { var: VarId::from("d2"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 2, ty: WirType::U64 } },
                        WirInst::Let { var: VarId::from("d3"), ty: WirType::U64, label: Label::PUBLIC, expr: WirExpr::ConstInt { val: 3, ty: WirType::U64 } },
                    ],
                    terminator: WirTerminator::Return(None),
                },
                BasicBlock {
                    id: BlockId(2),
                    instructions: vec![],
                    terminator: WirTerminator::Return(None),
                },
            ],
            ranking_metric: None,
        };
        // Path 1 has 3 steps (> 1), but condition is statically FALSE, so pc is Edge::FALSE
        let fb_res = bp_false.prove_step_bound(&false_bound_func, vec![], SymMemory::new(), 1);
        assert!(matches!(fb_res, BoundVerdict::Proved { .. }));
    }
}
