# CodingAdventures.HashFunctions.CSharp

Non-cryptographic hash functions for .NET.

This package mirrors the repository hash-functions surface with FNV-1a, DJB2, polynomial rolling, MurmurHash3, SipHash-2-4, and small analysis helpers.

```csharp
using CodingAdventures.HashFunctions;

uint fnv = HashFunctions.Fnv1a32("hello");
ulong djb = HashFunctions.Djb2("abc");
uint murmur = HashFunctions.Murmur3_32("abc");

double chi2 = HashFunctions.DistributionTest(
    bytes => HashFunctions.Fnv1a64(bytes),
    inputs,
    numBuckets: 16);
```
