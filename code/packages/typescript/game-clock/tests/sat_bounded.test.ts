import assert from "node:assert/strict";
import test from "node:test";

import { advance_frame, type GameClockState } from "../src/index.js";
import { CnfBuilder, negateLit, varOf } from "@coding-adventures/test-bootstrap";

interface ClockBits {
  readonly raw: readonly number[];
  readonly accumulator: number;
  readonly outputs: readonly number[];
}

function makeBoundedClockFormula(includeMismatch: boolean, mutateSpiralModel = false): {
  readonly builder: CnfBuilder;
  readonly bits: ClockBits;
  readonly spiralBranches: Set<boolean>;
} {
  const builder = new CnfBuilder();
  const rawBits = [builder.newLit(), builder.newLit(), builder.newLit()];
  const accumulator = builder.newLit();
  const outputs = [builder.newLit(), builder.newLit(), builder.newLit()];
  const falseLit = builder.falseLit();
  const spiralBranches = new Set<boolean>();

  // The reference model clamps raw 0..7 at 4, then adds the reachable
  // sub-tick accumulator (0 or 1) in a three-bit vector. dt=2 and
  // max_frame_steps=1 make the low bit the remainder and high bit the strict
  // spiral-drop flag.
  const clamped = [
    builder.ite(rawBits[2], falseLit, rawBits[0]),
    builder.ite(rawBits[2], falseLit, rawBits[1]),
    rawBits[2],
  ];
  const accumulated = builder.bvAdd(clamped, [accumulator, falseLit, falseLit]);
  const expectedOutputs = [
    builder.or(accumulated[1], accumulated[2]),
    accumulated[0],
    mutateSpiralModel ? builder.or(accumulated[1], accumulated[2]) : accumulated[2],
  ];

  for (let raw = 0; raw < 8; raw += 1) {
    for (let acc = 0; acc < 2; acc += 1) {
      const prior: GameClockState = {
        tick_count: 0n,
        frame_count: BigInt(acc),
        elapsed_sim_ns: 0n,
        elapsed_wall_ns: BigInt(acc),
        accumulator_ns: BigInt(acc),
        fixed_dt_ns: 2n,
        max_frame_steps: 1,
        max_frame_delta_ns: 4n,
        total_dropped_ns: 0n,
        spiral_clamped_frames: 0n,
      };
      const result = advance_frame(prior, BigInt(raw));
      if (!result.ok) throw new Error(`unexpected refusal for raw=${raw}, accumulator=${acc}: ${result.error}`);
      assert.ok([0, 1].includes(result.value.steps_to_run));
      assert.ok([0n, 1n].includes(result.value.state.accumulator_ns));
      const observed = [
        result.value.steps_to_run === 1,
        result.value.state.accumulator_ns === 1n,
        result.value.spiral_of_death_clamped,
      ];
      spiralBranches.add(observed[2]);

      let row = builder.trueLit();
      for (let bit = 0; bit < rawBits.length; bit += 1) {
        const literal = ((raw >> bit) & 1) === 1 ? rawBits[bit] : negateLit(rawBits[bit]);
        row = builder.and(row, literal);
      }
      row = builder.and(row, acc === 1 ? accumulator : negateLit(accumulator));
      for (let output = 0; output < outputs.length; output += 1) {
        builder.addClause([
          negateLit(row),
          observed[output] ? outputs[output] : negateLit(outputs[output]),
        ]);
      }
    }
  }

  if (includeMismatch) {
    const differences = outputs.map((observed, index) => builder.xor(observed, expectedOutputs[index]));
    builder.addClause(differences);
  }

  return {
    builder,
    bits: { raw: rawBits, accumulator, outputs },
    spiralBranches,
  };
}

test("SAT: 16-case bounded clock result table agrees with independent 3-bit model", (t) => {
  const baseline = makeBoundedClockFormula(false);
  const baselineSolver = baseline.builder.getSolver();
  assert.equal(baselineSolver.solve({ maxConflicts: 10_000, maxSteps: 1_000_000 }), "sat");
  assert.deepStrictEqual([...baseline.spiralBranches].sort(), [false, true]);

  const mismatch = makeBoundedClockFormula(true);
  const mismatchSolver = mismatch.builder.getSolver();
  assert.equal(
    mismatchSolver.solve({ maxConflicts: 10_000, maxSteps: 1_000_000 }),
    "unsat",
    "every one of the 16 encoded inputs has equal public and model result bits",
  );
  t.diagnostic("domain=raw[0..7] x accumulator[0..1]; result=steps/remainder/spiral; baseline=sat; mismatch=unsat; spiral_branches=2");
});

test("SAT mutant: the same mismatch path finds a public counterexample for a wrong spiral predicate", (t) => {
  const mutant = makeBoundedClockFormula(true, true);
  const solver = mutant.builder.getSolver();
  assert.equal(solver.solve({ maxConflicts: 10_000, maxSteps: 1_000_000 }), "sat");
  const model = solver.getModel();
  const raw = mutant.bits.raw.reduce((value, literal, bit) => {
    return value | (model[varOf(literal)] ? 1 << bit : 0);
  }, 0);
  const accumulator = model[varOf(mutant.bits.accumulator)] ? 1n : 0n;
  const state: GameClockState = {
    tick_count: 0n,
    frame_count: accumulator,
    elapsed_sim_ns: 0n,
    elapsed_wall_ns: accumulator,
    accumulator_ns: accumulator,
    fixed_dt_ns: 2n,
    max_frame_steps: 1,
    max_frame_delta_ns: 4n,
    total_dropped_ns: 0n,
    spiral_clamped_frames: 0n,
  };
  const actual = advance_frame(state, BigInt(raw));
  if (!actual.ok) throw new Error(`counterexample unexpectedly refused: ${actual.error}`);
  assert.notEqual(actual.value.spiral_of_death_clamped, actual.value.steps_to_run === 1);
  t.diagnostic(`mutant=spiral_if_one_step; solver=sat; public_witness={raw:${raw},accumulator:${accumulator},actual_spiral:${actual.value.spiral_of_death_clamped},mutant_spiral:${actual.value.steps_to_run === 1}}`);
});
