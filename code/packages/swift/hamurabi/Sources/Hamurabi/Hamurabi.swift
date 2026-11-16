// ============================================================================
// Hamurabi.swift — Deterministic Simulation Core for Swift
// ============================================================================
//
// Pure, deterministic simulation core for the seminal 1968 mainframe resource
// allocation game Hamurabi (GAME01 / HA01).
//
// In accordance with repository verification architecture:
// - Zero ambient system authority: capabilities: [] in required_capabilities.json
// - Pure state transitions: stepYear(state:order:) -> Result<HamurabiState, HamurabiError>
// - Explicit pseudorandomness: PCG32 state stored directly inside HamurabiState
// - CWE-190 mitigation: Swift traps on integer overflow by default. All arithmetic
//   uses either wrapping operators (&*, &+), reported overflow methods, or safe
//   widening (UInt32 -> UInt64, Int32.min negation guard via Int64).
// - Sendable and value semantics: All state types are immutable Sendable structs.

import Foundation

// ── Domain Types ─────────────────────────────────────────────────────────────

/// Rating classification awarded to ruler upon completing a 10-year term.
public enum Rating: String, Equatable, Hashable, Sendable, Codable {
    /// Average starvation <= 3% AND acres per person >= 10.
    case charlemagne = "Charlemagne"
    /// Average starvation <= 10% AND acres per person >= 9.
    case acre = "Acre"
    /// Average starvation <= 33% AND acres per person >= 7.
    case nero = "Nero"
    /// Average starvation > 33% OR acres per person < 7.
    case disaster = "Disaster"
}

/// Simulation lifecycle status.
public enum GameStatus: Equatable, Hashable, Sendable, Codable, CustomStringConvertible {
    /// Active, playable year (years 1..=10).
    case playing
    /// Completed 10-year term successfully (transitioned to terminal year 11).
    case won(rating: Rating)
    /// Impeached and expelled from office due to extreme starvation (> 45%).
    case impeached(starvedThisTurn: UInt32, percent: UInt32)
    /// Entire population died of starvation or disaster.
    case gameOverExtinction

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        let s = try container.decode(String.self)
        if s == "Playing" {
            self = .playing
        } else if s == "GameOverExtinction" {
            self = .gameOverExtinction
        } else if s.hasPrefix("Won(") && s.hasSuffix(")") {
            let ratingStr = String(s.dropFirst(4).dropLast())
            guard let rating = Rating(rawValue: ratingStr) else {
                throw DecodingError.dataCorruptedError(in: container, debugDescription: "Unknown rating: \(ratingStr)")
            }
            self = .won(rating: rating)
        } else if s.hasPrefix("Impeached(") && s.hasSuffix(")") {
            let inner = String(s.dropFirst(10).dropLast())
            let parts = inner.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }
            var starved: UInt32?
            var percent: UInt32?
            for part in parts {
                if part.hasPrefix("starved=") {
                    starved = UInt32(part.dropFirst(8))
                } else if part.hasPrefix("percent=") {
                    percent = UInt32(part.dropFirst(8))
                }
            }
            guard let st = starved, let pc = percent else {
                throw DecodingError.dataCorruptedError(in: container, debugDescription: "Invalid Impeached format: \(s)")
            }
            self = .impeached(starvedThisTurn: st, percent: pc)
        } else {
            throw DecodingError.dataCorruptedError(in: container, debugDescription: "Unknown status: \(s)")
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .playing:
            try container.encode("Playing")
        case .won(let rating):
            try container.encode("Won(\(rating.rawValue))")
        case .impeached(let starved, let percent):
            try container.encode("Impeached(starved=\(starved), percent=\(percent))")
        case .gameOverExtinction:
            try container.encode("GameOverExtinction")
        }
    }

    public var description: String {
        switch self {
        case .playing:
            return "Playing"
        case .won(let rating):
            return "Won(\(rating.rawValue))"
        case .impeached(let starved, let percent):
            return "Impeached(starved=\(starved), percent=\(percent))"
        case .gameOverExtinction:
            return "GameOverExtinction"
        }
    }
}

/// Closed enumeration of order rejection errors.
public enum HamurabiError: String, Error, Equatable, Hashable, Sendable, Codable {
    /// Attempted to execute an order when status is not Playing.
    case gameAlreadyEnded = "GameAlreadyEnded"
    /// Arithmetic overflow or invalid bounds on input parameters.
    case invalidOrderBounds = "InvalidOrderBounds"
    /// Player ordered to buy land but lacks sufficient bushels.
    case insufficientBushelsForLand = "InsufficientBushelsForLand"
    /// Player ordered to sell more land than the city owns.
    case insufficientAcresToSell = "InsufficientAcresToSell"
    /// Player ordered to feed more bushels than available after land trade.
    case insufficientBushelsToFeed = "InsufficientBushelsToFeed"
    /// Player ordered to plant more acres than owned after land trade.
    case insufficientAcresToPlant = "InsufficientAcresToPlant"
    /// Player ordered to plant more acres than population can tend (1 person tends 10 acres).
    case insufficientPopulationToPlant = "InsufficientPopulationToPlant"
    /// Player lacks grain to seed fields (1 bushel seeds 2 acres).
    case insufficientGrainForSeed = "InsufficientGrainForSeed"

    public var message: String {
        switch self {
        case .gameAlreadyEnded:
            return "Game has already ended"
        case .invalidOrderBounds:
            return "Order parameters exceed valid numerical bounds or overflow"
        case .insufficientBushelsForLand:
            return "Insufficient bushels to buy specified acreage"
        case .insufficientAcresToSell:
            return "Cannot sell more acres than the city owns"
        case .insufficientBushelsToFeed:
            return "Insufficient bushels in store to feed population"
        case .insufficientAcresToPlant:
            return "Cannot plant more acres than owned after land trade"
        case .insufficientPopulationToPlant:
            return "Insufficient population to tend planted acreage"
        case .insufficientGrainForSeed:
            return "Insufficient grain to seed planted acreage"
        }
    }
}

/// Player orders for a single year.
public struct TurnOrder: Equatable, Hashable, Sendable, Codable {
    /// Acres of land to buy (positive) or sell (negative). 0 indicates no trade.
    public var acresDelta: Int32
    /// Bushels of grain allocated to feed the population (20 bushels per person).
    public var bushelsToFeed: UInt32
    /// Acres of owned land to sow with grain (1 bushel seeds 2 acres).
    public var acresToPlant: UInt32

    public init(acresDelta: Int32, bushelsToFeed: UInt32, acresToPlant: UInt32) {
        self.acresDelta = acresDelta
        self.bushelsToFeed = bushelsToFeed
        self.acresToPlant = acresToPlant
    }

    enum CodingKeys: String, CodingKey {
        case acresDelta = "acres_delta"
        case bushelsToFeed = "bushels_to_feed"
        case acresToPlant = "acres_to_plant"
    }
}

/// Immutable value record capturing the complete simulation state.
public struct HamurabiState: Equatable, Hashable, Sendable, Codable {
    /// Current year of reign: 1..=10 are playable; 11 is terminal summary state.
    public var year: UInt8
    /// Current population of the city-state (initial: 100).
    public var population: UInt32
    /// Bushels of grain in storage (initial: 2800).
    public var bushels: UInt32
    /// Acres of farmland owned by the city (initial: 1000).
    public var acres: UInt32
    /// Current market price of land in bushels per acre (17..=26).
    public var landPrice: UInt32
    /// Yield from the previous harvest in bushels per acre (1..=5).
    public var harvestPerAcre: UInt32
    /// Bushels consumed by rats during previous year (initial: 200).
    public var ratsEaten: UInt32
    /// Citizens starved during previous year (initial: 0).
    public var starvedLastYear: UInt32
    /// Cumulative citizens starved across the entire reign.
    public var starvedTotal: UInt32
    /// Sum of yearly starvation percentages (for 10-year average computation).
    public var percentStarvedSum: UInt32
    /// New citizens arriving during previous year (initial: 5).
    public var immigrantsLastYear: UInt32
    /// Whether a plague occurred in the previous year.
    public var plagueStruck: Bool
    /// Current game lifecycle status.
    public var status: GameStatus
    /// Internal 64-bit state of the PCG32 generator.
    public var rngState: UInt64
    /// Internal 64-bit sequence increment of PCG32 (always odd).
    public var rngInc: UInt64

    public init(
        year: UInt8,
        population: UInt32,
        bushels: UInt32,
        acres: UInt32,
        landPrice: UInt32,
        harvestPerAcre: UInt32,
        ratsEaten: UInt32,
        starvedLastYear: UInt32,
        starvedTotal: UInt32,
        percentStarvedSum: UInt32,
        immigrantsLastYear: UInt32,
        plagueStruck: Bool,
        status: GameStatus,
        rngState: UInt64,
        rngInc: UInt64
    ) {
        self.year = year
        self.population = population
        self.bushels = bushels
        self.acres = acres
        self.landPrice = landPrice
        self.harvestPerAcre = harvestPerAcre
        self.ratsEaten = ratsEaten
        self.starvedLastYear = starvedLastYear
        self.starvedTotal = starvedTotal
        self.percentStarvedSum = percentStarvedSum
        self.immigrantsLastYear = immigrantsLastYear
        self.plagueStruck = plagueStruck
        self.status = status
        self.rngState = rngState
        self.rngInc = rngInc
    }

    enum CodingKeys: String, CodingKey {
        case year
        case population
        case bushels
        case acres
        case landPrice = "land_price"
        case harvestPerAcre = "harvest_per_acre"
        case ratsEaten = "rats_eaten"
        case starvedLastYear = "starved_last_year"
        case starvedTotal = "starved_total"
        case percentStarvedSum = "percent_starved_sum"
        case immigrantsLastYear = "immigrants_last_year"
        case plagueStruck = "plague_struck"
        case status
        case rngState = "rng_state"
        case rngInc = "rng_inc"
    }
}

// ── PRNG Internal Primitives (PCG32 XSH RR 64/32) ──────────────────────────

private let pcgMultiplier: UInt64 = 6364136223846793005
private let streamId: UInt64 = 54

@inline(__always)
private func pcg32Step(state: inout UInt64, inc: UInt64) -> UInt32 {
    let oldState = state
    state = oldState &* pcgMultiplier &+ inc
    let xorshifted = UInt32(truncatingIfNeeded: ((oldState >> 18) ^ oldState) >> 27)
    let rot = UInt32(truncatingIfNeeded: oldState >> 59)
    if rot == 0 {
        return xorshifted
    }
    return (xorshifted >> rot) | (xorshifted << (32 - rot))
}

private func pcg32NextIntInRange(state: inout UInt64, inc: UInt64, min: UInt32, max: UInt32) -> UInt32 {
    let span = UInt64(max - min) + 1
    let threshold = ((UInt64(1) << 32) - span) % span
    while true {
        let r = UInt64(pcg32Step(state: &state, inc: inc))
        if r >= threshold {
            return min + UInt32(r % span)
        }
    }
}

// ── Simulation Engine API ───────────────────────────────────────────────────

/// Initializes a new, deterministic 10-year reign for ruler Hamurabi.
public func newGame(seed: UInt64) -> HamurabiState {
    let inc: UInt64 = (streamId << 1) | 1 // 109
    var rngState: UInt64 = 0
    rngState = rngState &* pcgMultiplier &+ inc
    rngState = rngState &+ seed
    rngState = rngState &* pcgMultiplier &+ inc

    let landPrice = pcg32NextIntInRange(state: &rngState, inc: inc, min: 17, max: 26)

    return HamurabiState(
        year: 1,
        population: 100,
        bushels: 2800,
        acres: 1000,
        landPrice: landPrice,
        harvestPerAcre: 3,
        ratsEaten: 200,
        starvedLastYear: 0,
        starvedTotal: 0,
        percentStarvedSum: 0,
        immigrantsLastYear: 5,
        plagueStruck: false,
        status: .playing,
        rngState: rngState,
        rngInc: inc
    )
}

/// Validates player orders against current state constraints without advancing state.
///
/// Evaluates checks in canonical order:
/// 1. Status check (.playing)
/// 2. Bounds and overflow checks
/// 3. Land trade resource availability
/// 4. Food allocation feasibility
/// 5. Planting acreage and labor limits
/// 6. Seed grain availability
public func validateOrder(state: HamurabiState, order: TurnOrder) -> Result<Void, HamurabiError> {
    // 1. Status Check
    guard state.status == .playing else {
        return .failure(.gameAlreadyEnded)
    }

    // State invariant checks (VER04m state_inv & CWE-400 / CWE-835)
    guard (state.rngInc & 1) == 1 else {
        return .failure(.invalidOrderBounds)
    }
    guard state.year >= 1 && state.year <= 10 else {
        return .failure(.invalidOrderBounds)
    }
    guard state.landPrice >= 17 && state.landPrice <= 26 else {
        return .failure(.invalidOrderBounds)
    }
    guard state.harvestPerAcre >= 1 && state.harvestPerAcre <= 5 else {
        return .failure(.invalidOrderBounds)
    }

    // 2. Bounds & Overflow Guard on Land Trade
    if order.acresDelta > 0 {
        let buyCount = UInt64(order.acresDelta)
        let (cost, overflow) = buyCount.multipliedReportingOverflow(by: UInt64(state.landPrice))
        if overflow {
            return .failure(.invalidOrderBounds)
        }
        if cost > UInt64(state.bushels) {
            return .failure(.insufficientBushelsForLand)
        }
    } else if order.acresDelta < 0 {
        // Negate via Int64 to avoid trap on Int32.min (CWE-190)
        let sellCount = UInt64(-Int64(order.acresDelta))
        if sellCount > UInt64(state.acres) {
            return .failure(.insufficientAcresToSell)
        }
    }

    // 3. Land Trade Balances
    let acresAfterTrade: UInt32
    if order.acresDelta >= 0 {
        let (acresSum, overflow) = UInt64(state.acres).addingReportingOverflow(UInt64(order.acresDelta))
        if overflow || acresSum > UInt64(UInt32.max) {
            return .failure(.invalidOrderBounds)
        }
        acresAfterTrade = UInt32(acresSum)
    } else {
        let sellCount = UInt64(-Int64(order.acresDelta))
        acresAfterTrade = state.acres - UInt32(sellCount)
    }

    let bushelsAfterTrade: UInt32
    if order.acresDelta > 0 {
        let cost = UInt64(order.acresDelta) * UInt64(state.landPrice)
        bushelsAfterTrade = state.bushels - UInt32(cost)
    } else if order.acresDelta < 0 {
        let sellCount = UInt64(-Int64(order.acresDelta))
        let (revenue, overflow) = sellCount.multipliedReportingOverflow(by: UInt64(state.landPrice))
        if overflow {
            return .failure(.invalidOrderBounds)
        }
        let (bushelsSum, bOverflow) = UInt64(state.bushels).addingReportingOverflow(revenue)
        if bOverflow || bushelsSum > UInt64(UInt32.max) {
            return .failure(.invalidOrderBounds)
        }
        bushelsAfterTrade = UInt32(bushelsSum)
    } else {
        bushelsAfterTrade = state.bushels
    }

    // 4. Food Allocation Check
    if order.bushelsToFeed > bushelsAfterTrade {
        return .failure(.insufficientBushelsToFeed)
    }
    let bushelsAfterFeed = bushelsAfterTrade - order.bushelsToFeed

    // 5. Planting Land & Labor Checks
    if order.acresToPlant > acresAfterTrade {
        return .failure(.insufficientAcresToPlant)
    }
    let (maxLabor, laborOverflow) = UInt64(state.population).multipliedReportingOverflow(by: 10)
    if laborOverflow {
        return .failure(.invalidOrderBounds)
    }
    if UInt64(order.acresToPlant) > maxLabor {
        return .failure(.insufficientPopulationToPlant)
    }

    // 6. Planting Seed Check: each bushel seeds 2 acres (ceil(acres / 2))
    let seedNeeded = (UInt64(order.acresToPlant) + 1) / 2
    if seedNeeded > UInt64(bushelsAfterFeed) {
        return .failure(.insufficientGrainForSeed)
    }
    let bushelsAfterSeed = bushelsAfterFeed - UInt32(seedNeeded)

    // 7. Maximum Harvest Overflow Guard (CWE-190)
    let (maxHarvest, harvestOverflow) = UInt64(order.acresToPlant).multipliedReportingOverflow(by: 5)
    if harvestOverflow {
        return .failure(.invalidOrderBounds)
    }
    let (maxStored, storeOverflow) = UInt64(bushelsAfterSeed).addingReportingOverflow(maxHarvest)
    if storeOverflow || maxStored > UInt64(UInt32.max) {
        return .failure(.invalidOrderBounds)
    }

    return .success(())
}

/// Advances the simulation by exactly one year according to player orders.
public func stepYear(state: HamurabiState, order: TurnOrder) -> Result<HamurabiState, HamurabiError> {
    switch validateOrder(state: state, order: order) {
    case .failure(let error):
        return .failure(error)
    case .success:
        break
    }

    var pcgState = state.rngState
    let pcgInc = state.rngInc

    let acresAfterTrade: UInt32
    let bushelsAfterTrade: UInt32
    if order.acresDelta > 0 {
        acresAfterTrade = state.acres + UInt32(order.acresDelta)
        let cost = UInt64(order.acresDelta) * UInt64(state.landPrice)
        bushelsAfterTrade = state.bushels - UInt32(cost)
    } else if order.acresDelta < 0 {
        let sellCount = UInt64(-Int64(order.acresDelta))
        acresAfterTrade = state.acres - UInt32(sellCount)
        let revenue = sellCount * UInt64(state.landPrice)
        bushelsAfterTrade = state.bushels + UInt32(revenue)
    } else {
        acresAfterTrade = state.acres
        bushelsAfterTrade = state.bushels
    }

    let bushelsAfterFeed = bushelsAfterTrade - order.bushelsToFeed
    let seedNeeded = (UInt64(order.acresToPlant) + 1) / 2
    let bushelsAfterSeed = bushelsAfterFeed - UInt32(seedNeeded)

    // Step 1: Harvest Yield Draw [1, 5]
    let harvestPerAcre = pcg32NextIntInRange(state: &pcgState, inc: pcgInc, min: 1, max: 5)
    let totalHarvest = UInt64(order.acresToPlant) * UInt64(harvestPerAcre)
    let bushelsStored = UInt64(bushelsAfterSeed) + totalHarvest

    // Step 2: Rat Infestation Draws [1, 100]
    let ratRoll = pcg32NextIntInRange(state: &pcgState, inc: pcgInc, min: 1, max: 100)
    let ratsEaten: UInt32
    if ratRoll <= 40 {
        let ratFactor = pcg32NextIntInRange(state: &pcgState, inc: pcgInc, min: 1, max: 5)
        let eaten = bushelsStored / UInt64(ratFactor)
        ratsEaten = UInt32(min(eaten, UInt64(UInt32.max)))
    } else {
        ratsEaten = 0
    }

    let bushelsRemaining = bushelsStored > UInt64(ratsEaten) ? (bushelsStored - UInt64(ratsEaten)) : 0
    let bushelsFinal = UInt32(min(bushelsRemaining, UInt64(UInt32.max)))

    // Step 3: Plague Draw [1, 100]
    let plagueRoll = pcg32NextIntInRange(state: &pcgState, inc: pcgInc, min: 1, max: 100)
    let plagueStruck = plagueRoll <= 15

    // Step 4: Starvation & Impeachment Evaluation
    let peopleFed = order.bushelsToFeed / 20
    let starved: UInt32
    var survivors: UInt32
    if peopleFed >= state.population {
        starved = 0
        survivors = state.population
    } else {
        starved = state.population - peopleFed
        survivors = peopleFed
    }

    let percentStarved: UInt32
    if state.population > 0 {
        let pct = (UInt64(starved) * 100) / UInt64(state.population)
        percentStarved = UInt32(min(pct, 100))
    } else {
        percentStarved = 0
    }

    let newStarvedTotal = UInt32(min(UInt64(state.starvedTotal) + UInt64(starved), UInt64(UInt32.max)))
    let newPercentSum = UInt32(min(UInt64(state.percentStarvedSum) + UInt64(percentStarved), UInt64(UInt32.max)))

    var newStatus: GameStatus
    if UInt64(starved) * 100 > UInt64(state.population) * 45 {
        newStatus = .impeached(starvedThisTurn: starved, percent: percentStarved)
    } else if survivors == 0 {
        newStatus = .gameOverExtinction
    } else {
        newStatus = .playing
    }

    // Step 5: Plague Resolution
    if plagueStruck && newStatus == .playing {
        survivors /= 2
    }

    // Step 6: Immigration Draw [1, 5]
    let immFactor = pcg32NextIntInRange(state: &pcgState, inc: pcgInc, min: 1, max: 5)
    let immigrants: UInt32
    if starved == 0 && newStatus == .playing {
        let acresTerm = UInt64(acresAfterTrade) * 20
        let inner = acresTerm + UInt64(bushelsFinal)
        let numerator = UInt64(immFactor) * inner
        let denominator = 100 * UInt64(state.population)
        if denominator == 0 {
            immigrants = 0
        } else {
            let q = numerator / denominator
            immigrants = UInt32(min(q + 1, UInt64(UInt32.max)))
        }
    } else {
        immigrants = 0
    }

    let finalPopulation = UInt32(min(UInt64(survivors) + UInt64(immigrants), UInt64(UInt32.max)))

    // Step 7: Next Year Land Price Draw [17, 26]
    let nextLandPrice = pcg32NextIntInRange(state: &pcgState, inc: pcgInc, min: 17, max: 26)

    // Step 8: Turn Advance & Rating
    let nextYear = UInt8(min(UInt64(state.year) + 1, 255))
    if nextYear == 11 && newStatus == .playing {
        let avgStarved = newPercentSum / 10
        let acresPerPerson = finalPopulation == 0 ? 0 : (acresAfterTrade / finalPopulation)
        let rating: Rating
        if avgStarved <= 3 && acresPerPerson >= 10 {
            rating = .charlemagne
        } else if avgStarved <= 10 && acresPerPerson >= 9 {
            rating = .acre
        } else if avgStarved <= 33 && acresPerPerson >= 7 {
            rating = .nero
        } else {
            rating = .disaster
        }
        newStatus = .won(rating: rating)
    }

    return .success(HamurabiState(
        year: nextYear,
        population: finalPopulation,
        bushels: bushelsFinal,
        acres: acresAfterTrade,
        landPrice: nextLandPrice,
        harvestPerAcre: harvestPerAcre,
        ratsEaten: ratsEaten,
        starvedLastYear: starved,
        starvedTotal: newStarvedTotal,
        percentStarvedSum: newPercentSum,
        immigrantsLastYear: immigrants,
        plagueStruck: plagueStruck,
        status: newStatus,
        rngState: pcgState,
        rngInc: pcgInc
    ))
}

/// Formats the annual teletype report or final evaluation in authentic 1970s teletype styling.
public func formatReport(state: HamurabiState) -> String {
    switch state.status {
    case .impeached(let starvedThisTurn, _):
        return "YOU STARVED \(starvedThisTurn) PEOPLE IN ONE YEAR!!!\n" +
               "DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n"
    case .gameOverExtinction:
        return "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
    case .won(let rating):
        let avgStarved = state.percentStarvedSum / 10
        let acresPerPerson = state.population == 0 ? 0 : (state.acres / state.population)
        let ratingMsg: String
        switch rating {
        case .charlemagne:
            ratingMsg = "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!"
        case .acre:
            ratingMsg = "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE."
        case .nero:
            ratingMsg = "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU."
        case .disaster:
            ratingMsg = "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!"
        }
        return "IN YOUR 10-YEAR TERM OF OFFICE, \(avgStarved) PERCENT OF THE\n" +
               "POPULATION STARVED PER YEAR ON AVERAGE.\n" +
               "A TOTAL OF \(state.starvedTotal) PEOPLE DIED!!\n" +
               "YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n" +
               "\(acresPerPerson) ACRES PER PERSON.\n\n" +
               "\(ratingMsg)\n"
    case .playing:
        var out = "HAMURABI: I BEG TO REPORT TO YOU,\n" +
                  "IN YEAR \(state.year), \(state.starvedLastYear) PEOPLE STARVED, \(state.immigrantsLastYear) CAME TO THE CITY.\n"
        if state.plagueStruck {
            out += "A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n"
        }
        out += "POPULATION IS NOW \(state.population).\n" +
               "THE CITY NOW OWNS \(state.acres) ACRES.\n" +
               "YOU HARVESTED \(state.harvestPerAcre) BUSHELS PER ACRE.\n" +
               "RATS ATE \(state.ratsEaten) BUSHELS.\n" +
               "YOU NOW HAVE \(state.bushels) BUSHELS IN STORE.\n" +
               "LAND IS TRADING AT \(state.landPrice) BUSHELS PER ACRE.\n"
        return out
    }
}
