// Tests for coding_adventures_sat_solver (VER09d).
//
// Part 1 checks literals, DIMACS transport, solving, learning, assumptions,
// failed cores, and budget isolation. Part 2 runs 100 seeded 3-SAT rounds
// against a brute-force oracle plus Tseitin gate/adder equivalence with
// sabotage checks and a strict DIMACS rejection battery. Part 3 replays the
// shared sat-cnf-v1 stateful corpus (exact id list, per-step execution,
// models and cores validated against the independently decoded signed CNF).
// Part 4 runs 64 seeded stateful differential traces and a bounded 2,500
// iteration mutation campaign through the Dart fuzz lane.

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart' as fuzz;
import 'package:coding_adventures_sat_solver/coding_adventures_sat_solver.dart';
import 'package:coding_adventures_sha256/coding_adventures_sha256.dart'
    as sha256;
import 'package:test/test.dart';

// Standalone deterministic PRNG for differential tests (test-local, like the
// sibling lane: the engine under test takes no PRNG dependency).
class _SplitMix64 {
  BigInt _state;
  _SplitMix64(BigInt seed) : _state = seed & ((BigInt.one << 64) - BigInt.one);
  BigInt nextU64() {
    _state = (_state + BigInt.parse('9e3779b97f4a7c15', radix: 16)) &
        ((BigInt.one << 64) - BigInt.one);
    var z = _state;
    z = ((z ^ (z >> 30)) * BigInt.parse('bf58476d1ce4e5b9', radix: 16)) &
        ((BigInt.one << 64) - BigInt.one);
    z = ((z ^ (z >> 27)) * BigInt.parse('94d049bb133111eb', radix: 16)) &
        ((BigInt.one << 64) - BigInt.one);
    return (z ^ (z >> 31)) & ((BigInt.one << 64) - BigInt.one);
  }

  int bounded(int n) {
    final bn = BigInt.from(n);
    final threshold = ((BigInt.one << 64) - bn) % bn;
    while (true) {
      final x = nextU64();
      if (x >= threshold) {
        return (x % bn).toInt();
      }
    }
  }
}

/// Independent brute-force oracle over packed literals.
bool _bruteForceSat(
    int numVars, List<List<int>> clauses, List<int> assumptions) {
  final total = 1 << numVars;
  for (var mask = 0; mask < total; mask++) {
    var sat = true;
    for (final a in assumptions) {
      final v = varOf(a);
      final bit = (mask >> v) & 1;
      if ((signOf(a) == 1 ? bit == 0 : bit == 1) == false) {
        sat = false;
        break;
      }
    }
    if (!sat) continue;
    var allSat = true;
    for (final clause in clauses) {
      var clauseSat = false;
      for (final lit in clause) {
        final v = varOf(lit);
        final bit = (mask >> v) & 1;
        if (signOf(lit) == 1 ? bit == 0 : bit == 1) {
          clauseSat = true;
          break;
        }
      }
      if (!clauseSat) {
        allSat = false;
        break;
      }
    }
    if (allSat) return true;
  }
  return false;
}

/// Independent model validator over packed literals.
bool _isModelValid(List<List<int>> clauses, List<bool> model) {
  for (final clause in clauses) {
    var clauseSat = false;
    for (final lit in clause) {
      final v = varOf(lit);
      if (signOf(lit) == 1 ? !model[v] : model[v]) {
        clauseSat = true;
        break;
      }
    }
    if (!clauseSat) return false;
  }
  return true;
}

void main() {
  group('Literals, Variables, and DIMACS Transport', () {
    test('encodes variables and literals correctly', () {
      final l0 = makeLit(0, false);
      final l0Neg = makeLit(0, true);
      expect(varOf(l0), 0);
      expect(signOf(l0), 0);
      expect(varOf(l0Neg), 0);
      expect(signOf(l0Neg), 1);
      expect(negateLit(l0), l0Neg);
      expect(negateLit(l0Neg), l0);
      final l5 = makeLit(5, true);
      expect(varOf(l5), 5);
      expect(signOf(l5), 1);
      expect(litToString(l5), '-6');
      expect(litToString(makeLit(5, false)), '6');
    });

    test('parses and writes DIMACS CNF accurately', () {
      const dimacsText = 'c Simple test formula\n'
          'p cnf 3 2\n'
          '1 -2 3 0\n'
          '-1 2 0\n';
      final cnf = parseDimacs(dimacsText);
      expect(cnf.numVars, 3);
      expect(cnf.clauses.length, 2);
      final serialized = writeDimacs(cnf);
      expect(serialized.contains('p cnf 3 2'), isTrue);
      expect(serialized.contains('1 -2 3 0'), isTrue);
      expect(serialized.contains('-1 2 0'), isTrue);
    });

    test('rejects malformed DIMACS inputs', () {
      expect(() => parseDimacs('invalid header'), throwsFormatException);
      expect(() => parseDimacs('p cnf\n1 0\n'), throwsFormatException);
      expect(() => parseDimacs('p cnf 2 1\n1 bad 0\n'), throwsFormatException);
    });
  });

  group('Solving, 1-UIP Learning, and Model Extraction', () {
    test('solves simple formulas and returns valid models', () {
      final s = Solver();
      final a = makeLit(s.newVar(), false);
      final b = makeLit(s.newVar(), false);
      final c = makeLit(s.newVar(), false);
      s.addClause([a, b]);
      s.addClause([negateLit(a), c]);
      s.addClause([negateLit(b), c]);
      expect(s.solve(), SolveResult.sat);
      final model = s.getModel();
      expect(model.length, 3);
      expect(model[varOf(c)], isTrue);
    });

    test('grows variables from public clauses, complete model', () {
      final s = Solver();
      final third = makeLit(2, false);
      expect(s.addClause([third]), isTrue);
      expect(s.nVars(), 3);
      expect(s.solve(), SolveResult.sat);
      expect(s.getModel(), [false, false, true]);
    });

    test('proves direct contradiction UNSAT at level 0', () {
      final s = Solver();
      final x = makeLit(s.newVar(), false);
      expect(s.addClause([x]), isTrue);
      expect(s.addClause([negateLit(x)]), isFalse);
    });

    test('root contradiction while adding a unit clause is sticky', () {
      final s = Solver();
      final a = makeLit(s.newVar(), false);
      final b = makeLit(s.newVar(), false);
      s.addClause([negateLit(a), b]);
      s.addClause([negateLit(a), negateLit(b)]);
      expect(s.addClause([a]), isFalse);
      expect(s.okay(), isFalse);
      expect(s.solve(), SolveResult.unsat);
    });

    test('proves Pigeonhole PHP(4,3) UNSAT', () {
      final s = Solver();
      const pigeons = 4;
      const holes = 3;
      final p = List.generate(pigeons,
          (_) => List.generate(holes, (_) => makeLit(s.newVar(), false)));
      for (var i = 0; i < pigeons; i++) {
        s.addClause(p[i]);
      }
      for (var j = 0; j < holes; j++) {
        for (var i1 = 0; i1 < pigeons; i1++) {
          for (var i2 = i1 + 1; i2 < pigeons; i2++) {
            s.addClause([negateLit(p[i1][j]), negateLit(p[i2][j])]);
          }
        }
      }
      expect(s.solve(), SolveResult.unsat);
    });

    test('solves 4-Queens with non-attacking placement', () {
      final s = Solver();
      const n = 4;
      final grid = List.generate(
          n, (_) => List.generate(n, (_) => makeLit(s.newVar(), false)));
      for (var r = 0; r < n; r++) {
        s.addClause(grid[r]);
        for (var c1 = 0; c1 < n; c1++) {
          for (var c2 = c1 + 1; c2 < n; c2++) {
            s.addClause([negateLit(grid[r][c1]), negateLit(grid[r][c2])]);
          }
        }
      }
      for (var c = 0; c < n; c++) {
        for (var r1 = 0; r1 < n; r1++) {
          for (var r2 = r1 + 1; r2 < n; r2++) {
            s.addClause([negateLit(grid[r1][c]), negateLit(grid[r2][c])]);
          }
        }
      }
      for (var r1 = 0; r1 < n; r1++) {
        for (var c1 = 0; c1 < n; c1++) {
          for (var r2 = r1 + 1; r2 < n; r2++) {
            for (var c2 = 0; c2 < n; c2++) {
              if ((r1 - r2).abs() == (c1 - c2).abs()) {
                s.addClause([negateLit(grid[r1][c1]), negateLit(grid[r2][c2])]);
              }
            }
          }
        }
      }
      expect(s.solve(), SolveResult.sat);
      final model = s.getModel();
      var queenCount = 0;
      for (var r = 0; r < n; r++) {
        for (var c = 0; c < n; c++) {
          if (model[varOf(grid[r][c])]) queenCount++;
        }
      }
      expect(queenCount, 4);
    });
  });

  group('Incremental Solving, Assumptions, and Failed Core', () {
    test('assumptions solve and extract sound cores', () {
      final s = Solver();
      final a = makeLit(s.newVar(), false);
      final b = makeLit(s.newVar(), false);
      final c = makeLit(s.newVar(), false);
      s.addClause([negateLit(a), b]);
      expect(s.solveWithAssumptions([a, b]), SolveResult.sat);
      expect(s.solveWithAssumptions([c, a, negateLit(b)]), SolveResult.unsat);
      expect(s.okay(), isTrue);
      final core = s.getFailedCore();
      expect(core.contains(a), isTrue);
      expect(core.contains(negateLit(b)), isTrue);
      expect(core.contains(c), isFalse);
      s.addClause([negateLit(b)]);
      expect(s.solveWithAssumptions([a]), SolveResult.unsat);
      expect(s.okay(), isTrue);
      expect(s.getFailedCore().contains(a), isTrue);
      expect(s.solveWithAssumptions([negateLit(a)]), SolveResult.sat);
    });

    test('empty clause and level-0 contradiction via ok flag', () {
      final s1 = Solver();
      s1.newVar();
      expect(s1.addClause([]), isFalse);
      expect(s1.okay(), isFalse);
      expect(s1.solve(), SolveResult.unsat);
      final s2 = Solver();
      final y = makeLit(s2.newVar(), false);
      expect(s2.addClause([y]), isTrue);
      expect(s2.addClause([negateLit(y)]), isFalse);
      expect(s2.okay(), isFalse);
      expect(s2.solve(), SolveResult.unsat);
    });

    test('adding clauses after solve cannot drop constraints', () {
      final s = Solver();
      final x = makeLit(s.newVar(), false);
      makeLit(s.newVar(), false);
      s.addClause([x]);
      expect(s.solve(), SolveResult.sat);
      expect(s.getModel()[varOf(x)], isTrue);
      expect(s.addClause([negateLit(x)]), isFalse);
      expect(s.solve(), SolveResult.unsat);
    });

    test('budgets isolate across successive solves', () {
      final s = Solver();
      const pigeon = 4;
      const holes = 3;
      final vars = <List<int>>[];
      for (var p = 0; p < pigeon; p++) {
        final row = <int>[];
        for (var h = 0; h < holes; h++) {
          row.add(makeLit(s.newVar(), false));
        }
        vars.add(row);
        s.addClause(row);
      }
      for (var h = 0; h < holes; h++) {
        for (var p1 = 0; p1 < pigeon; p1++) {
          for (var p2 = p1 + 1; p2 < pigeon; p2++) {
            s.addClause([negateLit(vars[p1][h]), negateLit(vars[p2][h])]);
          }
        }
      }
      expect(
          s.solve(const SolverOptions(maxConflicts: 1)), SolveResult.unknown);
      expect(s.conflicts >= 1, isTrue);
      expect(
          s.solve(const SolverOptions(maxConflicts: 100)), SolveResult.unsat);
    });
  });

  group('Differential Testing vs Brute-Force Oracle', () {
    test('agrees across 100 random 3-SAT formulas', () {
      final rng = _SplitMix64(BigInt.parse('9e3779b97f4a7c15', radix: 16));
      var satCount = 0;
      var unsatCount = 0;
      for (var round = 0; round < 100; round++) {
        final vars = 4 + rng.bounded(5);
        final clausesCount = (vars * 4.26).floor();
        final clauses = <List<int>>[];
        final s = Solver();
        for (var v = 0; v < vars; v++) {
          s.newVar();
        }
        for (var c = 0; c < clausesCount; c++) {
          final cLits = <int>[];
          while (cLits.length < 3) {
            final v = rng.bounded(vars);
            final lit = makeLit(v, rng.bounded(2) == 1);
            if (!cLits.contains(lit) && !cLits.contains(negateLit(lit))) {
              cLits.add(lit);
            }
          }
          clauses.add(cLits);
          s.addClause(cLits);
        }
        final expected = _bruteForceSat(vars, clauses, []);
        final res = s.solve();
        if (expected) {
          expect(res, SolveResult.sat);
          expect(_isModelValid(clauses, s.getModel()), isTrue);
          satCount++;
        } else {
          expect(res, SolveResult.unsat);
          unsatCount++;
        }
      }
      expect(satCount > 0, isTrue);
      expect(unsatCount > 0, isTrue);
    });
  });

  group('Tseitin Bit-Blaster & Circuit Verification', () {
    test('verifies XOR truth table', () {
      final builder = CnfBuilder();
      final a = builder.newLit();
      final b = builder.newLit();
      final xorOut = builder.xor(a, b);
      final s = builder.getSolver();
      expect(s.solveWithAssumptions([negateLit(a), negateLit(b), xorOut]),
          SolveResult.unsat);
      expect(
          s.solveWithAssumptions(
              [negateLit(a), negateLit(b), negateLit(xorOut)]),
          SolveResult.sat);
      expect(
          s.solveWithAssumptions([a, negateLit(b), xorOut]), SolveResult.sat);
      expect(s.solveWithAssumptions([a, negateLit(b), negateLit(xorOut)]),
          SolveResult.unsat);
      expect(s.solveWithAssumptions([a, b, xorOut]), SolveResult.unsat);
      expect(
          s.solveWithAssumptions([a, b, negateLit(xorOut)]), SolveResult.sat);
    });

    test('verifies 1-bit full adder over all 8 combinations', () {
      final builder = CnfBuilder();
      final a = builder.newLit();
      final b = builder.newLit();
      final cin = builder.newLit();
      final (:sum, :cout) = builder.fullAdder(a, b, cin);
      final s = builder.getSolver();
      for (var va = 0; va <= 1; va++) {
        for (var vb = 0; vb <= 1; vb++) {
          for (var vc = 0; vc <= 1; vc++) {
            final expectedSum = (va + vb + vc) % 2 == 1;
            final expectedCout = va + vb + vc >= 2;
            final assumptions = [
              va == 1 ? a : negateLit(a),
              vb == 1 ? b : negateLit(b),
              vc == 1 ? cin : negateLit(cin),
              expectedSum ? sum : negateLit(sum),
              expectedCout ? cout : negateLit(cout),
            ];
            expect(s.solveWithAssumptions(assumptions), SolveResult.sat);
            final bad = [
              va == 1 ? a : negateLit(a),
              vb == 1 ? b : negateLit(b),
              vc == 1 ? cin : negateLit(cin),
              expectedSum ? negateLit(sum) : sum,
            ];
            expect(s.solveWithAssumptions(bad), SolveResult.unsat);
          }
        }
      }
    });

    test('verifies 4-bit ripple-carry adder with sabotage', () {
      final builder = CnfBuilder();
      final a = [
        builder.newLit(),
        builder.newLit(),
        builder.newLit(),
        builder.newLit()
      ];
      final b = [
        builder.newLit(),
        builder.newLit(),
        builder.newLit(),
        builder.newLit()
      ];
      final sum = builder.bvAdd(a, b);
      final s = builder.getSolver();
      final good = [
        a[0],
        a[1],
        negateLit(a[2]),
        negateLit(a[3]),
        b[0],
        negateLit(b[1]),
        b[2],
        negateLit(b[3]),
        negateLit(sum[0]),
        negateLit(sum[1]),
        negateLit(sum[2]),
        sum[3],
      ];
      expect(s.solveWithAssumptions(good), SolveResult.sat);
      final bad = [
        a[0],
        a[1],
        negateLit(a[2]),
        negateLit(a[3]),
        b[0],
        negateLit(b[1]),
        b[2],
        negateLit(b[3]),
        sum[0],
        sum[1],
        sum[2],
        negateLit(sum[3]),
      ];
      expect(s.solveWithAssumptions(bad), SolveResult.unsat);
    });

    test('ITE, EQUIV, bvEq, widths, and const lits', () {
      final builder = CnfBuilder();
      final cond = builder.newLit();
      final t = builder.newLit();
      final e = builder.newLit();
      final iteOut = builder.ite(cond, t, e);
      final s = builder.getSolver();
      expect(s.solveWithAssumptions([cond, t, negateLit(iteOut)]),
          SolveResult.unsat);
      expect(s.solveWithAssumptions([cond, t, iteOut]), SolveResult.sat);
      expect(s.solveWithAssumptions([negateLit(cond), e, negateLit(iteOut)]),
          SolveResult.unsat);
      final eqLit = builder.equiv(t, e);
      expect(s.solveWithAssumptions([t, e, eqLit]), SolveResult.sat);
      expect(
          s.solveWithAssumptions([t, negateLit(e), eqLit]), SolveResult.unsat);
      final bveqLit = builder.bvEq([t, e], [t, e]);
      expect(s.solveWithAssumptions([bveqLit]), SolveResult.sat);
      final emptyEq = builder.bvEq([], []);
      expect(s.solveWithAssumptions([emptyEq]), SolveResult.sat);
      expect(() => builder.bvEq([t], [t, e]), throwsArgumentError);
      expect(() => builder.bvAdd([t], [t, e]), throwsArgumentError);
      final tLit = builder.trueLit();
      final fLit = builder.falseLit();
      expect(s.solveWithAssumptions([tLit, negateLit(fLit)]), SolveResult.sat);
    });

    test('rejects malformed DIMACS strictly', () {
      expect(() => parseDimacs('c comment only\n\n'), throwsFormatException);
      expect(() => parseDimacs('p cnf 2 1\n1 2\n'), throwsFormatException);
      expect(() => parseDimacs('p cnf -2 1\n1 0\n'), throwsFormatException);
      expect(() => parseDimacs('p cnf 2 1\n3 0\n'), throwsFormatException);
      expect(
          () => parseDimacs('p cnf 2 1\n1 123abc 0\n'), throwsFormatException);
      expect(() => parseDimacs('p cnf 2 2\n1 2 0\n'), throwsFormatException);
      expect(
          () => parseDimacs(
              'p cnf 100 1\n1 0\n', const ParseDimacsOptions(maxVars: 50)),
          throwsFormatException);
      for (final malformed in [
        'p cnfx 1 0\n',
        'p cnf-extra 1 0\n',
        'p cnf 1 0 extra\n',
        'p cnf 1 0 extra fields\n',
      ]) {
        expect(() => parseDimacs(malformed), throwsFormatException);
      }
      expect(parseDimacs('p\tcnf\t1\t0\n').numVars, 1);
      final satlib = parseDimacs(
          'p cnf 2 1\n1 2 0\n%\nmalformed non-dimacs garbage trailing lines');
      expect(satlib.numVars, 2);
      expect(satlib.clauses.length, 1);
    });
  });

  group('Shared SAT CNF v1 stateful corpus', () {
    test('consumes the same fixed valid cases as the sibling lanes', () {
      final bytes = File('../../../specs/fixtures/sat-cnf-v1/cases.json')
          .readAsBytesSync();
      expect(
        sha256.sha256Hex(bytes),
        '0bea6fc66d5d407bebba080663a3d128eabf9804cb8d57c06b4955d9e29944a1',
      );
      final corpus = jsonDecode(utf8.decode(bytes)) as Map<String, dynamic>;
      const expectedIds = [
        'empty_formula_sat',
        'assumption_satisfies_clause',
        'assumption_conflict_with_irrelevant_and_duplicate',
        'nonminimal_but_sound_failed_core',
        'empty_clause_unsat',
        'transport_crlf_comments_spanning_and_multiple_clauses',
        'satlib_terminator_ignores_following_text',
        'negative_zero_terminates_empty_clause',
        'contradictory_assumptions_and_recovery',
        'zero_budget_unknown_then_unbounded_sat',
        'clause_addition_invalidates_model_and_preserves_constraint',
        'unknown_preserves_pending_root_conflict',
        'unknown_public_propagate_root_conflict_is_sticky',
        'known_root_contradiction_clears_core',
      ];
      expect(corpus['version'], 1);
      final cases = (corpus['cases'] as List).cast<Map<String, dynamic>>();
      expect(cases.map((c) => c['id']).toList(), expectedIds);

      for (final testCase in cases) {
        final id = testCase['id'] as String;
        final independent =
            _parseSharedDimacsIndependently(testCase['dimacs'] as String);
        final formula = parseDimacs(testCase['dimacs'] as String);
        expect(formula.numVars, independent.numVars, reason: '$id: vars');
        expect(
          formula.clauses,
          independent.clauses
              .map((clause) => clause.map(_packedFromSignedDimacs).toList())
              .toList(),
          reason: '$id: literal encoding',
        );
        final solver = Solver();
        for (var i = 0; i < formula.numVars; i++) {
          solver.newVar();
        }
        var clauses = independent.clauses.map((c) => [...c]).toList();
        for (final clause in formula.clauses) {
          solver.addClause(clause);
        }

        for (final step
            in (testCase['steps'] as List).cast<Map<String, dynamic>>()) {
          final op = step['op'] as String;
          if (op == 'solve') {
            final assumptions = ((step['assumptions'] as List?) ?? [])
                .map((a) => _dimacsLit((a as num).toInt()))
                .toList();
            final budget = step['budget'] as Map<String, dynamic>?;
            final options = SolverOptions(
              maxConflicts: (budget?['max_conflicts'] as num?)?.toDouble(),
              maxSteps: (budget?['max_steps'] as num?)?.toDouble(),
            );
            final result = solver.solveWithAssumptions(assumptions, options);
            expect(result, step['expect'], reason: '$id: solve result');
            if (result == SolveResult.sat) {
              final model = solver.getModel();
              expect(model.length, formula.numVars, reason: '$id: model width');
              expect(_independentlySatisfies(clauses, model), isTrue,
                  reason: '$id: model satisfies original CNF');
              expect(_independentlySatisfiesPackedLiterals(assumptions, model),
                  isTrue,
                  reason: '$id: model satisfies assumptions');
              expect(solver.getFailedCore(), isEmpty,
                  reason: '$id: SAT clears core');
            } else if (result == SolveResult.unsat) {
              final core = solver.getFailedCore();
              expect(core.toSet().length, core.length,
                  reason: '$id: core has no duplicates');
              expect(core.every(assumptions.contains), isTrue,
                  reason: '$id: core is an assumption subset');
              expect(
                  _independentlyBruteForceSat(formula.numVars, clauses,
                      core.map(_signedDimacsFromPacked).toList()),
                  isFalse,
                  reason: '$id: core is sound against original CNF');
              if (step['expect_core'] != null) {
                final want = ((step['expect_core'] as List)
                    .map((e) => _dimacsLit((e as num).toInt()))
                    .toList()
                  ..sort());
                expect([...core]..sort(), want);
              }
            } else {
              expect(solver.getFailedCore(), isEmpty,
                  reason: '$id: UNKNOWN clears core');
            }
          } else if (op == 'propagate') {
            final conflict = solver.propagate() >= 0;
            expect(conflict, step['expect_conflict'],
                reason: '$id: public propagation conflict');
          } else if (op == 'add_clause') {
            final clause = ((step['clause'] as List?) ?? [])
                .map((e) => _dimacsLit((e as num).toInt()))
                .toList();
            solver.addClause(clause);
            clauses = [
              ...clauses,
              ((step['clause'] as List?) ?? [])
                  .map((e) => (e as num).toInt())
                  .toList()
            ];
          } else if (op == 'reset') {
            solver.reset();
          } else {
            fail('$id: unknown corpus operation $op');
          }
        }
      }
    });
  });

  group('Seeded stateful SAT differential properties', () {
    test('checks 64 traces against the original signed CNF', () {
      final seed = BigInt.parse('51a7e5eed', radix: 16);
      final rng = _SplitMix64(seed);
      const iterations = 64;
      const budgets = [0, 1, 1.5];
      var operations = 0;
      var assumptionRecoveries = 0;
      var initialUnknowns = 0;

      for (var iteration = 0; iteration < iterations; iteration++) {
        final numVars = 1 + rng.bounded(5);
        final signedClauses = List.generate(rng.bounded(9), (_) {
          final width = rng.bounded(4);
          return List.generate(width, (_) {
            final variable = 1 + rng.bounded(numVars);
            return rng.bounded(2) == 0 ? variable : -variable;
          });
        });
        final dimacs = 'p cnf $numVars ${signedClauses.length}\n'
            '${signedClauses.map((clause) => '${clause.join(' ')} 0').join('\n')}\n';
        final original = _parseSharedDimacsIndependently(dimacs);
        final parsed = parseDimacs(dimacs);
        expect(parsed.numVars, original.numVars);
        expect(
          parsed.clauses,
          original.clauses
              .map((clause) => clause.map(_packedFromSignedDimacs).toList())
              .toList(),
        );

        final solver = Solver();
        for (var variable = 0; variable < numVars; variable++) {
          solver.newVar();
        }
        for (final clause in parsed.clauses) {
          solver.addClause(clause);
        }

        final firstBudget = budgets[iteration % budgets.length];
        final first =
            solver.solve(SolverOptions(maxSteps: firstBudget.toDouble()));
        operations++;
        if (first == SolveResult.unknown) initialUnknowns++;
        _assertStatefulOracleResult(solver, first, numVars, original.clauses,
            [], [], 'iter=$iteration initial budget=$firstBudget');

        final propagation = solver.propagate();
        operations++;
        expect(propagation == -1 || propagation >= 0, isTrue);
        if (!solver.okay()) {
          expect(_independentlyBruteForceSat(numVars, original.clauses, []),
              isFalse);
        }
        final recovered = solver.solve();
        operations++;
        _assertStatefulOracleResult(solver, recovered, numVars,
            original.clauses, [], [], 'iter=$iteration unbounded recovery');

        final signedAssumptions = List.generate(rng.bounded(3), (_) {
          final variable = 1 + rng.bounded(numVars);
          return rng.bounded(2) == 0 ? variable : -variable;
        });
        final packedAssumptions = signedAssumptions.map(_dimacsLit).toList();
        final assumptionBudget = budgets[(iteration + 1) % budgets.length];
        var underAssumptions = solver.solveWithAssumptions(packedAssumptions,
            SolverOptions(maxSteps: assumptionBudget.toDouble()));
        operations++;
        _assertStatefulOracleResult(
            solver,
            underAssumptions,
            numVars,
            original.clauses,
            packedAssumptions,
            signedAssumptions,
            'iter=$iteration assumptions budget=$assumptionBudget');
        if (underAssumptions == SolveResult.unknown) {
          assumptionRecoveries++;
          underAssumptions = solver.solveWithAssumptions(packedAssumptions);
          operations++;
          _assertStatefulOracleResult(
              solver,
              underAssumptions,
              numVars,
              original.clauses,
              packedAssumptions,
              signedAssumptions,
              'iter=$iteration assumptions recovery');
        }

        final addedClause = List.generate(rng.bounded(4), (_) {
          final variable = 1 + rng.bounded(numVars);
          return rng.bounded(2) == 0 ? variable : -variable;
        });
        solver.addClause(addedClause.map(_dimacsLit).toList());
        operations++;
        final updatedClauses = [...original.clauses, addedClause];
        final afterAddition = solver.solve();
        operations++;
        _assertStatefulOracleResult(solver, afterAddition, numVars,
            updatedClauses, [], [], 'iter=$iteration permanent clause');

        solver.reset();
        operations++;
        final afterReset = solver.solve();
        operations++;
        _assertStatefulOracleResult(solver, afterReset, numVars, updatedClauses,
            [], [], 'iter=$iteration reset recovery');
      }

      expect(iterations, 64);
      expect(operations, 512 + assumptionRecoveries);
      expect(initialUnknowns > 0, isTrue,
          reason: 'traces must exercise UNKNOWN recovery');
    });
  });

  group('Bounded SAT lifecycle mutation campaign', () {
    test('checks every public result against the independent oracle', () {
      final satisfiableSeed = _encodeMutationSeed(2, [
        [1],
        [-1, 2]
      ]);
      final unsatisfiableSeed = _encodeMutationSeed(2, [
        [1],
        [-1]
      ]);
      expect(
          _independentlyBruteForceSat(2, [
            [1],
            [-1, 2]
          ], []),
          isTrue,
          reason: 'positive seed is SAT');
      expect(
          _independentlyBruteForceSat(2, [
            [1],
            [-1]
          ], []),
          isFalse,
          reason: 'negative seed is UNSAT');

      var invocations = 0;
      final controlResults = <String>[];
      const budgets = [0, 1, 1.5];
      fuzz.FuzzOutcome target(Uint8List input, fuzz.CoverageBitmap coverage) {
        invocations++;
        int byte(int index) => index < input.length ? input[index] : 0;
        final formula = _decodeMutationSeed(input);
        final dimacs = 'p cnf ${formula.numVars} ${formula.clauses.length}\n'
            '${formula.clauses.map((clause) => '${clause.join(' ')} 0').join('\n')}\n';
        final oracleParsed = _parseSharedDimacsIndependently(dimacs);
        final parsed = parseDimacs(dimacs);
        expect(parsed.numVars, oracleParsed.numVars,
            reason: 'mutation parser variable count');
        expect(
          parsed.clauses,
          oracleParsed.clauses
              .map((clause) => clause.map(_packedFromSignedDimacs).toList())
              .toList(),
          reason: 'mutation parser signed clauses',
        );
        coverage.recordEdge(100 + (formula.numVars < 5 ? formula.numVars : 5));
        coverage.recordEdge(
            110 + (formula.clauses.length < 8 ? formula.clauses.length : 8));
        if (formula.clauses.any((clause) => clause.isEmpty)) {
          coverage.recordEdge(120);
        }

        final solver = Solver();
        for (var variable = 0; variable < formula.numVars; variable++) {
          solver.newVar();
        }
        for (final clause in parsed.clauses) {
          solver.addClause(clause);
        }

        final bounded = solver.solve(SolverOptions(
            maxSteps: budgets[byte(2) % budgets.length].toDouble()));
        if (invocations <= 2) controlResults.add(bounded);
        coverage.recordEdge(bounded == SolveResult.sat
            ? 130
            : bounded == SolveResult.unsat
                ? 131
                : 132);
        _assertStatefulOracleResult(solver, bounded, formula.numVars,
            oracleParsed.clauses, [], [], 'mutation bounded solve');

        final propagation = solver.propagate();
        coverage.recordEdge(propagation >= 0 ? 140 : 141);
        if (!solver.okay()) {
          coverage.recordEdge(142);
          expect(
              _independentlyBruteForceSat(
                  formula.numVars, oracleParsed.clauses, []),
              isFalse,
              reason: 'mutation root contradiction agrees with CNF');
        }
        final recovered = solver.solve();
        coverage.recordEdge(recovered == SolveResult.sat
            ? 150
            : recovered == SolveResult.unsat
                ? 151
                : 152);
        _assertStatefulOracleResult(solver, recovered, formula.numVars,
            oracleParsed.clauses, [], [], 'mutation recovery');

        final signedAssumptions = [byte(3), byte(4)].map((raw) {
          final variable = (raw % formula.numVars) + 1;
          return (raw & 0x80) == 0 ? variable : -variable;
        }).toList();
        final packedAssumptions = signedAssumptions.map(_dimacsLit).toList();
        final assumed = solver.solveWithAssumptions(
          packedAssumptions,
          SolverOptions(maxSteps: budgets[byte(5) % budgets.length].toDouble()),
        );
        coverage.recordEdge(assumed == SolveResult.sat
            ? 160
            : assumed == SolveResult.unsat
                ? 161
                : 162);
        _assertStatefulOracleResult(
            solver,
            assumed,
            formula.numVars,
            oracleParsed.clauses,
            packedAssumptions,
            signedAssumptions,
            'mutation assumptions');
        if (assumed == SolveResult.unknown) {
          final assumptionRecovery =
              solver.solveWithAssumptions(packedAssumptions);
          coverage.recordEdge(163);
          _assertStatefulOracleResult(
              solver,
              assumptionRecovery,
              formula.numVars,
              oracleParsed.clauses,
              packedAssumptions,
              signedAssumptions,
              'mutation assumption recovery');
        }

        final addedClause =
            [byte(6), byte(7), byte(8)].sublist(0, byte(9) % 4).map((raw) {
          final variable = (raw % formula.numVars) + 1;
          return (raw & 0x80) == 0 ? variable : -variable;
        }).toList();
        solver.addClause(addedClause.map(_dimacsLit).toList());
        final updatedClauses = [...oracleParsed.clauses, addedClause];
        final afterAddition = solver.solve();
        coverage.recordEdge(afterAddition == SolveResult.sat
            ? 170
            : afterAddition == SolveResult.unsat
                ? 171
                : 172);
        _assertStatefulOracleResult(solver, afterAddition, formula.numVars,
            updatedClauses, [], [], 'mutation permanent clause');

        solver.reset();
        coverage.recordEdge(180);
        final afterReset = solver.solve();
        _assertStatefulOracleResult(solver, afterReset, formula.numVars,
            updatedClauses, [], [], 'mutation reset recovery');
        return const fuzz.OutcomePass();
      }

      for (final seed in [satisfiableSeed, unsatisfiableSeed]) {
        final coverage = fuzz.CoverageBitmap();
        final outcome = target(seed, coverage);
        expect(outcome, isA<fuzz.OutcomePass>());
        expect(coverage.bytes.any((byte) => byte != 0), isTrue,
            reason: 'semantic control records harness feedback');
      }
      expect(controlResults, ['sat', 'unsat'],
          reason: 'same-target controls classify correctly');

      final report = fuzz.fuzzTarget(
        [satisfiableSeed, unsatisfiableSeed],
        fuzz.FuzzOptions(
          seed: BigInt.from(0x504043),
          maxIterations: 2500,
          maxInputLen: 34,
          baseEnergy: 12,
        ),
        target,
      );
      expect(report.iterations, 2500,
          reason: 'bounded campaign executes its full iteration budget');
      expect(report.crashes.length, 0,
          reason:
              'no mutation violates parser, model, core, or lifecycle oracle');
      expect(report.totalEdges >= 10, isTrue,
          reason: 'mutations explore distinct semantic harness outcomes');
      expect(invocations, 2504,
          reason:
              '2 harness controls + 2 engine seed evaluations + 2500 mutations');
    });
  });

  group('Accessors, validation, and deep engine paths', () {
    test('nClauses, varValue, and out-of-range values', () {
      final s = Solver();
      final a = makeLit(s.newVar(), false);
      final b = makeLit(s.newVar(), false);
      expect(s.nClauses(), 0);
      s.addClause([a, b]);
      expect(s.nClauses(), 1);
      expect(s.varValue(varOf(a)), LBool.undef);
      expect(s.varValue(-1), LBool.undef);
      expect(s.varValue(99), LBool.undef);
      expect(s.value(makeLit(99, false)), LBool.undef);
      expect(s.solve(), SolveResult.sat);
      expect(s.varValue(varOf(a)) != LBool.undef, isTrue);
    });

    test('negative literals are rejected', () {
      final s = Solver();
      expect(() => s.addClause([-1]), throwsArgumentError);
      expect(() => s.solveWithAssumptions([-1]), throwsArgumentError);
    });

    test('long clauses dedup in linear time, tautologies drop', () {
      // Security regression: dedup must be set-backed, not a nested scan.
      final s = Solver();
      final lits = <int>[];
      for (var i = 0; i < 5000; i++) {
        lits.add(makeLit(s.newVar(), false));
        lits.add(makeLit(0, false)); // duplicate first literal
      }
      final sw = Stopwatch()..start();
      expect(s.addClause(lits), isTrue);
      sw.stop();
      expect(sw.elapsedMilliseconds < 5000, isTrue,
          reason: 'clause load must be linear, not quadratic');
      final t = Solver();
      final a = makeLit(t.newVar(), false);
      expect(t.nClauses(), 0);
      expect(t.addClause([a, makeLit(1, false), negateLit(a)]), isTrue);
      expect(t.nClauses(), 0, reason: 'tautology adds no clause');
    });

    test('assumptions grow the variable store', () {
      final s = Solver();
      expect(s.solveWithAssumptions([makeLit(7, false)]), SolveResult.sat);
      expect(s.nVars(), 8);
      expect(s.getModel()[7], isTrue);
    });

    test('builder.not negates without clauses', () {
      final builder = CnfBuilder();
      final a = builder.newLit();
      expect(builder.not(a), negateLit(a));
      expect(builder.not(negateLit(a)), a);
      expect(builder.getSolver().nClauses(), 0);
    });

    test('root UNKNOWN preserves pending work for the next solve', () {
      // First call strands trail work mid-propagation; the second call with
      // a zero step budget must observe it at the root (budget check fires
      // before any dequeue) instead of deciding.
      final s = Solver();
      final x1 = makeLit(s.newVar(), false);
      final x2 = makeLit(s.newVar(), false);
      s.addClause([x1, x2]);
      s.addClause([x1, negateLit(x2)]);
      s.addClause([negateLit(x1), x2]);
      s.addClause([negateLit(x1), negateLit(x2)]);
      expect(s.solve(const SolverOptions(maxSteps: 1)), SolveResult.unknown);
      expect(s.solve(const SolverOptions(maxSteps: 0)), SolveResult.unknown);
      expect(s.solve(), SolveResult.unsat);
    });

    test('unit learnt clauses assign at level 0', () {
      // [¬x] pins x false; the negative-first decision falsifies [x, b],
      // and analysis learns the unit [b], which must hold in the model.
      final s = Solver();
      final x = makeLit(s.newVar(), false);
      final b = makeLit(s.newVar(), false);
      s.addClause([negateLit(x)]);
      s.addClause([x, b]);
      expect(s.solve(), SolveResult.sat);
      expect(s.getModel()[varOf(b)], isTrue);
    });

    test('PHP(8,7) drives restarts, reduction, and rescaling', () {
      // ~10k conflicts in ~0.1s: exercises Luby restarts past the first
      // interval, learned-clause database reduction, and the 1e100
      // activity-rescale guards. Thresholds pin that the stress landed.
      final s = Solver();
      const pigeons = 8;
      const holes = 7;
      final p = List.generate(pigeons,
          (_) => List.generate(holes, (_) => makeLit(s.newVar(), false)));
      for (var i = 0; i < pigeons; i++) {
        s.addClause(p[i]);
      }
      for (var j = 0; j < holes; j++) {
        for (var i1 = 0; i1 < pigeons; i1++) {
          for (var i2 = i1 + 1; i2 < pigeons; i2++) {
            s.addClause([negateLit(p[i1][j]), negateLit(p[i2][j])]);
          }
        }
      }
      expect(s.solve(), SolveResult.unsat);
      expect(s.conflicts > 4500, isTrue,
          reason: 'rescale guards need thousands of conflicts');
      expect(s.restarts > 1, isTrue, reason: 'Luby intervals past the first');
      expect(s.decisions > 0, isTrue);
    });
  });
}

// ─── Independent signed-CNF oracle (test-only) ──────────────────────────────
//
// This parser/oracle intentionally avoids the production literal helpers. It
// consumes signed DIMACS integers and decodes packed literals arithmetically,
// so a shared packing bug cannot validate the same bad result it created.

int _dimacsLit(int value) {
  assert(value != 0, '0 terminates a DIMACS clause; it is not an assumption');
  return makeLit(value.abs() - 1, value < 0);
}

({int numVars, List<List<int>> clauses}) _parseSharedDimacsIndependently(
    String text) {
  final lines = text.replaceAll(RegExp(r'\r\n?'), '\n').split('\n');
  int? numVars;
  int? declaredClauses;
  final tokens = <String>[];
  var terminated = false;
  for (final line in lines) {
    final trimmed = line.trim();
    if (trimmed.isEmpty || trimmed.startsWith('c')) continue;
    if (trimmed == '%') {
      terminated = true;
      break;
    }
    if (numVars == null) {
      final header = RegExp(r'^p cnf (\d+) (\d+)$').firstMatch(trimmed);
      assert(header != null, 'invalid shared fixture header: $trimmed');
      numVars = int.parse(header!.group(1)!);
      declaredClauses = int.parse(header.group(2)!);
      continue;
    }
    tokens.addAll(trimmed.split(RegExp(r'\s+')));
  }
  assert(terminated || numVars != null);
  assert(numVars != null);
  final clauses = <List<int>>[];
  var clause = <int>[];
  for (final token in tokens) {
    final value = int.parse(token);
    if (value == 0) {
      clauses.add(clause);
      clause = [];
    } else {
      assert(value.abs() <= numVars!,
          'fixture literal exceeds declared variables: $value');
      clause.add(value);
    }
  }
  assert(clause.isEmpty, 'shared fixture must terminate every clause');
  assert(clauses.length == declaredClauses);
  return (numVars: numVars!, clauses: clauses);
}

int _packedFromSignedDimacs(int value) =>
    2 * (value.abs() - 1) + (value < 0 ? 1 : 0);

int _signedDimacsFromPacked(int literal) {
  final variable = literal ~/ 2 + 1;
  return literal % 2 == 1 ? -variable : variable;
}

bool _independentlySatisfiesPackedLiterals(
    List<int> literals, List<bool> model) {
  return literals.every((literal) {
    final variable = literal ~/ 2;
    final negative = literal % 2 == 1;
    return negative ? !model[variable] : model[variable];
  });
}

bool _independentlySatisfies(List<List<int>> clauses, List<bool> model) {
  return clauses.every((clause) => clause.any((literal) {
        final value = model[literal.abs() - 1];
        return literal < 0 ? !value : value;
      }));
}

bool _independentlyBruteForceSat(
    int numVars, List<List<int>> clauses, List<int> assumptions) {
  final total = 1 << numVars;
  for (var mask = 0; mask < total; mask++) {
    final model =
        List.generate(numVars, (variable) => ((mask >> variable) & 1) == 1);
    final assumptionsHold = assumptions.every((literal) =>
        literal < 0 ? !model[literal.abs() - 1] : model[literal - 1]);
    if (assumptionsHold && _independentlySatisfies(clauses, model)) {
      return true;
    }
  }
  return false;
}

void _assertStatefulOracleResult(
  Solver solver,
  String result,
  int numVars,
  List<List<int>> originalClauses,
  List<int> packedAssumptions,
  List<int> signedAssumptions,
  String context,
) {
  final expectedSat =
      _independentlyBruteForceSat(numVars, originalClauses, signedAssumptions);
  if (result == SolveResult.unknown) {
    expect(solver.getFailedCore(), isEmpty,
        reason: '$context: UNKNOWN clears core');
    return;
  }
  expect(result, expectedSat ? SolveResult.sat : SolveResult.unsat,
      reason: '$context: result versus independent oracle');
  if (result == SolveResult.sat) {
    final model = solver.getModel();
    expect(model.length, numVars, reason: '$context: model width');
    expect(_independentlySatisfies(originalClauses, model), isTrue,
        reason: '$context: model satisfies original CNF');
    expect(
        _independentlySatisfiesPackedLiterals(packedAssumptions, model), isTrue,
        reason: '$context: model satisfies assumptions');
    expect(solver.getFailedCore(), isEmpty,
        reason: '$context: SAT clears core');
    return;
  }
  final core = solver.getFailedCore();
  expect(core.toSet().length, core.length,
      reason: '$context: core has no duplicates');
  expect(core.every(packedAssumptions.contains), isTrue,
      reason: '$context: core is an assumption subset');
  expect(
      _independentlyBruteForceSat(
          numVars, originalClauses, core.map(_signedDimacsFromPacked).toList()),
      isFalse,
      reason: '$context: core is sound against original CNF');
}

Uint8List _encodeMutationSeed(int numVars, List<List<int>> clauses) {
  final bytes = Uint8List(34);
  bytes[0] = numVars - 1;
  bytes[1] = clauses.length;
  for (var clauseIndex = 0;
      clauseIndex < (clauses.length < 8 ? clauses.length : 8);
      clauseIndex++) {
    final offset = 2 + clauseIndex * 4;
    final clause = clauses[clauseIndex];
    bytes[offset] = clause.length;
    for (var literalIndex = 0;
        literalIndex < (clause.length < 3 ? clause.length : 3);
        literalIndex++) {
      final literal = clause[literalIndex];
      bytes[offset + literalIndex + 1] =
          literal.abs() - 1 | (literal < 0 ? 0x80 : 0);
    }
  }
  return bytes;
}

({int numVars, List<List<int>> clauses}) _decodeMutationSeed(Uint8List input) {
  int byte(int index) => index < input.length ? input[index] : 0;
  final numVars = (byte(0) % 5) + 1;
  final clauseCount = byte(1) % 9;
  final clauses = List.generate(clauseCount, (clauseIndex) {
    final offset = 2 + clauseIndex * 4;
    final width = byte(offset) % 4;
    return List.generate(width, (literalIndex) {
      final raw = byte(offset + literalIndex + 1);
      final variable = (raw % numVars) + 1;
      return (raw & 0x80) == 0 ? variable : -variable;
    });
  });
  return (numVars: numVars, clauses: clauses);
}
