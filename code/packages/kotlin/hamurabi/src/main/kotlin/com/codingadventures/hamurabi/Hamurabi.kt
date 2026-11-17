@file:JvmName("Hamurabi")
package com.codingadventures.hamurabi

/**
 * Historical performance rating awarded to the ruler upon completing a 10-year term.
 */
enum class Rating {
    CHARLEMAGNE,
    ACRE,
    NERO,
    DISASTER
}

/**
 * Simulation lifecycle status.
 */
sealed interface GameStatus {
    object Playing : GameStatus {
        override fun toString(): String = "Playing"
    }

    data class Won(val rating: Rating) : GameStatus {
        override fun toString(): String {
            val r = when (rating) {
                Rating.CHARLEMAGNE -> "Charlemagne"
                Rating.ACRE -> "Acre"
                Rating.NERO -> "Nero"
                Rating.DISASTER -> "Disaster"
            }
            return "Won($r)"
        }
    }

    data class Impeached(val starvedThisTurn: Long, val percent: Long) : GameStatus {
        override fun toString(): String = "Impeached(starved=$starvedThisTurn, percent=$percent)"
    }

    object GameOverExtinction : GameStatus {
        override fun toString(): String = "GameOverExtinction"
    }
}

/**
 * Closed enumeration of order rejection errors.
 */
enum class HamurabiError(val message: String) {
    GAME_ALREADY_ENDED("Game has already ended"),
    INVALID_ORDER_BOUNDS("Order parameters exceed valid numerical bounds or overflow"),
    INSUFFICIENT_BUSHELS_FOR_LAND("Insufficient bushels to buy specified acreage"),
    INSUFFICIENT_ACRES_TO_SELL("Cannot sell more acres than the city owns"),
    INSUFFICIENT_BUSHELS_TO_FEED("Insufficient bushels in store to feed population"),
    INSUFFICIENT_ACRES_TO_PLANT("Cannot plant more acres than owned after land trade"),
    INSUFFICIENT_POPULATION_TO_PLANT("Insufficient population to tend planted acreage"),
    INSUFFICIENT_GRAIN_FOR_SEED("Insufficient grain to seed planted acreage");

    override fun toString(): String = message
}

/**
 * Functional result type representing either success ([Ok]) or failure ([Err]).
 */
sealed interface Result<out T, out E> {
    data class Ok<out T>(val value: T) : Result<T, Nothing>
    data class Err<out E>(val error: E) : Result<Nothing, E>

    val isOk: Boolean get() = this is Ok
    val isErr: Boolean get() = this is Err

    fun unwrap(): T = when (this) {
        is Ok -> value
        is Err -> error("Called unwrap on Err: $error")
    }

    fun unwrapErr(): E = when (this) {
        is Ok -> error("Called unwrapErr on Ok: $value")
        is Err -> error
    }

    companion object {
        fun <T> ok(value: T): Result<T, Nothing> = Ok(value)
        fun <E> err(error: E): Result<Nothing, E> = Err(error)
    }
}

/**
 * Player orders for a single year.
 */
data class TurnOrder(
    val acresDelta: Long,
    val bushelsToFeed: Long,
    val acresToPlant: Long
)

/**
 * Immutable value record capturing the complete simulation state.
 */
data class HamurabiState(
    val year: Int,
    val population: Long,
    val bushels: Long,
    val acres: Long,
    val landPrice: Long,
    val harvestPerAcre: Long,
    val ratsEaten: Long,
    val starvedLastYear: Long,
    val starvedTotal: Long,
    val percentStarvedSum: Long,
    val immigrantsLastYear: Long,
    val plagueStruck: Boolean,
    val status: GameStatus,
    val rngState: Long,
    val rngInc: Long
)

// ── PRNG Internal Primitives (PCG32 XSH RR 64/32) ──────────────────────────

private const val PCG_MULTIPLIER: Long = 6364136223846793005L
private const val HAMURABI_STREAM_ID: Long = 54L
private const val INITIAL_INC: Long = (HAMURABI_STREAM_ID shl 1) or 1L // 109L

private class Pcg32State(var state: Long, val inc: Long) {
    fun step(): Long {
        val oldState = state
        state = oldState * PCG_MULTIPLIER + inc
        val xorshifted: Int = (((oldState ushr 18) xor oldState) ushr 27).toInt()
        val rot: Int = (oldState ushr 59).toInt()
        return xorshifted.rotateRight(rot).toLong() and 0xFFFFFFFFL
    }

    fun nextIntInRange(min: Long, max: Long): Long {
        val span = (max - min) + 1L
        val two32 = 1L shl 32
        val threshold = (two32 - span) % span
        while (true) {
            val r = step()
            if (r >= threshold) {
                return min + (r % span)
            }
        }
    }
}

// ── Simulation Engine API ───────────────────────────────────────────────────

/**
 * Initializes a new, deterministic 10-year reign for ruler Hamurabi.
 */
fun newGame(seed: Long): HamurabiState {
    val inc = INITIAL_INC
    var state = 0L
    state = state * PCG_MULTIPLIER + inc
    state += seed
    state = state * PCG_MULTIPLIER + inc

    val prng = Pcg32State(state, inc)
    val landPrice = prng.nextIntInRange(17L, 26L)

    return HamurabiState(
        year = 1,
        population = 100L,
        bushels = 2800L,
        acres = 1000L,
        landPrice = landPrice,
        harvestPerAcre = 3L,
        ratsEaten = 200L,
        starvedLastYear = 0L,
        starvedTotal = 0L,
        percentStarvedSum = 0L,
        immigrantsLastYear = 5L,
        plagueStruck = false,
        status = GameStatus.Playing,
        rngState = prng.state,
        rngInc = inc
    )
}

/**
 * Validates player orders against current state constraints without advancing state.
 */
fun validateOrder(state: HamurabiState, order: TurnOrder): Result<Unit, HamurabiError> {
    // 1. Status Check
    if (state.status !is GameStatus.Playing) {
        return Result.err(HamurabiError.GAME_ALREADY_ENDED)
    }

    // State invariant checks (VER04m state_inv & CWE-400 / CWE-835)
    if ((state.rngInc and 1L) != 1L) {
        return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
    }
    if (state.year !in 1..10) {
        return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
    }
    if (state.landPrice !in 17L..26L) {
        return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
    }
    if (state.harvestPerAcre !in 1L..5L) {
        return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
    }
    if (state.population !in 0L..0xFFFFFFFFL) {
        return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
    }
    if (state.bushels !in 0L..0xFFFFFFFFL) {
        return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
    }
    if (state.acres !in 0L..0xFFFFFFFFL) {
        return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
    }

    // Order numerical bounds checks
    if (order.acresDelta < Int.MIN_VALUE || order.acresDelta > Int.MAX_VALUE) {
        return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
    }
    if (order.bushelsToFeed !in 0L..0xFFFFFFFFL) {
        return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
    }
    if (order.acresToPlant !in 0L..0xFFFFFFFFL) {
        return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
    }

    // 2. Bounds & Overflow Guard on Land Trade
    if (order.acresDelta > 0) {
        val buyCount = order.acresDelta
        val cost = try {
            Math.multiplyExact(buyCount, state.landPrice)
        } catch (_: ArithmeticException) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
        }
        if (cost > state.bushels) {
            return Result.err(HamurabiError.INSUFFICIENT_BUSHELS_FOR_LAND)
        }
    } else if (order.acresDelta < 0) {
        // Guard Int.MIN_VALUE before negation (CWE-190)
        val sellCount = -order.acresDelta
        if (sellCount > state.acres) {
            return Result.err(HamurabiError.INSUFFICIENT_ACRES_TO_SELL)
        }
    }

    // 3. Land Trade Balances
    val acresAfterTrade = if (order.acresDelta >= 0) {
        val a = state.acres + order.acresDelta
        if (a > 0xFFFFFFFFL) return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
        a
    } else {
        val sellCount = -order.acresDelta
        state.acres - sellCount
    }

    val bushelsAfterTrade = if (order.acresDelta > 0) {
        val cost = order.acresDelta * state.landPrice
        state.bushels - cost
    } else if (order.acresDelta < 0) {
        val sellCount = -order.acresDelta
        val revenue = try {
            Math.multiplyExact(sellCount, state.landPrice)
        } catch (_: ArithmeticException) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
        }
        val b = state.bushels + revenue
        if (b > 0xFFFFFFFFL) return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
        b
    } else {
        state.bushels
    }

    // 4. Food Allocation Check
    if (order.bushelsToFeed > bushelsAfterTrade) {
        return Result.err(HamurabiError.INSUFFICIENT_BUSHELS_TO_FEED)
    }
    val bushelsAfterFeed = bushelsAfterTrade - order.bushelsToFeed

    // 5. Planting Land & Labor Checks
    if (order.acresToPlant > acresAfterTrade) {
        return Result.err(HamurabiError.INSUFFICIENT_ACRES_TO_PLANT)
    }
    val maxLaborAcres = try {
        Math.multiplyExact(state.population, 10L)
    } catch (_: ArithmeticException) {
        return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
    }
    if (order.acresToPlant > maxLaborAcres) {
        return Result.err(HamurabiError.INSUFFICIENT_POPULATION_TO_PLANT)
    }

    // 6. Planting Seed Check: each bushel seeds 2 acres (ceil(acres / 2))
    val seedNeeded = (order.acresToPlant + 1L) / 2L
    if (seedNeeded > bushelsAfterFeed) {
        return Result.err(HamurabiError.INSUFFICIENT_GRAIN_FOR_SEED)
    }
    val bushelsAfterSeed = bushelsAfterFeed - seedNeeded

    // 7. Maximum Harvest Overflow Guard (CWE-190)
    val maxHarvest = order.acresToPlant * 5L
    val maxStored = bushelsAfterSeed + maxHarvest
    if (maxStored > 0xFFFFFFFFL) {
        return Result.err(HamurabiError.INVALID_ORDER_BOUNDS)
    }

    return Result.ok(Unit)
}

/**
 * Advances the simulation by exactly one year according to player orders.
 */
fun stepYear(state: HamurabiState, order: TurnOrder): Result<HamurabiState, HamurabiError> {
    val validation = validateOrder(state, order)
    if (validation.isErr) {
        return Result.err(validation.unwrapErr())
    }

    val acresAfterTrade = if (order.acresDelta >= 0) {
        state.acres + order.acresDelta
    } else {
        state.acres - (-order.acresDelta)
    }

    val bushelsAfterTrade = if (order.acresDelta > 0) {
        state.bushels - order.acresDelta * state.landPrice
    } else if (order.acresDelta < 0) {
        state.bushels + (-order.acresDelta) * state.landPrice
    } else {
        state.bushels
    }

    val bushelsAfterFeed = bushelsAfterTrade - order.bushelsToFeed
    val seedNeeded = (order.acresToPlant + 1L) / 2L
    val bushelsAfterSeed = bushelsAfterFeed - seedNeeded

    val prng = Pcg32State(state.rngState, state.rngInc)

    // Step 1: Harvest Yield Draw
    val harvestPerAcre = prng.nextIntInRange(1L, 5L)
    val totalHarvest = order.acresToPlant * harvestPerAcre
    val bushelsStored = bushelsAfterSeed + totalHarvest

    // Step 2: Rat Infestation Draws
    val ratRoll = prng.nextIntInRange(1L, 100L)
    val ratsEaten = if (ratRoll <= 40L) {
        val ratFactor = prng.nextIntInRange(1L, 5L)
        val eaten = bushelsStored / ratFactor
        minOf(eaten, 0xFFFFFFFFL)
    } else {
        0L
    }
    val bushelsFinal = minOf(maxOf(0L, bushelsStored - ratsEaten), 0xFFFFFFFFL)

    // Step 3: Plague Draw
    val plagueRoll = prng.nextIntInRange(1L, 100L)
    val plagueStruck = plagueRoll <= 15L

    // Step 4: Starvation & Impeachment Evaluation
    val peopleFed = order.bushelsToFeed / 20L
    val starved: Long
    var survivors: Long
    if (peopleFed >= state.population) {
        starved = 0L
        survivors = state.population
    } else {
        starved = state.population - peopleFed
        survivors = peopleFed
    }

    val percentStarved = if (state.population > 0L) {
        minOf(100L, (starved * 100L) / state.population)
    } else {
        0L
    }
    val newStarvedTotal = minOf(0xFFFFFFFFL, state.starvedTotal + starved)
    val newPercentSum = minOf(0xFFFFFFFFL, state.percentStarvedSum + percentStarved)

    var newStatus: GameStatus = if (starved * 100L > state.population * 45L) {
        GameStatus.Impeached(starved, percentStarved)
    } else if (survivors == 0L) {
        GameStatus.GameOverExtinction
    } else {
        GameStatus.Playing
    }

    // Step 5: Plague Resolution
    if (plagueStruck && newStatus is GameStatus.Playing) {
        survivors /= 2L
    }

    // Step 6: Immigration Draw
    val immFactor = prng.nextIntInRange(1L, 5L)
    val immigrants = if (starved == 0L && newStatus is GameStatus.Playing) {
        val acresTerm = minOf(Long.MAX_VALUE, acresAfterTrade * 20L)
        val inner = minOf(Long.MAX_VALUE, acresTerm + bushelsFinal)
        val numerator = immFactor * inner
        val denominator = 100L * state.population
        if (denominator > 0L) {
            val q = numerator / denominator
            minOf(0xFFFFFFFFL, q + 1L)
        } else {
            0L
        }
    } else {
        0L
    }

    val finalPopulation = minOf(0xFFFFFFFFL, survivors + immigrants)

    // Step 7: Next Year Land Price Draw
    val nextLandPrice = prng.nextIntInRange(17L, 26L)

    // Step 8: Turn Advance & Rating
    val nextYear = minOf(255, state.year + 1)
    if (nextYear == 11 && newStatus is GameStatus.Playing) {
        val avgStarved = newPercentSum / 10L
        val acresPerPerson = if (finalPopulation > 0L) (acresAfterTrade / finalPopulation) else 0L
        val rating = if (avgStarved <= 3L && acresPerPerson >= 10L) {
            Rating.CHARLEMAGNE
        } else if (avgStarved <= 10L && acresPerPerson >= 9L) {
            Rating.ACRE
        } else if (avgStarved <= 33L && acresPerPerson >= 7L) {
            Rating.NERO
        } else {
            Rating.DISASTER
        }
        newStatus = GameStatus.Won(rating)
    }

    return Result.ok(
        HamurabiState(
            year = nextYear,
            population = finalPopulation,
            bushels = bushelsFinal,
            acres = acresAfterTrade,
            landPrice = nextLandPrice,
            harvestPerAcre = harvestPerAcre,
            ratsEaten = ratsEaten,
            starvedLastYear = starved,
            starvedTotal = newStarvedTotal,
            percentStarvedSum = newPercentSum,
            immigrantsLastYear = immigrants,
            plagueStruck = plagueStruck,
            status = newStatus,
            rngState = prng.state,
            rngInc = prng.inc
        )
    )
}

/**
 * Formats the annual teletype report or final evaluation in authentic 1970s teletype styling.
 */
fun formatReport(state: HamurabiState): String {
    return when (val s = state.status) {
        is GameStatus.Impeached -> {
            "YOU STARVED ${s.starvedThisTurn} PEOPLE IN ONE YEAR!!!\n" +
            "DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n"
        }
        is GameStatus.GameOverExtinction -> {
            "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
        }
        is GameStatus.Won -> {
            val avgStarved = state.percentStarvedSum / 10L
            val acresPerPerson = if (state.population > 0L) (state.acres / state.population) else 0L
            val ratingMsg = when (s.rating) {
                Rating.CHARLEMAGNE -> "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!"
                Rating.ACRE -> "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE."
                Rating.NERO -> "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU."
                Rating.DISASTER -> "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!"
            }
            "IN YOUR 10-YEAR TERM OF OFFICE, $avgStarved PERCENT OF THE\n" +
            "POPULATION STARVED PER YEAR ON AVERAGE.\n" +
            "A TOTAL OF ${state.starvedTotal} PEOPLE DIED!!\n" +
            "YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n" +
            "$acresPerPerson ACRES PER PERSON.\n\n" +
            "$ratingMsg\n"
        }
        is GameStatus.Playing -> {
            val sb = StringBuilder()
            sb.append("HAMURABI: I BEG TO REPORT TO YOU,\n")
            sb.append("IN YEAR ${state.year}, ${state.starvedLastYear} PEOPLE STARVED, ${state.immigrantsLastYear} CAME TO THE CITY.\n")
            if (state.plagueStruck) {
                sb.append("A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n")
            }
            sb.append("POPULATION IS NOW ${state.population}.\n")
            sb.append("THE CITY NOW OWNS ${state.acres} ACRES.\n")
            sb.append("YOU HARVESTED ${state.harvestPerAcre} BUSHELS PER ACRE.\n")
            sb.append("RATS ATE ${state.ratsEaten} BUSHELS.\n")
            sb.append("YOU NOW HAVE ${state.bushels} BUSHELS IN STORE.\n")
            sb.append("LAND IS TRADING AT ${state.landPrice} BUSHELS PER ACRE.\n")
            sb.toString()
        }
    }
}
