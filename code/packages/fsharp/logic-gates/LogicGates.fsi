namespace CodingAdventures.LogicGates

open System

/// Raised when a gate receives an argument that is not the bit 0 or 1.
/// Subclasses ArgumentOutOfRangeException; the message is invariant-culture.
/// A null parameter name raises ArgumentNullException.
[<Class; Sealed>]
type InvalidBitException =
    inherit ArgumentOutOfRangeException
    new: parameter: string * value: int -> InvalidBitException
    /// Name of the offending parameter ("a", "b" or "inputs[i]").
    member Parameter: string
    /// The rejected value.
    member Value: int
    override Message: string

/// Core digital logic gates over integer bits. Every gate is monomorphic
/// (int -> int); no generic inference can widen the public surface.
[<RequireQualifiedAccess>]
module LogicGates =
    val notGate: a: int -> int
    val andGate: a: int -> b: int -> int
    val orGate: a: int -> b: int -> int
    val xorGate: a: int -> b: int -> int
    val nand: a: int -> b: int -> int
    val nor: a: int -> b: int -> int
    val xnor: a: int -> b: int -> int
    val nandNot: a: int -> int
    val nandAnd: a: int -> b: int -> int
    val nandOr: a: int -> b: int -> int
    val nandXor: a: int -> b: int -> int
    /// Maximum number of inputs accepted by andN / orN / xorN.
    val maxInputs: int
    val andN: inputs: int list -> int
    val orN: inputs: int list -> int
    val xorN: inputs: int list -> int
