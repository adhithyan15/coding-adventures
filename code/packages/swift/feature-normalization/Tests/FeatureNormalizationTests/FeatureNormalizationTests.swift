import XCTest
@testable import FeatureNormalization

final class FeatureNormalizationTests: XCTestCase {
    private let rows = [
        [1000.0, 3.0, 1.0],
        [1500.0, 4.0, 0.0],
        [2000.0, 5.0, 1.0],
    ]

    func testStandardScalerCentersAndScalesColumns() throws {
        let scaler = try FeatureNormalization.fitStandardScaler(rows)
        XCTAssertEqual(1500.0, scaler.means[0], accuracy: 1.0e-9)
        XCTAssertEqual(4.0, scaler.means[1], accuracy: 1.0e-9)

        let transformed = try FeatureNormalization.transformStandard(rows, scaler: scaler)
        XCTAssertEqual(-1.224744871391589, transformed[0][0], accuracy: 1.0e-9)
        XCTAssertEqual(0.0, transformed[1][0], accuracy: 1.0e-9)
        XCTAssertEqual(1.224744871391589, transformed[2][0], accuracy: 1.0e-9)
    }

    func testMinMaxScalerMapsColumnsToUnitRange() throws {
        let transformed = try FeatureNormalization.transformMinMax(rows, scaler: FeatureNormalization.fitMinMaxScaler(rows))

        XCTAssertEqual([0.0, 0.0, 1.0], transformed[0])
        XCTAssertEqual([0.5, 0.5, 0.0], transformed[1])
        XCTAssertEqual([1.0, 1.0, 1.0], transformed[2])
    }

    func testConstantColumnsMapToZero() throws {
        let data = [[1.0, 7.0], [2.0, 7.0]]
        let standard = try FeatureNormalization.transformStandard(data, scaler: FeatureNormalization.fitStandardScaler(data))
        let minMax = try FeatureNormalization.transformMinMax(data, scaler: FeatureNormalization.fitMinMaxScaler(data))

        XCTAssertEqual(0.0, standard[0][1], accuracy: 1.0e-9)
        XCTAssertEqual(0.0, minMax[0][1], accuracy: 1.0e-9)
    }
}
