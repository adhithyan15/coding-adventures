import XCTest
@testable import HashFunctions

final class HashFunctionsTests: XCTestCase {
    func testFNV1aKnownVectorsAndUTF8Convenience() {
        XCTAssertEqual(fnv1a32([]), 0x811C_9DC5)
        XCTAssertEqual(fnv1a32("a"), 0xE40C_292C)
        XCTAssertEqual(fnv1a32("abc"), 0x1A47_E90B)
        XCTAssertEqual(fnv1a32("hello"), 1_335_831_723)
        XCTAssertEqual(fnv1a32("foobar"), 3_214_735_720)

        XCTAssertEqual(fnv1a64([]), 0xCBF2_9CE4_8422_2325)
        XCTAssertEqual(fnv1a64("a"), 0xAF63_DC4C_8601_EC8C)
        XCTAssertEqual(fnv1a64("abc"), 0xE71F_A219_0541_574B)
        XCTAssertEqual(fnv1a64("hello"), 0xA430_D846_80AA_BD0B)

        let utf8 = Array("café".utf8)
        XCTAssertEqual(fnv1a32("café"), fnv1a32(utf8))
        XCTAssertEqual(hashStringFNV1a32("hello"), fnv1a32("hello"))
    }

    func testDJB2KnownVectorsAndWrapping() {
        XCTAssertEqual(djb2([]), 5_381)
        XCTAssertEqual(djb2("a"), 177_670)
        XCTAssertEqual(djb2("abc"), 193_485_963)
        XCTAssertEqual(djb2("hello"), 210_714_636_441)

        let longInput = [UInt8](repeating: 0xFF, count: 1_000)
        XCTAssertEqual(djb2(longInput), djb2(longInput))
    }

    func testPolynomialRollingVectorsParametersAndWideIntermediate() throws {
        XCTAssertEqual(try polynomialRolling([]), 0)
        XCTAssertEqual(try polynomialRolling("a"), 97)
        XCTAssertEqual(try polynomialRolling("ab"), 3_105)
        XCTAssertEqual(try polynomialRolling("abc"), 96_354)
        XCTAssertEqual(
            try polynomialRolling("abc", base: 37, modulus: 1_000_000_007),
            (97 * 37 + 98) * 37 + 99
        )
        XCTAssertEqual(
            try polynomialRolling(
                [255, 255, 255],
                base: UInt64.max - 1,
                modulus: UInt64.max - 58
            ),
            843_285
        )
        XCTAssertThrowsError(try polynomialRolling([1], modulus: 0)) { error in
            XCTAssertEqual(error as? HashFunctionError, .invalidModulus)
        }
    }

    func testMurmur3KnownVectorsSeedsAndEveryTailLength() {
        XCTAssertEqual(murmur3_32([]), 0)
        XCTAssertEqual(murmur3_32([], seed: 1), 0x514E_28B7)
        XCTAssertEqual(murmur3_32("a"), 0x3C25_69B2)
        XCTAssertEqual(murmur3_32("abc"), 0xB3DD_93FA)
        XCTAssertEqual(murmur3_32("abcd"), 0x43ED_676A)
        XCTAssertNotEqual(murmur3_32("hello"), murmur3_32("hello", seed: 1))

        let hashes = (0...11).map { length in
            murmur3_32(Array((0..<length).map(UInt8.init)))
        }
        XCTAssertEqual(Set(hashes).count, hashes.count)
    }

    func testAllUpstreamSipHash24Vectors() throws {
        let expected: [UInt64] = [
            0x726F_DB47_DD0E_0E31, 0x74F8_39C5_93DC_67FD,
            0x0D6C_8009_D9A9_4F5A, 0x8567_6696_D7FB_7E2D,
            0xCF27_94E0_2771_87B7, 0x1876_5564_CD99_A68D,
            0xCBC9_466E_58FE_E3CE, 0xAB02_00F5_8B01_D137,
            0x93F5_F579_9A93_2462, 0x9E00_82DF_0BA9_E4B0,
            0x7A5D_BBC5_94DD_B9F3, 0xF4B3_2F46_226B_ADA7,
            0x751E_8FBC_860E_E5FB, 0x14EA_5627_C084_3D90,
            0xF723_CA90_8E7A_F2EE, 0xA129_CA61_49BE_45E5,
            0x3F2A_CC7F_57C2_9BDB, 0x699A_E9F5_2CBE_4794,
            0x4BC1_B3F0_968D_D39C, 0xBB6D_C91D_A779_61BD,
            0xBED6_5CF2_1AA2_EE98, 0xD0F2_CBB0_2E3B_67C7,
            0x9353_6795_E3A3_3E88, 0xA80C_038C_CD5C_CEC8,
            0xB8AD_50C6_F649_AF94, 0xBCE1_92DE_8A85_B8EA,
            0x17D8_35B8_5BBB_15F3, 0x2F2E_6163_076B_CFAD,
            0xDE4D_AAAC_A71D_C9A5, 0xA6A2_5066_8795_6571,
            0xAD87_A353_5C49_EF28, 0x32D8_92FA_D841_C342,
            0x7127_512F_72F2_7CCE, 0xA7F3_2346_F959_78E3,
            0x12E0_B01A_BB05_1238, 0x15E0_34D4_0FA1_97AE,
            0x314D_FFBE_0815_A3B4, 0x0279_90F0_2962_3981,
            0xCADC_D4E5_9EF4_0C4D, 0x9ABF_D876_6A33_735C,
            0x0E3E_A96B_5304_A7D0, 0xAD0C_42D6_FC58_5992,
            0x1873_06C8_9BC2_15A9, 0xD4A6_0ABC_F379_2B95,
            0xF935_451D_E4F2_1DF2, 0xA953_8F04_1975_5787,
            0xDB9A_CDDF_F56C_A510, 0xD06C_98CD_5C09_75EB,
            0xE612_A3CB_9ECB_A951, 0xC766_E62C_FCAD_AF96,
            0xEE64_435A_9752_FE72, 0xA192_D576_B245_165A,
            0x0A87_87BF_8ECB_74B2, 0x81B3_E73D_20B4_9B6F,
            0x7FA8_220B_A3B2_ECEA, 0x2457_31C1_3CA4_2499,
            0xB78D_BFAF_3A8D_83BD, 0xEA1A_D565_322A_1A0B,
            0x60E6_1C23_A379_5013, 0x6606_D7E4_4628_2B93,
            0x6CA4_ECB1_5C5F_91E1, 0x9F62_6DA1_5C96_25F3,
            0xE51B_3860_8EF2_5F57, 0x958A_324C_EB06_4572,
        ]
        let key = Array(UInt8(0)...UInt8(15))
        for length in 0..<expected.count {
            let message = Array((0..<length).map(UInt8.init))
            XCTAssertEqual(
                try sipHash24(message, key: key),
                expected[length],
                "reference vector length \(length)"
            )
        }
        XCTAssertEqual(
            try hashStringSipHash("hello", key: key),
            try sipHash24(Array("hello".utf8), key: key)
        )
        XCTAssertThrowsError(try sipHash24([], key: [0])) { error in
            XCTAssertEqual(error as? HashFunctionError, .invalidSipHashKeyLength(1))
        }
    }

    func testStrategyTypesForwardToFreeFunctions() throws {
        let input = Array("abc".utf8)
        let strategies: [any HashFunction] = [
            FNV1a32(),
            FNV1a64(),
            DJB2Hash(),
            PolynomialRollingHash(),
            Murmur3Hash(seed: 7),
            try SipHash24(key: [UInt8](repeating: 0, count: 16)),
        ]

        XCTAssertEqual(strategies.map(\.outputBits), [32, 64, 64, 64, 32, 64])
        XCTAssertEqual(strategies[0].hash(input), UInt64(fnv1a32(input)))
        XCTAssertEqual(strategies[1].hash(input), fnv1a64(input))
        XCTAssertEqual(strategies[2].hash(input), djb2(input))
        XCTAssertEqual(strategies[3].hash(input), try polynomialRolling(input))
        XCTAssertEqual(strategies[4].hash(input), UInt64(murmur3_32(input, seed: 7)))
        XCTAssertEqual(strategies[5].hash(input), try sipHash24(input, key: [UInt8](repeating: 0, count: 16)))

        XCTAssertThrowsError(try PolynomialRollingHash(base: 31, modulus: 0))
        XCTAssertThrowsError(try SipHash24(key: []))
    }

    func testAvalancheScoreUsesOnlyExplicitCallerSamples() throws {
        let samples: [[UInt8]] = [
            Array("abcdefgh".utf8),
            Array("ABCDEFGH".utf8),
            [0, 1, 2, 3, 4, 5, 6, 7],
        ]
        XCTAssertEqual(
            try avalancheScore(hashFunction: { _ in 0 }, outputBits: 32, samples: samples),
            0
        )
        let first = try avalancheScore(
            hashFunction: { UInt64(murmur3_32($0)) },
            outputBits: 32,
            samples: samples
        )
        let second = try avalancheScore(
            hashFunction: { UInt64(murmur3_32($0)) },
            outputBits: 32,
            samples: samples
        )
        XCTAssertEqual(first, second)
        XCTAssertGreaterThan(first, 0.35)
        XCTAssertLessThan(first, 0.65)

        XCTAssertThrowsError(
            try avalancheScore(hashFunction: { _ in 0 }, outputBits: 0, samples: samples)
        )
        XCTAssertThrowsError(
            try avalancheScore(hashFunction: { _ in 0 }, outputBits: 65, samples: samples)
        )
        XCTAssertThrowsError(
            try avalancheScore(hashFunction: { _ in 0 }, outputBits: 32, samples: [])
        )
        XCTAssertThrowsError(
            try avalancheScore(hashFunction: { _ in 0 }, outputBits: 32, samples: [[]])
        )
    }

    func testDistributionTestExactMathAndInputValidation() throws {
        let inputs = ["a", "b", "c", "d"].map { Array($0.utf8) }
        XCTAssertEqual(
            try distributionTest(hashFunction: { _ in 0 }, inputs: inputs, numBuckets: 4),
            12
        )
        XCTAssertGreaterThanOrEqual(
            try distributionTest(hashFunction: fnv1a64, inputs: inputs, numBuckets: 4),
            0
        )
        XCTAssertThrowsError(
            try distributionTest(hashFunction: fnv1a64, inputs: [], numBuckets: 4)
        )
        XCTAssertThrowsError(
            try distributionTest(hashFunction: fnv1a64, inputs: inputs, numBuckets: 0)
        )
    }
}
