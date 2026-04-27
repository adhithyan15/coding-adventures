# reed-solomon

Pure F# Reed-Solomon error-correcting codes over GF(256).

## What It Includes

- generator polynomial construction for positive even check-byte counts
- systematic encoding as `[message bytes | check bytes]`
- syndrome computation in the repo's big-endian codeword convention
- Berlekamp-Massey, Chien search, and Forney decoding
- one-shot `Encode` and `Decode` helpers with package-specific error types

## Example

```fsharp
open System.Text
open CodingAdventures.ReedSolomon.FSharp

let message = Encoding.UTF8.GetBytes("hello")
let codeword = ReedSolomon.Encode(message, 8)

codeword[0] <- codeword[0] ^^^ 0xFFuy
codeword[3] <- codeword[3] ^^^ 0x55uy

let recovered = Encoding.UTF8.GetString(ReedSolomon.Decode(codeword, 8))
printfn "%s" recovered
```

## Development

```bash
bash BUILD
```
