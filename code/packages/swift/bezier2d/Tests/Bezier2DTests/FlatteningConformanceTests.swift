import Foundation
import XCTest
import Point2D
@testable import Bezier2D

private struct FlatteningFixture: Decodable {
    let version: Int
    let absoluteTolerance: Double
    let maxDepth: Int
    let maxSubdivisions: Int
    let cases: [FlatteningCase]
}

private struct FlatteningCase: Decodable {
    let id: String
    let degree: Int
    let controlPoints: [[Double]]
    let tolerance: Double
    let expectedDisposition: String
    let expectedTermination: String?
    let witnessT: Double?
    let expectedWitness: [Double]?
    let expectedWitnessDistance: Double?
}

final class FlatteningConformanceTests: XCTestCase {
    private static let maximumFixtureBytes = 65_536

    func testNeutralCasesAndPinnedEnvelope() throws {
        let fixture = try Self.loadFixture()
        XCTAssertEqual(fixture.version, 1)
        XCTAssertEqual(fixture.absoluteTolerance, 1e-12)
        XCTAssertEqual(fixture.maxDepth, 32)
        XCTAssertEqual(fixture.maxSubdivisions, 65_535)
        XCTAssertEqual(fixture.cases.count, 9)

        for testCase in fixture.cases where testCase.expectedTermination == nil {
            let controls = try testCase.controlPoints.map(Self.point)
            if testCase.expectedDisposition == "invalid-tolerance" {
                XCTAssertThrowsError(try Self.flatten(testCase, controls: controls)) { error in
                    XCTAssertEqual(error as? FlatteningError, .invalidTolerance, testCase.id)
                }
                continue
            }

            let points = try Self.flatten(testCase, controls: controls)
            XCTAssertEqual(points.first, controls.first, testCase.id)
            XCTAssertEqual(points.last, controls.last, testCase.id)
            XCTAssertTrue(points.allSatisfy { $0.x.isFinite && $0.y.isFinite }, testCase.id)
            if testCase.expectedDisposition == "chord" {
                XCTAssertEqual(points.count, 2, testCase.id)
            } else {
                XCTAssertGreaterThan(points.count, 2, testCase.id)
            }
            try Self.assertWitness(testCase, controls: controls)
        }
    }

    func testBudgetExhaustionFailsDeterministically() throws {
        let fixture = try Self.loadFixture()
        let testCase = try XCTUnwrap(
            fixture.cases.first { $0.expectedTermination == "budget-error" })
        let controls = try testCase.controlPoints.map(Self.point)
        XCTAssertThrowsError(try Self.flatten(testCase, controls: controls)) { error in
            XCTAssertEqual(error as? FlatteningError, .budgetExceeded)
        }
    }

    func testNonfiniteControlsFailBeforeSubdivision() {
        let origin = Point(0, 0)
        XCTAssertThrowsError(try QuadraticBezier(
            origin, Point(.nan, 1), Point(1, 0)).polyline(tolerance: 0.1)) { error in
            XCTAssertEqual(error as? FlatteningError, .nonFiniteControlPoint)
        }
        XCTAssertThrowsError(try CubicBezier(
            origin, Point(0, 1), Point(.infinity, -1), origin).polyline(tolerance: 0.1)) { error in
            XCTAssertEqual(error as? FlatteningError, .nonFiniteControlPoint)
        }
    }

    func testFiniteInputsThatOverflowDerivedGeometryFailClosed() {
        let negative = Point(-Double.greatestFiniteMagnitude, 0)
        let positive = Point(Double.greatestFiniteMagnitude, 0)
        XCTAssertThrowsError(try QuadraticBezier(
            negative, Point(0, 1), positive).polyline(tolerance: 0.1)) { error in
            XCTAssertEqual(error as? FlatteningError, .nonFiniteGeometry)
        }
        XCTAssertThrowsError(try CubicBezier(
            negative, Point(0, 1), Point(0, -1), positive).polyline(tolerance: 0.1)) { error in
            XCTAssertEqual(error as? FlatteningError, .nonFiniteGeometry)
        }
    }

    func testDepthBudgetFailsBeforeReturningAPartialPolyline() {
        let quadratic = QuadraticBezier(Point(0, 0), Point(0.5, 1), Point(1, 0))
        let cubic = CubicBezier(Point(0, 0), Point(0, 1), Point(1, 1), Point(1, 0))
        XCTAssertThrowsError(try quadratic.polyline(tolerance: .leastNonzeroMagnitude)) { error in
            XCTAssertEqual(error as? FlatteningError, .budgetExceeded)
        }
        XCTAssertThrowsError(try cubic.polyline(tolerance: .leastNonzeroMagnitude)) { error in
            XCTAssertEqual(error as? FlatteningError, .budgetExceeded)
        }
    }

    private static func flatten(_ testCase: FlatteningCase, controls: [Point]) throws -> [Point] {
        switch (testCase.degree, controls.count) {
        case (2, 3):
            return try QuadraticBezier(controls[0], controls[1], controls[2])
                .polyline(tolerance: testCase.tolerance)
        case (3, 4):
            return try CubicBezier(controls[0], controls[1], controls[2], controls[3])
                .polyline(tolerance: testCase.tolerance)
        default:
            throw NSError(domain: "FlatteningConformance", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "unsupported fixture shape \(testCase.id)"])
        }
    }

    private static func assertWitness(_ testCase: FlatteningCase, controls: [Point]) throws {
        let amount = try XCTUnwrap(testCase.witnessT, testCase.id)
        let expected = try point(XCTUnwrap(testCase.expectedWitness, testCase.id))
        let expectedDistance = try XCTUnwrap(testCase.expectedWitnessDistance, testCase.id)
        let actual: Point
        if testCase.degree == 2 {
            actual = QuadraticBezier(controls[0], controls[1], controls[2]).eval(amount)
        } else {
            actual = CubicBezier(controls[0], controls[1], controls[2], controls[3]).eval(amount)
        }
        XCTAssertEqual(actual.x, expected.x, accuracy: 1e-12, testCase.id)
        XCTAssertEqual(actual.y, expected.y, accuracy: 1e-12, testCase.id)
        XCTAssertEqual(
            distanceToSegment(actual, controls[0], controls[controls.count - 1]),
            expectedDistance,
            accuracy: 1e-12,
            testCase.id)
    }

    private static func distanceToSegment(_ point: Point, _ start: Point, _ end: Point) -> Double {
        let dx = end.x - start.x
        let dy = end.y - start.y
        let lengthSquared = dx * dx + dy * dy
        if lengthSquared == 0 { return point.distance(to: start) }
        let projection = min(1, max(0,
            ((point.x - start.x) * dx + (point.y - start.y) * dy) / lengthSquared))
        return point.distance(to: Point(start.x + projection * dx, start.y + projection * dy))
    }

    private static func point(_ coordinates: [Double]) throws -> Point {
        guard coordinates.count == 2 else {
            throw NSError(domain: "FlatteningConformance", code: 2,
                          userInfo: [NSLocalizedDescriptionKey: "fixture point must have two coordinates"])
        }
        return Point(coordinates[0], coordinates[1])
    }

    private static func loadFixture() throws -> FlatteningFixture {
        var directory = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
        let manager = FileManager.default
        var fixtureURL: URL?
        while directory.path != "/" {
            let candidate = directory
                .appendingPathComponent("code/specs/fixtures/bezier2d-flattening-v1/cases.json")
            if manager.fileExists(atPath: candidate.path) {
                fixtureURL = candidate
                break
            }
            directory.deleteLastPathComponent()
        }
        let url = try XCTUnwrap(fixtureURL, "could not locate the checked-in G2D02 flattening corpus")
        let handle = try FileHandle(forReadingFrom: url)
        defer { try? handle.close() }
        let data = try handle.read(upToCount: maximumFixtureBytes + 1) ?? Data()
        guard data.count <= maximumFixtureBytes else {
            throw NSError(domain: "FlatteningConformance", code: 3,
                          userInfo: [NSLocalizedDescriptionKey: "fixture exceeds bounded reader limit"])
        }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try decoder.decode(FlatteningFixture.self, from: data)
    }
}
