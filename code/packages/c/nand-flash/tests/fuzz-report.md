# DB-E12 C NAND fuzz receipt

This receipt covers the C NAND test adapter and command-history harness.
Fixture bytes that fail syntax, schema, model consistency, or the
replay-domain cap are expected inputs. A crash records a sanitizer finding or
a disagreement between the model and runtime.

## Campaign

- Baseline Git revision `87948dfbfd334df27ea33b33e2505a919be5966b`.
- Debian Clang, llvm-profdata, and llvm-cov 19.1.7.
- Run `sh tools/fuzz.sh` from the package directory with those tools on PATH.
  The script builds both targets with `-fsanitize=fuzzer,address,undefined`.
- Fixture mode ran 100,000 inputs in 13 seconds with seed `12001` and a
  65,536-byte limit. It reached 1,018 edge counters and 3,763 features in a
  216-file corpus of 552 KiB.
- Binary history mode ran 100,000 inputs in under one second with seed
  `12002` and a 514-byte limit. It reached 299 edge counters and 816 features
  in a 77-file corpus of 2,457 bytes.
- Both campaigns completed without ASan/UBSan diagnostics, runtime
  disagreements, or crash artifacts. Each began from a fresh scratch copy of
  the seed corpus. Fixture mode also copied the shared NAND vector.

The source hashes identify the adapter and driver used in the campaigns. The
baseline Git revision precedes the implementation commit.

| Input | SHA-256 |
|---|---|
| `src/nand_flash.c` | `8ed4d91597cc75763c5f7a91f9c36ca10ee8aeedf47f3d3f4084289fb0f5e94f` |
| `tests/nand_flash_test.c` | `9b914f7d1e0999867d538ff2d9a9adc20824bc4648182b75106bfa4cd32aa5fe` |
| `tests/nand_flash_fuzz_support.c` | `596288fe371e61de26dc1d69e050876db500307624b20617aa7ec2c30c0fed78` |
| `tests/nand_flash_fuzz_support.h` | `5d9f4efa7ccd8b9c81b6c98882ee4fbee23d446dc8bcbf73f2dc7be73612fd14` |
| `tests/nand_flash_fuzz.c` | `cab0b24693c830444c3203ba410028d2bae3d63a3639f28f22c915e31dc250c2` |
| `tools/fuzz.sh` | `26f913f138a4b92855b08504c155e659cd3f32d3f5467addb0c0192a709f7db8` |
| `../bounded-json/include/bounded_json.h` | `c061c218549ea0f4663cb1386ffc2eeff2e96d583ece727221e44d1885ed7422` |
| `../bounded-json/src/bounded_json.c` | `a5d9a0cff2cd8442d57d4c9c556fc5b60a40cbe90f0353462af02571ee2d54a5` |
| Canonical NAND fixture | `6bf9f32a47af48da802a56d3d5104b48a8601286a31c62cc43c78c3e0baaf05e` |

## Coverage

Coverage comes from each 100,000-run sanitized fuzzer binary, using LLVM
source/branch instrumentation. The rows separate the runtime, strict adapter,
fuzz driver, and parser.

| Source | Fixture lines / branches | History lines / branches |
|---|---:|---:|
| NAND runtime | 81/88 (92.05%) / 39/80 (48.75%) | 81/88 (92.05%) / 52/80 (65.00%) |
| Fixture adapter | 400/602 (66.45%) / 298/566 (52.65%) | 184/602 (30.56%) / 151/566 (26.68%) |
| Fuzz driver | 6/6 (100%) / 3/4 (75.00%) | 6/6 (100%) / 1/2 (50.00%) |
| Bounded JSON parser | 280/328 (85.37%) / 233/364 (64.01%) | not entered by history mode |

The native ISO suite adds geometry, boundary, ownership, seeded, and bounded-set
coverage. A separate LLVM profile combined both fuzz modes with the sanitized
native suite. It measured 85/88 runtime lines (96.59%) and 70/80 runtime branch
outcomes (87.50%). The remaining paths include allocation failures and status
branches outside the generated command domain.

## Native build portability

The package `BUILD` passed under both GCC and Clang with 113,713 checks and no
failures. The launcher passed a PowerShell 7.6.1 syntax check. Native Windows
compilation and execution were unavailable in this Linux
environment.

## Fixed seed hashes

The fixture corpus starts with these malformed examples plus the copied
canonical fixture above.

| Seed | SHA-256 |
|---|---|
| `boolean-address.json` | `f0ed216315cf70230c7bf3cfe0dff191b21e1245ce063d64320a44fc84b33221` |
| `duplicate-key.json` | `716034626843f8d0e0403f30a42723caf15087ed4999ebfa9f2b3fbf74346044` |
| `invalid-utf8.json` | `7d4f8a6e9dfd5daf6de7cafb0dcad310bbfc39a91d22749110f697ce699ba39e` |
| `malformed-hex.json` | `7dfd0dd0d3d84cd63e69103704c6329a9a949656304d31139e728cef039295a0` |
| `missing-required-state.json` | `692450ca15db0235b576d1e195665dea525b1b262e5bdf5d264346a8d03b6027` |
| `out-of-range-geometry.json` | `ad65697cef6d06d06857bad9ddbd80166e180331e4d4bba2cca64156d4f14def` |
| `trailing-value.json` | `6243730518b5ae6ef38842814b317943033cdd28c09cb07ad3c6d51780621db4` |
| `truncated.json` | `5a25ff35b6d6a357567302776f42832a743341f35af5fbc173f8beadb9d92dd7` |
| `unknown-member.json` | `3c69d3b90e8e7cd9eee47c0c4c0b898516a2bc5986ca0d0d7868ff80f115633e` |
| `unknown-operation.json` | `1650e1e43eddaedbbb5ae0790ca76794086395cf74a25972656c633f7cd6c7dd` |

The binary history corpus contains `address-refusal`, `erase-limit`,
`mixed-read-erase`, `null-buffer`, `program-once`, `program-twice`, and
`wrong-length`:

| Seed | SHA-256 |
|---|---|
| `address-refusal` | `68265cca3dc3227d4486d7b88e28cd39b6c0a23f66f6e3fd3be91dac4f4158da` |
| `erase-limit` | `f18ae654426d024a6fe50cc2259be83332d3fd91a9cec5e56467f2c75d3e703b` |
| `mixed-read-erase` | `db1f54cc0f57bbbef0b94d8f108bce9ed970ec5f65904179b14ceef99d7042e6` |
| `null-buffer` | `cc338884ac217195ade7cef9e69914255beb3c67935f7513650c531ce253e9fb` |
| `program-once` | `847342f5e56efc411cd2ad6333eaf88fd9c264e106b4b66f13f884717d45d91a` |
| `program-twice` | `9f62c17310da2b120fcba4c4cafe72f372eaf1d8e1ab44c5d7ff4c9d84df975c` |
| `wrong-length` | `d45bf07e481c27d32333b0cf9674be8fe31af4147009ffb29bd781fbbe3b78db` |
