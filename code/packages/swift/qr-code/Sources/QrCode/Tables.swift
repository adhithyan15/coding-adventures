// Tables.swift — ISO/IEC 18004:2015 QR Code lookup tables
//
// This file contains every table from the QR Code standard that the encoder
// needs. Embedding them as Swift constants is far safer than computing them at
// runtime: the values are exact copies from the ISO standard, easy to audit,
// and immune to floating-point or off-by-one computation bugs.
//
// ============================================================================
// What tables are here
// ============================================================================
//
//   1. ECC_CODEWORDS_PER_BLOCK — How many ECC bytes each block gets.
//   2. NUM_BLOCKS              — How many RS blocks per version/ECC combination.
//   3. ALIGNMENT_POSITIONS     — Where to place 5×5 alignment patterns.
//   4. REMAINDER_BITS          — Extra zero bits after interleaved codewords.
//
// The capacity of a version/ECC combination is derived from these tables.
// The encoder never hard-codes capacities; it always computes from formulas.
//
// ============================================================================

// MARK: - ECC codewords per block
//
// Indexed by [eccIndex][version]. eccIndex: 0=L, 1=M, 2=Q, 3=H.
// Version index 0 is a sentinel placeholder (QR versions are 1-based).
//
// Source: ISO/IEC 18004:2015 Table 9.
//
// These values tell the RS encoder how many error-correction bytes to produce
// for each block. Each block is encoded independently with its own RS
// computation, so errors in one block don't affect the RS computation for
// adjacent blocks.
//
// Example: version 5, ECC=Q → ECC_CODEWORDS_PER_BLOCK[2][5] = 18.
// Every block in version 5 Q has exactly 18 ECC bytes.
let ECC_CODEWORDS_PER_BLOCK: [[Int]] = [
    // L:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  7, 10, 15, 20, 26, 18, 20, 24, 30, 18, 20, 24, 26, 30, 22, 24, 28, 30, 28, 28, 28, 28, 30, 30, 26, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    // M:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26, 30, 22, 22, 24, 24, 28, 28, 26, 26, 26, 26, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28, 28],
    // Q:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 13, 22, 18, 26, 18, 24, 18, 22, 20, 24, 28, 26, 24, 20, 30, 24, 28, 28, 26, 30, 28, 30, 30, 30, 30, 28, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
    // H:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1, 17, 28, 22, 16, 22, 28, 26, 26, 24, 28, 24, 28, 22, 24, 24, 30, 28, 28, 26, 28, 30, 24, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30, 30],
]

// MARK: - Number of RS blocks
//
// Indexed by [eccIndex][version]. eccIndex: 0=L, 1=M, 2=Q, 3=H.
// Version index 0 is a sentinel placeholder.
//
// Source: ISO/IEC 18004:2015 Table 9.
//
// The data codewords are split evenly across this many independent blocks.
// Splitting into blocks provides a major advantage: a burst error (e.g. a
// scratch that destroys 50 consecutive modules) will only wipe out 1-2 blocks.
// The other blocks are unaffected. Each block has enough ECC to recover from
// a partial loss of its own data.
//
// Example: version 5, ECC=Q → NUM_BLOCKS[2][5] = 4 blocks.
// The 64 total data codewords are split: 2 blocks of 15, 2 blocks of 16.
let NUM_BLOCKS: [[Int]] = [
    // L:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  1,  1,  1,  1,  1,  2,  2,  2,  2,  4,  4,  4,  4,  4,  6,  6,  6,  6,  7,  8,  8,  9,  9, 10, 12, 12, 12, 13, 14, 15, 16, 17, 18, 19, 19, 20, 21, 22, 24, 25],
    // M:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  1,  1,  1,  2,  2,  4,  4,  4,  5,  5,  5,  8,  9,  9, 10, 10, 11, 13, 14, 16, 17, 17, 18, 20, 21, 23, 25, 26, 28, 29, 31, 33, 35, 37, 38, 40, 43, 45, 47, 49],
    // Q:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  1,  1,  2,  2,  4,  4,  6,  6,  8,  8,  8, 10, 12, 16, 12, 17, 16, 18, 21, 20, 23, 23, 25, 27, 29, 34, 34, 35, 38, 40, 43, 45, 48, 51, 53, 56, 59, 62, 65, 68],
    // H:  0   1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
    [-1,  1,  1,  2,  4,  4,  4,  5,  6,  8,  8, 11, 11, 16, 16, 18, 16, 19, 21, 25, 25, 25, 34, 30, 32, 35, 37, 40, 42, 45, 48, 51, 54, 57, 60, 63, 66, 70, 74, 77, 80],
]

// MARK: - Alignment pattern centre coordinates
//
// Indexed by (version - 1). Version 1 has no alignment patterns.
// For version v, take the cross-product of ALIGNMENT_POSITIONS[v-1] with
// itself: all pairs (row, col). Any pair whose centre falls on an already-
// reserved module (finder, separator, timing) must be skipped.
//
// Source: ISO/IEC 18004:2015 Annex E, Table E.1.
//
// ## What are alignment patterns?
//
// Alignment patterns are small 5×5 finder-like squares embedded in the data
// area of large QR codes. Their job: give the decoder a set of known positions
// it can use to correct for optical distortion.
//
// A smartphone camera sees QR codes at an angle, through a warped lens, on a
// curved surface. The decoder needs to "unwarp" the perspective before reading
// the data. With only three corner finders it can correct for uniform
// rotation and scaling. Alignment patterns add interior reference points,
// enabling correction of complex lens distortion and perspective skew.
//
// Version 1 (21×21) is small enough that distortion never matters. Version 7+
// can be large (45+ modules per side), so several alignment patterns are
// needed.
let ALIGNMENT_POSITIONS: [[Int]] = [
    [],                             // v1  — none
    [6, 18],                        // v2
    [6, 22],                        // v3
    [6, 26],                        // v4
    [6, 30],                        // v5
    [6, 34],                        // v6
    [6, 22, 38],                    // v7
    [6, 24, 42],                    // v8
    [6, 26, 46],                    // v9
    [6, 28, 50],                    // v10
    [6, 30, 54],                    // v11
    [6, 32, 58],                    // v12
    [6, 34, 62],                    // v13
    [6, 26, 46, 66],                // v14
    [6, 26, 48, 70],                // v15
    [6, 26, 50, 74],                // v16
    [6, 30, 54, 78],                // v17
    [6, 30, 56, 82],                // v18
    [6, 30, 58, 86],                // v19
    [6, 34, 62, 90],                // v20
    [6, 28, 50, 72, 94],            // v21
    [6, 26, 50, 74, 98],            // v22
    [6, 30, 54, 78, 102],           // v23
    [6, 28, 54, 80, 106],           // v24
    [6, 32, 58, 84, 110],           // v25
    [6, 30, 58, 86, 114],           // v26
    [6, 34, 62, 90, 118],           // v27
    [6, 26, 50, 74, 98, 122],       // v28
    [6, 30, 54, 78, 102, 126],      // v29
    [6, 26, 52, 78, 104, 130],      // v30
    [6, 30, 56, 82, 108, 134],      // v31
    [6, 34, 60, 86, 112, 138],      // v32
    [6, 30, 58, 86, 114, 142],      // v33
    [6, 34, 62, 90, 118, 146],      // v34
    [6, 30, 54, 78, 102, 126, 150], // v35
    [6, 24, 50, 76, 102, 128, 154], // v36
    [6, 28, 54, 80, 106, 132, 158], // v37
    [6, 32, 58, 84, 110, 136, 162], // v38
    [6, 26, 54, 82, 110, 138, 166], // v39
    [6, 30, 58, 86, 114, 142, 170], // v40
]

// MARK: - Remainder bits
//
// After the interleaved codeword stream is placed in the grid, some versions
// have a few "leftover" module positions. The standard specifies that these
// are filled with zero (light) bits. They are not ECC bits; they just complete
// the last module group.
//
// Source: ISO/IEC 18004:2015 Table 1.
//
// Formula: numRawDataModules(version) % 8
// But the per-version values from the standard are given for reference:
//
//   v1:  0  v2:  7  v3:  7  v4:  7  v5:  7  v6:  0  v7:  0  v8:  0
//   v9:  0  v10: 0  v11: 0  v12: 0  v13: 0  v14: 3  v15: 3  v16: 3
//   v17: 3  v18: 3  v19: 3  v20: 3  v21: 4  v22: 4  v23: 4  v24: 4
//   v25: 4  v26: 4  v27: 4  v28: 3  v29: 3  v30: 3  v31: 3  v32: 3
//   v33: 3  v34: 3  v35: 0  v36: 0  v37: 0  v38: 0  v39: 0  v40: 0
//
// We compute this from numRawDataModules() rather than a separate table.
