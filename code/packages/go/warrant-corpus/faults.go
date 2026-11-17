package warrantcorpus

// # The ten portable fault wrappers (VER04i section 4.2)
//
// A NEGATIVE CONTROL is how a runner proves it can fail. We deliberately break
// exactly one operation of the library -- not by editing the library, but by
// wrapping the callable the runner holds -- and require the suites to notice.
//
// # Why wrap one op at a time
//
// Take the claim mul(a, b) == mul(b, a). Break EVERY op by XOR-ing the low bit
// of its result and the claim still holds: mul(a,b)^1 == mul(b,a)^1. The suite
// passes under the fault and looks "insensitive". Per-op scoping removes the
// ambiguity: a control is the triple (suite kind, op, fault) and perturbs exactly
// one op.
//
// Each fault has two halves, applied around the call, both on LIBRARY-FORM values
// (see values.go):
//
//   - onInput rewrites the arguments BEFORE the library sees them;
//   - onOutput rewrites the (already error-normalised) result afterwards.
//
//	fault              applies when                  behaviour
//	invert             op returns Bool               negate the result
//	ignore-last-byte   op has Bytes params           zero each input's last byte
//	ignore-high-bit    op has U64 params             clear bit 63 of each input
//	swap-choice        Bool param `choice`           negate it
//	missing-error      op returns Result             Err -> Ok(first Bytes arg)
//	xor-lsb            returns U8/U64/Nat            result ^ 1
//	off-by-one         returns U8/U64/Nat            result + 1 (masked)
//	return-first-arg   two params of the return type return the first argument
//	truncate-last      op returns Bytes              drop the final byte
//	drop-to-none       op returns Option             none
//
// Option is outside this profile's value model, so drop-to-none never applies to
// a checked corpus today; it is kept so the registry matches the spec.
//
// A wrapper never mutates what it is given: it builds a new value, so a fault
// cannot corrupt the enumerated domain it is being run against.

// Fault is a named transformer with a signature-based applicability test.
type Fault struct {
	Name     string
	Applies  func(sig *OpSig) bool
	OnInput  func(args []any, sig *OpSig) []any
	OnOutput func(result any, args []any, sig *OpSig) any
}

func sameInput(args []any, _ *OpSig) []any { return args }

func sameOutput(result any, _ []any, _ *OpSig) any { return result }

func hasParam(sig *OpSig, kind string) bool {
	for _, param := range sig.Params {
		if param.Ty.Kind == kind {
			return true
		}
	}
	return false
}

func mapParams(args []any, sig *OpSig, kind string, change func(any) any) []any {
	out := make([]any, len(args))
	for i, value := range args {
		if i < len(sig.Params) && sig.Params[i].Ty.Kind == kind {
			out[i] = change(value)
		} else {
			out[i] = value
		}
	}
	return out
}

func zeroLast(value any) any {
	data, ok := value.([]byte)
	if !ok || len(data) == 0 {
		return value
	}
	copied := make([]byte, len(data))
	copy(copied, data)
	copied[len(copied)-1] = 0
	return copied
}

func swapChoice(args []any, sig *OpSig) []any {
	out := make([]any, len(args))
	for i, value := range args {
		out[i] = value
		if i < len(sig.Params) && sig.Params[i].Name == "choice" && sig.Params[i].Ty.Kind == kindBool {
			if b, ok := value.(bool); ok {
				out[i] = !b
			}
		}
	}
	return out
}

func missingError(result any, args []any, sig *OpSig) any {
	if pair, ok := result.(Pair); ok && pair.Tag == "err" {
		for i, param := range sig.Params {
			if param.Ty.Kind == kindBytes {
				return Pair{"ok", args[i]}
			}
		}
	}
	return result
}

// offByOne is value + 1, wrapped to the width of the return type.
func offByOne(result any, _ []any, sig *OpSig) any {
	switch v := result.(type) {
	case uint64:
		if sig.Ret.Kind == kindU8 {
			return (v + 1) & 0xff
		}
		return v + 1
	case uint8:
		return v + 1
	}
	return result
}

func xorLsb(result any, _ []any, _ *OpSig) any {
	switch v := result.(type) {
	case uint64:
		return v ^ 1
	case uint8:
		return v ^ 1
	}
	return result
}

func twoParamsOfReturnType(sig *OpSig) bool {
	if len(sig.Params) != 2 {
		return false
	}
	for _, param := range sig.Params {
		if param.Ty.Kind != sig.Ret.Kind {
			return false
		}
	}
	return true
}

// faults is the registry, by portable name.
var faults = map[string]*Fault{
	"invert": {
		Name:    "invert",
		Applies: func(sig *OpSig) bool { return sig.Ret.Kind == kindBool },
		OnInput: sameInput,
		OnOutput: func(result any, _ []any, _ *OpSig) any {
			if b, ok := result.(bool); ok {
				return !b
			}
			return result
		},
	},
	"ignore-last-byte": {
		Name:     "ignore-last-byte",
		Applies:  func(sig *OpSig) bool { return hasParam(sig, kindBytes) },
		OnInput:  func(args []any, sig *OpSig) []any { return mapParams(args, sig, kindBytes, zeroLast) },
		OnOutput: sameOutput,
	},
	"ignore-high-bit": {
		Name:    "ignore-high-bit",
		Applies: func(sig *OpSig) bool { return hasParam(sig, kindU64) },
		OnInput: func(args []any, sig *OpSig) []any {
			return mapParams(args, sig, kindU64, func(value any) any {
				if n, ok := value.(uint64); ok {
					return n & (u64Max >> 1)
				}
				return value
			})
		},
		OnOutput: sameOutput,
	},
	"swap-choice": {
		Name: "swap-choice",
		Applies: func(sig *OpSig) bool {
			for _, param := range sig.Params {
				if param.Name == "choice" && param.Ty.Kind == kindBool {
					return true
				}
			}
			return false
		},
		OnInput:  swapChoice,
		OnOutput: sameOutput,
	},
	"missing-error": {
		Name:     "missing-error",
		Applies:  func(sig *OpSig) bool { return sig.Ret.Kind == kindResult && hasParam(sig, kindBytes) },
		OnInput:  sameInput,
		OnOutput: missingError,
	},
	"xor-lsb": {
		Name:     "xor-lsb",
		Applies:  func(sig *OpSig) bool { return isIntKind(sig.Ret.Kind) },
		OnInput:  sameInput,
		OnOutput: xorLsb,
	},
	"off-by-one": {
		Name:     "off-by-one",
		Applies:  func(sig *OpSig) bool { return isIntKind(sig.Ret.Kind) },
		OnInput:  sameInput,
		OnOutput: offByOne,
	},
	"return-first-arg": {
		Name:     "return-first-arg",
		Applies:  twoParamsOfReturnType,
		OnInput:  sameInput,
		OnOutput: func(_ any, args []any, _ *OpSig) any { return args[0] },
	},
	"truncate-last": {
		Name:    "truncate-last",
		Applies: func(sig *OpSig) bool { return sig.Ret.Kind == kindBytes },
		OnInput: sameInput,
		OnOutput: func(result any, _ []any, _ *OpSig) any {
			data, ok := result.([]byte)
			if !ok {
				return result
			}
			keep := len(data) - 1
			if keep < 0 {
				keep = 0
			}
			copied := make([]byte, keep)
			copy(copied, data)
			return copied
		},
	},
	"drop-to-none": {
		Name:     "drop-to-none",
		Applies:  func(sig *OpSig) bool { return len(sig.Ret.Text) >= 7 && sig.Ret.Text[:7] == "Option<" },
		OnInput:  sameInput,
		OnOutput: func(any, []any, *OpSig) any { return Pair{"none", nil} },
	},
}

// getFault looks a fault up by its portable name, or refuses.
func getFault(name string) *Fault {
	fault, ok := faults[name]
	if !ok {
		usageFail("unknown-fault", "unknown fault `%s`", name)
	}
	return fault
}
