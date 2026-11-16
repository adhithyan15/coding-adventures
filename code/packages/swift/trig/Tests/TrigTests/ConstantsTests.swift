// ============================================================================
// ConstantsTests.swift — the bits of every constant TRIG-REF-1 pins
// ============================================================================
//
// PHY00a 6.5 (e): a port asserts the bits of every constant it embeds.  The
// expected values below are copied from the manifest of the shared fixture
// (and the manifest itself is read and compared, so a typo cannot hide in both
// places).  Swift builds each constant with `Double(bitPattern:)`, so this also
// guards against someone "simplifying" one to a decimal literal.

import XCTest
@testable import Trig

final class ConstantsTests: XCTestCase {

    private func manifestConstants() throws -> [String: UInt64] {
        var directory = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        while directory.path != "/" {
            let candidate = directory.appendingPathComponent(
                "code/specs/fixtures/trig-reference-v1/manifest.json")
            if FileManager.default.fileExists(atPath: candidate.path) {
                let data = try Data(contentsOf: candidate)
                let json = try JSONSerialization.jsonObject(with: data) as! [String: Any]
                XCTAssertEqual(json["algorithm"] as? String, "TRIG-REF-1")
                let table = json["constants"] as! [String: String]
                return table.mapValues { UInt64($0, radix: 16)! }
            }
            directory.deleteLastPathComponent()
        }
        throw NSError(domain: "TrigConstants", code: 1)
    }

    func testConstantsMatchTheFixtureManifest() throws {
        let c = try manifestConstants()
        XCTAssertEqual(PI.bitPattern, c["PI"])
        XCTAssertEqual(TWO_PI.bitPattern, c["TWO_PI"])
        XCTAssertEqual(HALF_PI.bitPattern, c["HALF_PI"])
        XCTAssertEqual(INV_PIO2.bitPattern, c["INV_PIO2"])
        XCTAssertEqual(PIO2.count, 5)
        for (i, piece) in PIO2.enumerated() {
            XCTAssertEqual(piece.bitPattern, c["PIO2_\(i + 1)"], "PIO2_\(i + 1)")
        }
        XCTAssertEqual(MAGIC.bitPattern, c["MAGIC"])
        XCTAssertEqual(LIMIT.bitPattern, c["LIMIT"])
        XCTAssertEqual(TINY.bitPattern, c["TINY"])
        XCTAssertEqual(SIN_COEFFS.count, 9)
        for (i, v) in SIN_COEFFS.enumerated() {
            XCTAssertEqual(v.bitPattern, c["S\(i + 1)"], "S\(i + 1)")
        }
        XCTAssertEqual(COS_COEFFS.count, 8)
        for (i, v) in COS_COEFFS.enumerated() {
            XCTAssertEqual(v.bitPattern, c["C\(i + 2)"], "C\(i + 2)")
        }
        XCTAssertEqual(RAD_PER_DEG.bitPattern, c["RAD_PER_DEG"])
        XCTAssertEqual(DEG_PER_RAD.bitPattern, c["DEG_PER_RAD"])
    }

    /// The same table written out here, independently of the manifest.
    func testConstantsMatchTheSpecTable() {
        XCTAssertEqual(PI.bitPattern, 0x400921FB54442D18)
        XCTAssertEqual(INV_PIO2.bitPattern, 0x3FE45F306DC9C883)
        XCTAssertEqual(MAGIC.bitPattern, 0x4338000000000000)
        XCTAssertEqual(LIMIT, 536870912.0)
        XCTAssertEqual(TINY, 7.450580596923828e-09)
        XCTAssertEqual(SIN_COEFFS[0].bitPattern, 0xBFC5555555555555)
        XCTAssertEqual(SIN_COEFFS[8].bitPattern, 0xBC62F49B46814157)
        XCTAssertEqual(COS_COEFFS[0].bitPattern, 0x3FA5555555555555)
        XCTAssertEqual(COS_COEFFS[7].bitPattern, 0xBCA6827863B97D97)
    }

    /// Decimal literals the source still writes (tan pole, sqrt and atan
    /// tolerances) parse to the pinned bit patterns.
    func testLiteralBits() {
        XCTAssertEqual((1e-15).bitPattern, 0x3CD203AF9EE75616)
        XCTAssertEqual((1.0e308).bitPattern, 0x7FE1CCF385EBC8A0)
        XCTAssertEqual((1e-17).bitPattern, 0x3C670EF54646D497)
        XCTAssertEqual((1e-300).bitPattern, 0x01A56E1FC2F8F359)
        XCTAssertEqual((7.450580596923828e-9).bitPattern, 0x3E40000000000000)
    }

    func testDegreeConstantsEqualTheDivisions() {
        XCTAssertEqual(RAD_PER_DEG, PI / 180.0)
        XCTAssertEqual(DEG_PER_RAD, 180.0 / PI)
    }

    // MARK: PHY00a 3.3 contract, asserted directly

    func testSinNegativeZeroAndTinyAndDomain() {
        XCTAssertEqual(Trig.sin(-0.0).bitPattern, 0x8000000000000000)
        XCTAssertEqual(Trig.sin(0.0).bitPattern, 0)
        XCTAssertEqual(Trig.cos(0.0), 1.0)
        XCTAssertEqual(Trig.cos(-0.0), 1.0)
        for x in [Double.nan, .infinity, -.infinity, 1e20, 536870913.0, -536870913.0] {
            XCTAssertTrue(Trig.sin(x).isNaN, "sin(\(x))")
            XCTAssertTrue(Trig.cos(x).isNaN, "cos(\(x))")
            XCTAssertTrue(Trig.tan(x).isNaN, "tan(\(x))")
        }
        // The last in-domain argument (worked example of PHY00a 3.10).
        XCTAssertEqual(Trig.sin(536870912.0).bitPattern, 0x3FD4E67C0E2622DF)
        XCTAssertEqual(Trig.cos(536870912.0).bitPattern, 0xBFEE3EDD2DFEEF90)
        XCTAssertEqual(Trig.sin(1.0).bitPattern, 0x3FEAED548F090CEE)
        XCTAssertEqual(Trig.sin(PI).bitPattern, 0x3CA1A62633145C07)
    }

    func testAtan2OfNaNIsNaN() {
        XCTAssertTrue(Trig.atan2(.nan, 1.0).isNaN)
        XCTAssertTrue(Trig.atan2(1.0, .nan).isNaN)
        XCTAssertTrue(Trig.atan2(.nan, .nan).isNaN)
        XCTAssertTrue(Trig.atan2(0.0, .nan).isNaN)
        XCTAssertEqual(Trig.atan2(-0.0, -1.0), PI)
    }

    func testConversionsAreExactAtLandmarks() {
        XCTAssertEqual(Trig.radians(180.0), PI)
        XCTAssertEqual(Trig.degrees(PI), 180.0)
        XCTAssertEqual(Trig.radians(-0.0).bitPattern, 0x8000000000000000)
        XCTAssertEqual(Trig.degrees(-0.0).bitPattern, 0x8000000000000000)
    }

    func testSqrtCheckedThrowsExactlyWhereSqrtTraps() {
        XCTAssertThrowsError(try Trig.sqrtChecked(-1.0)) {
            XCTAssertEqual($0 as? TrigError, .negativeSquareRoot)
        }
        XCTAssertThrowsError(try Trig.sqrtChecked(-.infinity))
        XCTAssertThrowsError(try Trig.sqrtChecked(-.leastNonzeroMagnitude))
        XCTAssertEqual(try Trig.sqrtChecked(-0.0).bitPattern, 0x8000000000000000)
        XCTAssertEqual(try Trig.sqrtChecked(4.0), Trig.sqrt(4.0))
        XCTAssertTrue(try Trig.sqrtChecked(.nan).isNaN)
        XCTAssertEqual(try Trig.sqrtChecked(.infinity), .infinity)
    }
}
