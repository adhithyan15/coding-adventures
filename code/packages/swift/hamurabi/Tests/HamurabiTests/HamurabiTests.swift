// ============================================================================
// HamurabiTests.swift — Unit and Security Tests for Hamurabi
// ============================================================================

import XCTest
@testable import Hamurabi

final class HamurabiTests: XCTestCase {
    private func assertError(
        _ res: Result<Void, HamurabiError>,
        _ expected: HamurabiError,
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        switch res {
        case .failure(let err):
            XCTAssertEqual(err, expected, file: file, line: line)
        case .success:
            XCTFail("Expected failure \(expected), got success", file: file, line: line)
        }
    }

    func testInitialState() {
        let state = newGame(seed: 1337)
        XCTAssertEqual(state.year, 1)
        XCTAssertEqual(state.population, 100)
        XCTAssertEqual(state.bushels, 2800)
        XCTAssertEqual(state.acres, 1000)
        XCTAssertGreaterThanOrEqual(state.landPrice, 17)
        XCTAssertLessThanOrEqual(state.landPrice, 26)
        XCTAssertEqual(state.harvestPerAcre, 3)
        XCTAssertEqual(state.ratsEaten, 200)
        XCTAssertEqual(state.starvedLastYear, 0)
        XCTAssertEqual(state.starvedTotal, 0)
        XCTAssertEqual(state.percentStarvedSum, 0)
        XCTAssertEqual(state.immigrantsLastYear, 5)
        XCTAssertFalse(state.plagueStruck)
        XCTAssertEqual(state.status, .playing)
        XCTAssertEqual(state.rngInc & 1, 1)
    }

    func testAllEightErrorArms() {
        let state = newGame(seed: 1337)
        let normalOrder = TurnOrder(acresDelta: 0, bushelsToFeed: 2000, acresToPlant: 100)

        // 1. GameAlreadyEnded
        var wonState = state
        wonState.status = .won(rating: .charlemagne)
        assertError(validateOrder(state: wonState, order: normalOrder), .gameAlreadyEnded)
        switch stepYear(state: wonState, order: normalOrder) {
        case .failure(let err):
            XCTAssertEqual(err, .gameAlreadyEnded)
        case .success:
            XCTFail("Expected failure")
        }

        // 2. InvalidOrderBounds (corrupted land price or overflow)
        var badPriceState = state
        badPriceState.landPrice = 10 // valid range 17..=26
        assertError(validateOrder(state: badPriceState, order: normalOrder), .invalidOrderBounds)

        // 3. InsufficientBushelsForLand
        let buyTooMuch = TurnOrder(acresDelta: 200, bushelsToFeed: 0, acresToPlant: 0)
        assertError(validateOrder(state: state, order: buyTooMuch), .insufficientBushelsForLand)

        // 4. InsufficientAcresToSell
        let sellTooMuch = TurnOrder(acresDelta: -1001, bushelsToFeed: 0, acresToPlant: 0)
        assertError(validateOrder(state: state, order: sellTooMuch), .insufficientAcresToSell)

        // 5. InsufficientBushelsToFeed
        let feedTooMuch = TurnOrder(acresDelta: 0, bushelsToFeed: 2801, acresToPlant: 0)
        assertError(validateOrder(state: state, order: feedTooMuch), .insufficientBushelsToFeed)

        // 6. InsufficientAcresToPlant
        let plantTooMuchAcres = TurnOrder(acresDelta: 0, bushelsToFeed: 1000, acresToPlant: 1001)
        assertError(validateOrder(state: state, order: plantTooMuchAcres), .insufficientAcresToPlant)

        // 7. InsufficientPopulationToPlant (1 person tends 10 acres)
        var smallPopState = state
        smallPopState.population = 50 // can tend 500
        let plantTooMuchPop = TurnOrder(acresDelta: 0, bushelsToFeed: 1000, acresToPlant: 501)
        assertError(validateOrder(state: smallPopState, order: plantTooMuchPop), .insufficientPopulationToPlant)

        // 8. InsufficientGrainForSeed (1 bushel seeds 2 acres)
        let seedStarvedOrder = TurnOrder(acresDelta: 0, bushelsToFeed: 2500, acresToPlant: 800)
        assertError(validateOrder(state: state, order: seedStarvedOrder), .insufficientGrainForSeed)
    }

    func testCWE190OverflowGuards() {
        let state = newGame(seed: 42)

        // Huge buy: Int32.max acresDelta
        let hugeBuy = TurnOrder(acresDelta: Int32.max, bushelsToFeed: 0, acresToPlant: 0)
        assertError(validateOrder(state: state, order: hugeBuy), .insufficientBushelsForLand)

        // Int32.min negation guard: -Int32.min must not trap (crash)
        let int32MinOrder = TurnOrder(acresDelta: Int32.min, bushelsToFeed: 0, acresToPlant: 0)
        assertError(validateOrder(state: state, order: int32MinOrder), .insufficientAcresToSell)

        // Huge plant: UInt32.max
        let hugePlant = TurnOrder(acresDelta: 0, bushelsToFeed: 0, acresToPlant: UInt32.max)
        assertError(validateOrder(state: state, order: hugePlant), .insufficientAcresToPlant)

        // Harvest overflow guard (acres_to_plant * 5 + bushels > UInt32.max)
        var bigState = state
        bigState.population = 500_000_000
        bigState.bushels = 2_000_000_000
        bigState.acres = 1_000_000_000
        let overflowHarvestOrder = TurnOrder(acresDelta: 0, bushelsToFeed: 0, acresToPlant: 900_000_000)
        assertError(validateOrder(state: bigState, order: overflowHarvestOrder), .invalidOrderBounds)
    }

    func testAdversarialStateInvariantRefusal() {
        let validOrder = TurnOrder(acresDelta: 0, bushelsToFeed: 2000, acresToPlant: 1000)

        // Even rng_inc
        var evenIncState = newGame(seed: 1)
        evenIncState.rngInc = 0
        assertError(validateOrder(state: evenIncState, order: validOrder), .invalidOrderBounds)

        evenIncState.rngInc = 2
        assertError(validateOrder(state: evenIncState, order: validOrder), .invalidOrderBounds)

        // Hostile year = 0
        var badYearState = newGame(seed: 1)
        badYearState.year = 0
        assertError(validateOrder(state: badYearState, order: validOrder), .invalidOrderBounds)

        // Hostile year = 11
        badYearState.year = 11
        assertError(validateOrder(state: badYearState, order: validOrder), .invalidOrderBounds)
    }

    func testImpeachmentAndReport() {
        let state = newGame(seed: 100)
        let starveOrder = TurnOrder(acresDelta: 0, bushelsToFeed: 0, acresToPlant: 0)
        guard case .success(let nextState) = stepYear(state: state, order: starveOrder) else {
            XCTFail("stepYear failed unexpectedly")
            return
        }

        guard case .impeached(let starved, let percent) = nextState.status else {
            XCTFail("Expected .impeached status")
            return
        }
        XCTAssertEqual(starved, 100)
        XCTAssertEqual(percent, 100)

        let report = formatReport(state: nextState)
        XCTAssertTrue(report.contains("YOU STARVED 100 PEOPLE IN ONE YEAR!!!"))
        XCTAssertTrue(report.contains("IMPEACHED AND THROWN OUT OF OFFICE"))
    }

    func testWonRatingsEvaluation() {
        var state = newGame(seed: 1)
        state.year = 11
        state.percentStarvedSum = 20
        state.acres = 1000
        state.population = 100
        state.status = .won(rating: .charlemagne)

        let report = formatReport(state: state)
        XCTAssertTrue(report.contains("CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!"))
    }
}
