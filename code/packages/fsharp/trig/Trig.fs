namespace CodingAdventures.Trig

open System

/// Trigonometric functions from first principles.
///
/// We deliberately build these operations from arithmetic and iterative
/// algorithms so the source teaches how a runtime approximates transcendental
/// functions under the hood.
[<RequireQualifiedAccess>]
module Trig =
    [<Literal>]
    let PI = 3.141592653589793

    let private twoPi = 2.0 * PI
    let private halfPi = PI / 2.0

    let private rangeReduce (x: float) =
        let mutable reduced = x % twoPi
        if reduced > PI then
            reduced <- reduced - twoPi

        if reduced < -PI then
            reduced <- reduced + twoPi

        reduced

    let sin (x: float) =
        let reduced = rangeReduce x
        let squared = reduced * reduced
        let mutable term = reduced
        let mutable sum = term

        for k in 1 .. 19 do
            let denominator = float (2 * k) * float (2 * k + 1)
            term <- term * (-squared) / denominator
            sum <- sum + term

        sum

    let cos (x: float) =
        let reduced = rangeReduce x
        let squared = reduced * reduced
        let mutable term = 1.0
        let mutable sum = term

        for k in 1 .. 19 do
            let denominator = float (2 * k - 1) * float (2 * k)
            term <- term * (-squared) / denominator
            sum <- sum + term

        sum

    let radians (degrees: float) = degrees * (PI / 180.0)

    let degrees (radians: float) = radians * (180.0 / PI)

    let sqrt (x: float) =
        if x < 0.0 then
            raise (ArgumentOutOfRangeException("x", x, "sqrt: domain error -- input is negative"))

        elif x = 0.0 then
            0.0

        else
            let mutable guess = if x >= 1.0 then x else 1.0

            for _ in 0 .. 59 do
                let next = (guess + x / guess) / 2.0
                if abs (next - guess) < 1e-15 * guess + 1e-300 then
                    guess <- next
                else
                    guess <- next

            guess

    let tan (x: float) =
        let sine = sin x
        let cosine = cos x

        if abs cosine < 1e-15 then
            if sine > 0.0 then 1.0e308 else -1.0e308
        else
            sine / cosine

    let private atanCore (x: float) =
        let reduced = x / (1.0 + sqrt (1.0 + x * x))
        let squared = reduced * reduced
        let mutable term = reduced
        let mutable result = reduced

        for n in 1 .. 30 do
            term <- term * (-squared) * float (2 * n - 1) / float (2 * n + 1)
            result <- result + term

            if abs term < 1e-17 then
                ()

        2.0 * result

    let atan (x: float) =
        if x = 0.0 then
            0.0
        elif x > 1.0 then
            halfPi - atanCore (1.0 / x)
        elif x < -1.0 then
            -halfPi - atanCore (1.0 / x)
        else
            atanCore x

    let atan2 (y: float) (x: float) =
        if x > 0.0 then
            atan (y / x)
        elif x < 0.0 && y >= 0.0 then
            atan (y / x) + PI
        elif x < 0.0 && y < 0.0 then
            atan (y / x) - PI
        elif x = 0.0 && y > 0.0 then
            halfPi
        elif x = 0.0 && y < 0.0 then
            -halfPi
        else
            0.0
