// =========================================================================
// conditions.go — ARM1 Condition Code Evaluator
// =========================================================================
//
// Every ARM instruction has a 4-bit condition code in bits 31:28. The
// instruction only executes if the condition is satisfied by the current
// flags (N, Z, C, V). This is one of the most distinctive features of the
// ARM architecture — no other RISC architecture of the era made every
// instruction conditional.
//
// # Why conditional execution matters
//
// Without conditional execution:
//   CMP R0, #0        ; compare R0 with 0
//   BLT else          ; branch if less than
//   ADD R1, R1, #1    ; R1++ (if >= 0)
//   B done
//   else:
//   SUB R1, R1, #1    ; R1-- (if < 0)
//   done:
//
// With conditional execution:
//   CMP R0, #0        ; compare R0 with 0
//   ADDGE R1, R1, #1  ; R1++ if >= 0  (no branch!)
//   SUBLT R1, R1, #1  ; R1-- if < 0   (no branch!)
//
// Two branches eliminated. On the ARM1's 3-stage pipeline, each branch
// costs 3 cycles (pipeline flush + refill). So conditional execution
// saves 6 cycles in this example.
//
// # Condition Truth Table
//
//   Code  Suffix  Meaning                  Test
//   ────  ──────  ───────                  ────
//   0000  EQ      Equal                    Z == 1
//   0001  NE      Not Equal                Z == 0
//   0010  CS/HS   Carry Set / Unsigned ≥   C == 1
//   0011  CC/LO   Carry Clear / Unsigned < C == 0
//   0100  MI      Minus (Negative)         N == 1
//   0101  PL      Plus (Non-negative)      N == 0
//   0110  VS      Overflow Set             V == 1
//   0111  VC      Overflow Clear           V == 0
//   1000  HI      Unsigned Higher          C == 1 AND Z == 0
//   1001  LS      Unsigned Lower or Same   C == 0 OR  Z == 1
//   1010  GE      Signed ≥                 N == V
//   1011  LT      Signed <                 N != V
//   1100  GT      Signed >                 Z == 0 AND N == V
//   1101  LE      Signed ≤                 Z == 1 OR  N != V
//   1110  AL      Always                   true
//   1111  NV      Never (reserved)         false

package arm1simulator

// EvaluateCondition tests whether the given condition code is satisfied
// by the current flag state.
//
// This function is the behavioral equivalent of the ARM1's condition
// evaluation hardware — a small block of combinational logic that gates
// instruction execution.
func EvaluateCondition(cond int, flags Flags) bool {
	switch cond {
	case CondEQ:
		return flags.Z
	case CondNE:
		return !flags.Z
	case CondCS:
		return flags.C
	case CondCC:
		return !flags.C
	case CondMI:
		return flags.N
	case CondPL:
		return !flags.N
	case CondVS:
		return flags.V
	case CondVC:
		return !flags.V
	case CondHI:
		return flags.C && !flags.Z
	case CondLS:
		return !flags.C || flags.Z
	case CondGE:
		return flags.N == flags.V
	case CondLT:
		return flags.N != flags.V
	case CondGT:
		return !flags.Z && (flags.N == flags.V)
	case CondLE:
		return flags.Z || (flags.N != flags.V)
	case CondAL:
		return true
	case CondNV:
		return false
	default:
		return false
	}
}
