namespace CodingAdventures.HyperLogLog.FSharp

open System
open System.Numerics
open System.Text
open System.Text.Json

type HyperLogLog(?precision: int) =
    let precision = defaultArg precision 14
    let registers = Array.zeroCreate<byte> (1 <<< precision)

    let valueToBytes (value: obj) =
        match value with
        | null -> Encoding.UTF8.GetBytes("null")
        | :? (byte array) as bytes -> bytes
        | :? string as text -> Encoding.UTF8.GetBytes(text)
        | _ -> Encoding.UTF8.GetBytes(JsonSerializer.Serialize(value))

    let fnv1a64 (bytes: byte[]) =
        let mutable hash = 0xcbf29ce484222325UL
        for value in bytes do
            hash <- hash ^^^ uint64 value
            hash <- hash * 0x100000001b3UL
        hash

    let fmix64 (value: uint64) =
        let mutable current = value
        current <- current ^^^ (current >>> 33)
        current <- current * 0xff51afd7ed558ccdUL
        current <- current ^^^ (current >>> 33)
        current <- current * 0xc4ceb9fe1a85ec53UL
        current <- current ^^^ (current >>> 33)
        current

    let countLeadingZeros (value: uint64) (bitWidth: int) =
        if bitWidth <= 0 then 0
        elif value = 0UL then bitWidth
        else BitOperations.LeadingZeroCount value - (64 - bitWidth)

    do
        if precision < 4 || precision > 16 then
            invalidArg "precision" "precision must be between 4 and 16"

    member _.Clone() =
        let next = HyperLogLog(precision)
        Array.Copy(registers, next.Registers, registers.Length)
        next

    member _.Registers = registers
    member _.Precision = precision

    member _.Add(value: obj) =
        let hash = valueToBytes value |> fnv1a64 |> fmix64
        let bucket = int (hash >>> (64 - precision))
        let remainingBits = 64 - precision
        let mask = if remainingBits = 64 then UInt64.MaxValue else (1UL <<< remainingBits) - 1UL
        let remaining = hash &&& mask
        let rho = countLeadingZeros remaining remainingBits + 1
        if rho > int registers.[bucket] then
            registers.[bucket] <- byte rho
        ()

    member _.Count() =
        let m = float registers.Length
        let z = registers |> Array.sumBy (fun register -> Math.Pow(2.0, -float register))
        let alpha =
            match registers.Length with
            | 16 -> 0.673
            | 32 -> 0.697
            | 64 -> 0.709
            | _ -> 0.7213 / (1.0 + (1.079 / m))
        let mutable estimate = alpha * m * m / z
        if estimate <= 2.5 * m then
            let zeros = registers |> Array.filter ((=) 0uy) |> Array.length
            if zeros > 0 then
                estimate <- m * Math.Log(m / float zeros)
        max 0 (int (Math.Round estimate))

    member this.Merge(other: HyperLogLog) =
        if other.Precision <> precision then
            invalidOp "precision mismatch"
        let next = this.Clone()
        for i in 0 .. registers.Length - 1 do
            next.Registers.[i] <- max next.Registers.[i] other.Registers.[i]
        next
