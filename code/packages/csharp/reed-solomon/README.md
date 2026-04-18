# reed-solomon

Pure C# Reed-Solomon error-correcting codes over GF(256).

## What It Includes

- generator polynomial construction for positive even check-byte counts
- systematic encoding as `[message bytes | check bytes]`
- syndrome computation in the repo's big-endian codeword convention
- Berlekamp-Massey, Chien search, and Forney decoding
- one-shot `Encode` and `Decode` helpers with package-specific error types

## Example

```csharp
using System.Text;
using CodingAdventures.ReedSolomon;

var message = Encoding.UTF8.GetBytes("hello");
var codeword = ReedSolomon.Encode(message, 8);

codeword[0] ^= 0xFF;
codeword[3] ^= 0x55;

var recovered = Encoding.UTF8.GetString(ReedSolomon.Decode(codeword, 8));
Console.WriteLine(recovered); // hello
```

## Development

```bash
bash BUILD
```
