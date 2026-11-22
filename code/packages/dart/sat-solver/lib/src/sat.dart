/// Level 3 CDCL SAT solver & Tseitin bit-blaster: the Dart lane of VER09d.
///
/// Conflict-Driven Clause Learning over packed 32-bit literals, with
/// two-watched-literal propagation, 1-UIP conflict analysis, VSIDS decisions,
/// phase saving, Luby restarts, learned-clause reduction, incremental
/// assumption solving with failed-core extraction, and Tseitin circuit
/// builders with bit-vector primitives.
///
/// Behavioral port of the TypeScript VER09a engine including the VER-P043
/// repair semantics (pre-dequeue budget checks, sticky root conflicts,
/// assumption-local cores, strict four-token DIMACS).
///
/// ```dart
/// import 'package:coding_adventures_sat_solver/coding_adventures_sat_solver.dart';
///
/// void main() {
///   final solver = Solver();
///   final a = makeLit(solver.newVar(), false);
///   final b = makeLit(solver.newVar(), false);
///   solver.addClause([a, b]);
///   solver.addClause([negateLit(a)]);
///   print(solver.solve()); // sat
///   print(solver.getModel()); // [false, true]
/// }
/// ```
library sat;

import 'dart:typed_data';

// ─── Literals and Variables (VER09 §3.1) ────────────────────────────────────
//
// A literal packs a variable index and a sign bit into one int:
//   lit = (v << 1) | (negated ? 1 : 0)
// so negation is a single XOR and both parts extract with one shift/mask.

/// A variable index (`0..N-1`).
typedef Var = int;

/// A packed literal: `(variable << 1) | sign`.
typedef Lit = int;

/// Three-valued logic: true, false, or unassigned.
abstract final class LBool {
  /// Unassigned.
  static const int undef = 0;

  /// True.
  static const int true_ = 1;

  /// False.
  static const int false_ = -1;
}

/// Extracts the variable from [lit].
int varOf(int lit) => lit >> 1;

/// Extracts the sign from [lit] (`1` = negated, `0` = positive).
int signOf(int lit) => lit & 1;

/// Negates [lit].
int negateLit(int lit) => lit ^ 1;

/// Packs variable [v] with [negated] sign into a literal.
int makeLit(int v, bool negated) => (v << 1) | (negated ? 1 : 0);

/// Renders [lit] in DIMACS signed form (`-6`, `6`).
String litToString(int lit) {
  final v = varOf(lit) + 1;
  return signOf(lit) == 1 ? '-$v' : '$v';
}

/// Solver verdicts.
abstract final class SolveResult {
  /// Satisfiable; see [Solver.getModel].
  static const String sat = 'sat';

  /// Unsatisfiable; see [Solver.getFailedCore].
  static const String unsat = 'unsat';

  /// Budgets exhausted before a verdict.
  static const String unknown = 'unknown';
}

// ─── Priority Heap for VSIDS (VER09 §3.5) ───────────────────────────────────
//
// Binary max-heap over variable activity with an index array for O(log n)
// updates. Ties keep insertion order (strict `>` when sifting), matching the
// sibling lanes' decision order.

class _VarOrderHeap {
  final List<int> _heap = [];
  Int32List _indices;
  final Float64List _activity;

  _VarOrderHeap(this._activity)
      : _indices = Int32List(_activity.length)
          ..fillRange(0, _activity.length, -1);

  void insert(int v) {
    if (_indices[v] == -1) {
      _indices[v] = _heap.length;
      _heap.add(v);
      _percolateUp(_indices[v]);
    }
  }

  void update(int v) {
    if (_indices[v] != -1) {
      _percolateUp(_indices[v]);
    }
  }

  int? pop() {
    if (_heap.isEmpty) return null;
    final top = _heap[0];
    final last = _heap.removeLast();
    _indices[top] = -1;
    if (_heap.isNotEmpty) {
      _heap[0] = last;
      _indices[last] = 0;
      _percolateDown(0);
    }
    return top;
  }

  bool get isEmpty => _heap.isEmpty;

  void _percolateUp(int i) {
    var curr = i;
    final v = _heap[curr];
    final act = _activity[v];
    while (curr > 0) {
      final parent = (curr - 1) >> 1;
      final pv = _heap[parent];
      if (_activity[pv] >= act) break;
      _heap[curr] = pv;
      _indices[pv] = curr;
      curr = parent;
    }
    _heap[curr] = v;
    _indices[v] = curr;
  }

  void _percolateDown(int i) {
    var curr = i;
    final v = _heap[curr];
    final act = _activity[v];
    final half = _heap.length >> 1;
    while (curr < half) {
      var child = (curr << 1) + 1;
      final right = child + 1;
      if (right < _heap.length &&
          _activity[_heap[right]] > _activity[_heap[child]]) {
        child = right;
      }
      if (act >= _activity[_heap[child]]) break;
      _heap[curr] = _heap[child];
      _indices[_heap[child]] = curr;
      curr = child;
    }
    _heap[curr] = v;
    _indices[v] = curr;
  }
}

// ─── Watcher (VER09 §3.2) ───────────────────────────────────────────────────

/// A clause watching one literal, with a blocker fast-path literal.
class _Watcher {
  /// Index into the clause database.
  final int clauseIdx;

  /// Other watched literal; skips the clause visit when already true.
  int blocker;
  _Watcher(this.clauseIdx, this.blocker);
}

/// Per-solve resource budgets.
class SolverOptions {
  /// Maximum new conflicts before `unknown` (default unbounded).
  final num? maxConflicts;

  /// Maximum new propagation steps before `unknown` (default unbounded).
  final num? maxSteps;
  const SolverOptions({this.maxConflicts, this.maxSteps});
}

// ─── CDCL SAT Solver ────────────────────────────────────────────────────────

/// Conflict-Driven Clause Learning SAT solver.
class Solver {
  int _numVars = 0;
  final List<List<int>> _clauses = [];
  final List<bool> _isLearned = [];
  final List<double> _clauseActivity = [];

  // Assignments & trail.
  Int8List _assigns = Int8List(0);
  final List<int> _trail = [];
  final List<int> _trailLim = [];
  Int32List _level = Int32List(0);
  Int32List _reason = Int32List(0);
  int _qhead = 0;

  // Watcher lists indexed by literal.
  final List<List<_Watcher>> _watches = [];

  // Heuristics.
  Float64List _activity = Float64List(0);
  double _varInc = 1.0;
  static const double _varDecay = 0.95;
  double _clauseInc = 1.0;
  static const double _clauseDecay = 0.999;
  late _VarOrderHeap _orderHeap;
  Int8List _polarity = Int8List(0);

  // Statistics & budgets.
  /// Conflicts encountered across all solves.
  int conflicts = 0;

  /// Decisions made across all solves.
  int decisions = 0;

  /// Trail items propagated across all solves.
  int propagations = 0;

  /// Restarts performed across all solves.
  int restarts = 0;
  double _conflictBudget = double.infinity;
  double _stepBudget = double.infinity;
  int _maxLearned = 2000;

  // Solver status & soundness.
  bool _ok = true;

  // Assumptions & core.
  Set<int> _assumptionSet = <int>{};
  List<int> _failedCore = [];

  /// Creates an empty solver.
  Solver() {
    _orderHeap = _VarOrderHeap(_activity);
  }

  /// False once a permanent root contradiction is known.
  bool okay() => _ok;

  /// Allocates a fresh variable.
  int newVar() {
    final v = _numVars++;
    _growToVars(_numVars);
    _orderHeap.insert(v);
    return v;
  }

  void _growToVars(int size) {
    if (_assigns.length >= size) return;
    final newCap = _maxInt(size, _maxInt(_assigns.length * 2, 16));

    final newAssigns = Int8List(newCap);
    newAssigns.setRange(0, _assigns.length, _assigns);
    _assigns = newAssigns;

    final newLevel = Int32List(newCap)..fillRange(0, newCap, -1);
    newLevel.setRange(0, _level.length, _level);
    _level = newLevel;

    final newReason = Int32List(newCap)..fillRange(0, newCap, -1);
    newReason.setRange(0, _reason.length, _reason);
    _reason = newReason;

    final newActivity = Float64List(newCap);
    newActivity.setRange(0, _activity.length, _activity);
    _activity = newActivity;

    final newPolarity = Int8List(newCap)..fillRange(0, newCap, LBool.false_);
    newPolarity.setRange(0, _polarity.length, _polarity);
    _polarity = newPolarity;

    while (_watches.length < newCap * 2) {
      _watches.add([]);
    }

    _orderHeap = _VarOrderHeap(_activity);
    for (var i = 0; i < size; i++) {
      if (_assigns[i] == LBool.undef) {
        _orderHeap.insert(i);
      }
    }
  }

  /// Number of allocated variables.
  int nVars() => _numVars;

  /// Number of clauses (including deleted slots).
  int nClauses() => _clauses.length;

  /// Current value of [lit] (`Undef` when unassigned or out of range).
  int value(int lit) {
    final v = varOf(lit);
    if (v < 0 || v >= _numVars) return LBool.undef;
    final val = _assigns[v];
    if (val == LBool.undef) return LBool.undef;
    return signOf(lit) == 1 ? -val : val;
  }

  /// Current value of variable [v].
  int varValue(int v) {
    if (v < 0 || v >= _numVars) return LBool.undef;
    return _assigns[v];
  }

  /// Adds a clause; simplifies against level-0 facts first.
  ///
  /// Returns false when the clause is empty after simplification (permanent
  /// contradiction). Grows the variable store for unseen variables.
  bool addClause(List<int> lits) {
    if (!_ok) return false;

    for (final lit in lits) {
      if (lit < 0) {
        throw ArgumentError(
            'Invalid literal: $lit. Literals must be non-negative integers encoded via makeLit().');
      }
    }

    // Reset to level 0 so transient solution assignments cannot corrupt
    // simplification or drop valid constraints.
    if (_decisionLevel() > 0) {
      _backjumpTo(0);
    }

    final normalized = <int>[];
    var seenTrue = false;

    for (final lit in lits) {
      final v = varOf(lit);
      while (v >= _numVars) {
        newVar();
      }

      final val = value(lit);
      if (val == LBool.true_) {
        seenTrue = true;
        break;
      }
      if (val == LBool.false_) {
        continue;
      }
      if (!normalized.contains(lit)) {
        if (normalized.contains(negateLit(lit))) {
          seenTrue = true;
          break;
        }
        normalized.add(lit);
      }
    }

    if (seenTrue) return true;

    if (normalized.isEmpty) {
      _ok = false;
      return false;
    }

    if (normalized.length == 1) {
      _assignLiteral(normalized[0], -1);
      final conflict = propagate();
      if (conflict >= 0) {
        _ok = false;
        return false;
      }
      return true;
    }

    final clauseIdx = _clauses.length;
    _clauses.add(normalized);
    _isLearned.add(false);
    _clauseActivity.add(0);
    _maxLearned = _maxInt(_clauses.length ~/ 3, 2000);

    _watches[normalized[0]].add(_Watcher(clauseIdx, normalized[1]));
    _watches[normalized[1]].add(_Watcher(clauseIdx, normalized[0]));

    return true;
  }

  void _assignLiteral(int lit, int reasonClause) {
    final v = varOf(lit);
    final val = signOf(lit) == 1 ? LBool.false_ : LBool.true_;
    _assigns[v] = val;
    _level[v] = _decisionLevel();
    _reason[v] = reasonClause;
    _polarity[v] = val;
    _trail.add(lit);
  }

  int _decisionLevel() => _trailLim.length;

  /// Two-watched-literal unit propagation.
  ///
  /// Returns the conflicting clause index, `-1` when clean, or `-2` when
  /// the step budget runs out. The budget is checked before dequeuing, so
  /// UNKNOWN preserves pending root work. Public so harnesses can drive
  /// propagation step by step.
  int propagate() {
    var conflictClause = -1;

    while (_qhead < _trail.length) {
      if (propagations + 1 > _stepBudget) {
        return -2;
      }
      final p = _trail[_qhead++];
      propagations++;

      final falseLit = negateLit(p);
      final ws = _watches[falseLit];
      var i = 0;
      var j = 0;

      while (i < ws.length) {
        final watcher = ws[i++];
        final cr = watcher.clauseIdx;
        final clause = _clauses[cr];

        // Skip clauses deleted by reduceDb.
        if (clause.isEmpty) {
          continue;
        }

        // Blocker optimization.
        if (value(watcher.blocker) == LBool.true_) {
          ws[j++] = watcher;
          continue;
        }

        // Ensure falseLit is at clause[1].
        if (clause[0] == falseLit) {
          clause[0] = clause[1];
          clause[1] = falseLit;
        }

        // If 0th literal is satisfied, keep watcher.
        final first = clause[0];
        if (first != watcher.blocker && value(first) == LBool.true_) {
          watcher.blocker = first;
          ws[j++] = watcher;
          continue;
        }

        // Find new literal to watch.
        var foundNewWatch = false;
        for (var k = 2; k < clause.length; k++) {
          if (value(clause[k]) != LBool.false_) {
            clause[1] = clause[k];
            clause[k] = falseLit;
            _watches[clause[1]].add(_Watcher(cr, first));
            foundNewWatch = true;
            break;
          }
        }

        if (foundNewWatch) continue;

        // Clause is unit or conflicting.
        ws[j++] = watcher;
        if (value(first) == LBool.false_) {
          conflictClause = cr;
          if (_decisionLevel() == 0) {
            // A root contradiction found by public propagation is permanent.
            // Assumption conflicts stay local to their solve.
            _ok = false;
          }
          _qhead = _trail.length;
          while (i < ws.length) {
            ws[j++] = ws[i++];
          }
        } else {
          _assignLiteral(first, cr);
        }
      }

      ws.length = j;
      if (conflictClause != -1) break;
    }

    return conflictClause;
  }

  /// 1-UIP conflict analysis: returns the learned clause + backjump level.
  ({List<int> learnedClause, int backjumpLevel}) _analyze(int conflictClause) {
    final seen = Uint8List(_numVars);
    final learnedClause = <int>[0];
    var pathC = 0;
    int? p;
    var index = _trail.length - 1;
    var currentClause = _clauses[conflictClause];

    do {
      for (final lit in currentClause) {
        final v = varOf(lit);
        if (p != null && v == varOf(p)) continue;
        if (seen[v] == 0 && _level[v] > 0) {
          seen[v] = 1;
          _bumpVarActivity(v);
          if (_level[v] >= _decisionLevel()) {
            pathC++;
          } else {
            learnedClause.add(lit);
          }
        }
      }

      while (seen[varOf(_trail[index])] == 0) {
        index--;
      }
      p = _trail[index--];
      final v = varOf(p);
      seen[v] = 0;
      pathC--;
      if (pathC > 0) {
        currentClause = _clauses[_reason[v]];
      }
    } while (pathC > 0);

    learnedClause[0] = negateLit(p);

    var backjumpLevel = 0;
    for (var i = 1; i < learnedClause.length; i++) {
      final lvl = _level[varOf(learnedClause[i])];
      if (lvl > backjumpLevel) {
        backjumpLevel = lvl;
        final tmp = learnedClause[1];
        learnedClause[1] = learnedClause[i];
        learnedClause[i] = tmp;
      }
    }

    return (learnedClause: learnedClause, backjumpLevel: backjumpLevel);
  }

  void _backjumpTo(int lvl) {
    if (_decisionLevel() <= lvl) return;

    for (var i = _trail.length - 1; i >= _trailLim[lvl]; i--) {
      final v = varOf(_trail[i]);
      _assigns[v] = LBool.undef;
      _level[v] = -1;
      _reason[v] = -1;
      _orderHeap.insert(v);
    }

    _trail.length = _trailLim[lvl];
    _trailLim.length = lvl;
    _qhead = _trail.length;
  }

  void _bumpVarActivity(int v) {
    _activity[v] += _varInc;
    if (_activity[v] > 1e100) {
      for (var i = 0; i < _numVars; i++) {
        _activity[i] *= 1e-100;
      }
      _varInc *= 1e-100;
    }
    _orderHeap.update(v);
  }

  void _decayActivities() {
    _varInc *= 1 / _varDecay;
    _clauseInc *= 1 / _clauseDecay;
    if (_varInc > 1e100) {
      for (var i = 0; i < _numVars; i++) {
        _activity[i] *= 1e-100;
      }
      _varInc *= 1e-100;
    }
    if (_clauseInc > 1e100) {
      for (var i = 0; i < _clauseActivity.length; i++) {
        _clauseActivity[i] *= 1e-100;
      }
      _clauseInc *= 1e-100;
    }
  }

  /// Luby restart sequence value at index [i].
  static int _luby(int i) {
    var size = 1;
    var seq = i;
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

  /// Failed-assumption core via implication-graph backward reachability.
  List<int> _extractCore(List<int> initialVars, [int? initialLit]) {
    final core = <int>[];
    if (initialLit != null) {
      core.add(initialLit);
    }
    final seen = Uint8List(_numVars);
    for (final v in initialVars) {
      if (_level[v] > 0) {
        seen[v] = 1;
      }
    }

    for (var i = _trail.length - 1; i >= 0; i--) {
      final p = _trail[i];
      final v = varOf(p);
      if (seen[v] == 1) {
        seen[v] = 0;
        final r = _reason[v];
        if (r == -1) {
          if (_level[v] > 0) {
            if (!core.contains(p)) {
              core.add(p);
            }
          }
        } else {
          final c = _clauses[r];
          for (final lit in c) {
            final lv = varOf(lit);
            if (_level[lv] > 0) {
              seen[lv] = 1;
            }
          }
        }
      }
    }

    final result = <int>[];
    for (final lit in core) {
      if (_assumptionSet.contains(lit) && !result.contains(lit)) {
        result.add(lit);
      }
    }
    return result;
  }

  /// Halves the unlocked learned multi-literal clauses by activity.
  void _reduceDb() {
    final locked = Uint8List(_clauses.length);
    for (var i = 0; i < _numVars; i++) {
      final r = _reason[i];
      if (r >= 0 && r < _clauses.length) {
        locked[r] = 1;
      }
    }

    final candidates = <({int idx, double act})>[];
    for (var i = 0; i < _clauses.length; i++) {
      if (_isLearned[i] && locked[i] == 0 && _clauses[i].length > 2) {
        candidates.add((idx: i, act: _clauseActivity[i]));
      }
    }

    candidates.sort((a, b) => a.act.compareTo(b.act));

    final deleteCount = candidates.length ~/ 2;
    for (var i = 0; i < deleteCount; i++) {
      final idx = candidates[i].idx;
      _clauses[idx] = [];
      _isLearned[idx] = false;
      _clauseActivity[idx] = 0;
    }

    _maxLearned = (_maxLearned * 1.1).floor();
  }

  /// Solves with no assumptions.
  String solve([SolverOptions options = const SolverOptions()]) =>
      solveWithAssumptions([], options);

  /// Main CDCL loop under [assumptions] with [options] budgets.
  String solveWithAssumptions(List<int> assumptions,
      [SolverOptions options = const SolverOptions()]) {
    if (!_ok) {
      _failedCore = [];
      return SolveResult.unsat;
    }

    for (final lit in assumptions) {
      if (lit < 0) {
        throw ArgumentError(
            'Invalid assumption literal: $lit. Literals must be non-negative integers.');
      }
    }

    _backjumpTo(0);
    final startConflicts = conflicts;
    final startSteps = propagations;
    _conflictBudget =
        startConflicts + (options.maxConflicts?.toDouble() ?? double.infinity);
    _stepBudget =
        startSteps + (options.maxSteps?.toDouble() ?? double.infinity);
    _assumptionSet = assumptions.toSet();
    _failedCore = [];

    try {
      for (final lit in assumptions) {
        final v = varOf(lit);
        while (v >= _numVars) {
          newVar();
        }
      }

      final rootConflict = propagate();
      if (rootConflict == -2) {
        _backjumpTo(0);
        return SolveResult.unknown;
      }
      if (rootConflict >= 0) {
        _ok = false;
        return SolveResult.unsat;
      }

      var restartCount = 0;
      var restartConflictLimit = 100 * _luby(restartCount);

      while (true) {
        final conflict = propagate();

        if (conflict == -2) {
          _backjumpTo(0);
          return SolveResult.unknown;
        }

        if (conflict != -1) {
          if (_decisionLevel() == 0) {
            _ok = false;
            return SolveResult.unsat;
          }

          conflicts++;
          if (conflicts > _conflictBudget) {
            _backjumpTo(0);
            return SolveResult.unknown;
          }

          if (_decisionLevel() <= assumptions.length &&
              assumptions.isNotEmpty) {
            final initialVars = _clauses[conflict].map(varOf).toList();
            _failedCore = _extractCore(initialVars);
            _backjumpTo(0);
            return SolveResult.unsat;
          }

          final (:learnedClause, :backjumpLevel) = _analyze(conflict);

          _backjumpTo(backjumpLevel);

          if (learnedClause.length == 1) {
            _assignLiteral(learnedClause[0], -1);
          } else {
            final cIdx = _clauses.length;
            _clauses.add(learnedClause);
            _isLearned.add(true);
            _clauseActivity.add(_clauseInc);

            _watches[learnedClause[0]].add(_Watcher(cIdx, learnedClause[1]));
            _watches[learnedClause[1]].add(_Watcher(cIdx, learnedClause[0]));

            _assignLiteral(learnedClause[0], cIdx);

            if (_clauses.length >= _maxLearned) {
              _reduceDb();
            }
          }

          _decayActivities();

          if (--restartConflictLimit <= 0) {
            restartCount++;
            restarts++;
            restartConflictLimit = 100 * _luby(restartCount);
            _backjumpTo(0);
          }
        } else {
          if (_decisionLevel() < assumptions.length) {
            final p = assumptions[_decisionLevel()];
            if (value(p) == LBool.true_) {
              _trailLim.add(_trail.length);
            } else if (value(p) == LBool.false_) {
              _failedCore = _extractCore([varOf(p)], p);
              _backjumpTo(0);
              return SolveResult.unsat;
            } else {
              _trailLim.add(_trail.length);
              _assignLiteral(p, -1);
            }
          } else {
            int? nextVar;
            while (!_orderHeap.isEmpty) {
              final v = _orderHeap.pop()!;
              if (_assigns[v] == LBool.undef) {
                nextVar = v;
                break;
              }
            }

            if (nextVar == null) {
              return SolveResult.sat;
            }

            decisions++;
            _trailLim.add(_trail.length);
            final sign = _polarity[nextVar] == LBool.true_ ? false : true;
            _assignLiteral(makeLit(nextVar, sign), -1);
          }
        }
      }
    } finally {
      _stepBudget = double.infinity;
      _conflictBudget = double.infinity;
    }
  }

  /// Model of the last SAT verdict (index = variable).
  List<bool> getModel() {
    final model = <bool>[];
    for (var i = 0; i < _numVars; i++) {
      model.add(_assigns[i] == LBool.true_);
    }
    return model;
  }

  /// Failed-assumption core of the last UNSAT-under-assumptions verdict.
  List<int> getFailedCore() => [..._failedCore];

  /// Backjumps to level 0, keeping learned clauses and heuristics.
  void reset() {
    _backjumpTo(0);
  }
}

// ─── Tseitin Transformation & Bit-Blaster (VER09a §3) ────────────────────────
//
// Tseitin encoding introduces one fresh variable per subformula, turning
// circuit equivalence into linear-size CNF instead of the exponential blowup
// of naive distribution.

/// A bit-vector: LSB-first list of literals.
typedef BitVector = List<int>;

/// Builds CNF circuits into an owned [Solver] via Tseitin encoding.
class CnfBuilder {
  final Solver _solver;

  /// Creates a builder over [solver] (or a fresh one).
  CnfBuilder([Solver? solver]) : _solver = solver ?? Solver();

  /// The underlying solver.
  Solver getSolver() => _solver;

  /// Allocates a fresh variable.
  int newVar() => _solver.newVar();

  /// Allocates a fresh positive literal.
  int newLit() => makeLit(newVar(), false);

  /// Adds a clause to the solver.
  bool addClause(List<int> lits) => _solver.addClause(lits);

  /// A literal constrained true.
  int trueLit() {
    final t = makeLit(newVar(), false);
    addClause([t]);
    return t;
  }

  /// A literal constrained false.
  int falseLit() {
    final f = makeLit(newVar(), false);
    addClause([negateLit(f)]);
    return f;
  }

  /// `out <-> (a and b)`.
  int and(int a, int b) {
    final out = newLit();
    addClause([out, negateLit(a), negateLit(b)]);
    addClause([negateLit(out), a]);
    addClause([negateLit(out), b]);
    return out;
  }

  /// `out <-> (a or b)`.
  int or(int a, int b) {
    final out = newLit();
    addClause([negateLit(out), a, b]);
    addClause([out, negateLit(a)]);
    addClause([out, negateLit(b)]);
    return out;
  }

  /// `out <-> (a xor b)`.
  int xor(int a, int b) {
    final out = newLit();
    addClause([negateLit(out), a, b]);
    addClause([negateLit(out), negateLit(a), negateLit(b)]);
    addClause([out, negateLit(a), b]);
    addClause([out, a, negateLit(b)]);
    return out;
  }

  /// `out <-> not a` (literal negation needs no clauses).
  int not(int a) => negateLit(a);

  /// `out <-> (cond ? thenLit : elseLit)`.
  int ite(int cond, int thenLit, int elseLit) {
    final out = newLit();
    addClause([negateLit(cond), negateLit(thenLit), out]);
    addClause([negateLit(cond), thenLit, negateLit(out)]);
    addClause([cond, negateLit(elseLit), out]);
    addClause([cond, elseLit, negateLit(out)]);
    return out;
  }

  /// `out <-> (a == b)`.
  int equiv(int a, int b) {
    final out = newLit();
    addClause([negateLit(out), negateLit(a), b]);
    addClause([negateLit(out), a, negateLit(b)]);
    addClause([out, a, b]);
    addClause([out, negateLit(a), negateLit(b)]);
    return out;
  }

  /// 1-bit full adder returning `(sum, cout)`.
  ({int sum, int cout}) fullAdder(int a, int b, int cin) {
    final sum = xor(xor(a, b), cin);
    final ab = and(a, b);
    final axorb = xor(a, b);
    final axorbCin = and(axorb, cin);
    final cout = or(ab, axorbCin);
    return (sum: sum, cout: cout);
  }

  /// Literal true iff [a] and [b] are bitwise equal.
  int bvEq(List<int> a, List<int> b) {
    if (a.length != b.length) {
      throw ArgumentError(
          'Bit-vector width mismatch: ${a.length} != ${b.length}');
    }
    if (a.isEmpty) return trueLit();

    var eq = equiv(a[0], b[0]);
    for (var i = 1; i < a.length; i++) {
      final bitEq = equiv(a[i], b[i]);
      eq = and(eq, bitEq);
    }
    return eq;
  }

  /// Ripple-carry adder returning LSB-first sum bits (carry out dropped).
  List<int> bvAdd(List<int> a, List<int> b) {
    if (a.length != b.length) {
      throw ArgumentError(
          'Bit-vector width mismatch: ${a.length} != ${b.length}');
    }
    final sum = <int>[];
    var carry = falseLit();

    for (var i = 0; i < a.length; i++) {
      final fa = fullAdder(a[i], b[i], carry);
      sum.add(fa.sum);
      carry = fa.cout;
    }
    return sum;
  }
}

// ─── DIMACS CNF Transport (VER09 §2, §3.6, §5) ───────────────────────────────

/// A parsed CNF formula: variable count plus packed-literal clauses.
class CnfFormula {
  /// Declared variable count.
  final int numVars;

  /// Clauses as packed literals.
  final List<List<int>> clauses;
  const CnfFormula(this.numVars, this.clauses);
}

/// Options for [parseDimacs].
class ParseDimacsOptions {
  /// Declared-variable ceiling (default 2^20).
  final int maxVars;
  const ParseDimacsOptions({this.maxVars = 1048576});
}

/// Parses strict DIMACS CNF, throwing [FormatException] on any violation.
///
/// Header must be exactly `p cnf <vars> <clauses>`; a lone `%` line ends
/// input (SATLIB); every clause needs its terminating `0`; the clause count
/// must match the header.
CnfFormula parseDimacs(String dimacs,
    [ParseDimacsOptions options = const ParseDimacsOptions()]) {
  final maxVars = options.maxVars;
  final lines = dimacs.split(RegExp(r'\r?\n'));
  var numVars = 0;
  var expectedClauses = 0;
  final clauses = <List<int>>[];
  var currentClause = <int>[];
  var headerSeen = false;

  final headerCounts = RegExp(r'^\d+$');
  final integerToken = RegExp(r'^-?\d+$');

  for (var lineIdx = 0; lineIdx < lines.length; lineIdx++) {
    final lineNum = lineIdx + 1;
    final trimmed = lines[lineIdx].trim();
    if (trimmed.isEmpty || trimmed.startsWith('c')) {
      continue;
    }

    if (trimmed == '%') {
      break;
    }

    if (!headerSeen) {
      final parts = trimmed.split(RegExp(r'\s+'));
      if (parts.length != 4 || parts[0] != 'p' || parts[1] != 'cnf') {
        throw FormatException(
            "line $lineNum: Expected DIMACS header 'p cnf <vars> <clauses>', got: $trimmed");
      }
      if (!headerCounts.hasMatch(parts[2]) ||
          !headerCounts.hasMatch(parts[3])) {
        throw FormatException(
            'line $lineNum: Malformed DIMACS header counts: vars=${parts[2]}, clauses=${parts[3]}');
      }
      numVars = int.parse(parts[2]);
      expectedClauses = int.parse(parts[3]);
      if (numVars > maxVars) {
        throw FormatException(
            'line $lineNum: Declared variable count $numVars exceeds maximum allowed limit $maxVars');
      }
      headerSeen = true;
      continue;
    }

    final tokens = trimmed.split(RegExp(r'\s+'));
    for (final tok in tokens) {
      if (tok.isEmpty) continue;
      if (!integerToken.hasMatch(tok)) {
        throw FormatException(
            'line $lineNum: Malformed integer in DIMACS clause: $tok');
      }
      final num = int.parse(tok);
      if (num == 0) {
        clauses.add(currentClause);
        currentClause = [];
      } else {
        final absNum = num.abs();
        if (absNum <= 0 || absNum > numVars) {
          throw FormatException(
              'line $lineNum: Literal index $tok out of range [1, $numVars]');
        }
        final v = absNum - 1;
        currentClause.add(makeLit(v, num < 0));
      }
    }
  }

  if (currentClause.isNotEmpty) {
    throw const FormatException(
        'DIMACS format error: Unterminated clause (missing trailing 0)');
  }

  if (!headerSeen) {
    throw const FormatException("DIMACS format error: Missing 'p cnf' header");
  }

  if (clauses.length != expectedClauses) {
    throw FormatException(
        'DIMACS clause count mismatch: header declared $expectedClauses clauses, parsed ${clauses.length}');
  }

  return CnfFormula(numVars, clauses);
}

/// Formats [formula] as standard DIMACS text.
String writeDimacs(CnfFormula formula) {
  final out = <String>['p cnf ${formula.numVars} ${formula.clauses.length}'];
  for (final clause in formula.clauses) {
    out.add('${clause.map(litToString).join(' ')} 0');
  }
  return out.join('\n');
}

int _maxInt(int a, int b) => a > b ? a : b;
