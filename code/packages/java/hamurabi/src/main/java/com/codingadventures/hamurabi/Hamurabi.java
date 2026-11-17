package com.codingadventures.hamurabi;

/**
 * Pure, deterministic simulation core for the seminal 1968 mainframe resource allocation game.
 */
public final class Hamurabi {

    private Hamurabi() {}

    /** LCG multiplier constant for 64-bit PCG32 (Knuth / Numerical Recipes). */
    private static final long PCG_MULTIPLIER = 6364136223846793005L;

    /** Stream ID for PCG32 PRNG stream in Hamurabi simulation. */
    private static final long HAMURABI_STREAM_ID = 54L;

    /** Initial PCG increment: (54 << 1) | 1 = 109. */
    private static final long INITIAL_INC = (HAMURABI_STREAM_ID << 1) | 1L;

    // ── PRNG Internal Primitives (PCG32 XSH RR 64/32) ──────────────────────────

    private static long pcg32Step(long[] rngState, long inc) {
        long oldState = rngState[0];
        rngState[0] = oldState * PCG_MULTIPLIER + inc;
        int xorshifted = (int) (((oldState >>> 18) ^ oldState) >>> 27);
        int rot = (int) (oldState >>> 59);
        int result = Integer.rotateRight(xorshifted, rot);
        return Integer.toUnsignedLong(result);
    }

    private static long pcg32NextIntInRange(long[] rngState, long inc, long min, long max) {
        long span = (max - min) + 1L;
        long two32 = 1L << 32;
        long threshold = Long.remainderUnsigned(two32 - span, span);
        while (true) {
            long r = pcg32Step(rngState, inc);
            if (Long.compareUnsigned(r, threshold) >= 0) {
                return min + Long.remainderUnsigned(r, span);
            }
        }
    }

    // ── Simulation Engine API ───────────────────────────────────────────────────

    /**
     * Initializes a new, deterministic 10-year reign for ruler Hamurabi.
     *
     * @param seed 64-bit unsigned seed for PCG32
     * @return Initial HamurabiState at Year 1
     */
    public static HamurabiState newGame(long seed) {
        long inc = INITIAL_INC;
        long[] rngState = new long[]{0L};
        // Standard PCG initseq warm-up
        rngState[0] = rngState[0] * PCG_MULTIPLIER + inc;
        rngState[0] = rngState[0] + seed;
        rngState[0] = rngState[0] * PCG_MULTIPLIER + inc;

        // Initial draw: land price for year 1 (range 17..=26)
        long landPrice = pcg32NextIntInRange(rngState, inc, 17, 26);

        return new HamurabiState(
            1,
            100L,
            2800L,
            1000L,
            landPrice,
            3L,
            200L,
            0L,
            0L,
            0L,
            5L,
            false,
            new GameStatus.Playing(),
            rngState[0],
            inc
        );
    }

    /**
     * Validates player orders against current state constraints without advancing state.
     *
     * @param state Current simulation state
     * @param order Proposed turn order
     * @return Result.ok(null) if valid, or Result.err(HamurabiError) on failure
     */
    public static Result<Void, HamurabiError> validateOrder(HamurabiState state, TurnOrder order) {
        // 1. Status Check
        if (!(state.status() instanceof GameStatus.Playing)) {
            return Result.err(HamurabiError.GAME_ALREADY_ENDED);
        }

        // State invariant checks (VER04m state_inv & CWE-400 / CWE-835)
        if ((state.rngInc() & 1L) != 1L) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
        }
        if (state.year() < 1 || state.year() > 10) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
        }
        if (state.landPrice() < 17 || state.landPrice() > 26) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
        }
        if (state.harvestPerAcre() < 1 || state.harvestPerAcre() > 5) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
        }
        if (state.population() < 0 || state.population() > 0xFFFF_FFFFL) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
        }
        if (state.bushels() < 0 || state.bushels() > 0xFFFF_FFFFL) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
        }
        if (state.acres() < 0 || state.acres() > 0xFFFF_FFFFL) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
        }

        // Order numerical bounds checks
        if (order.acresDelta() < Integer.MIN_VALUE || order.acresDelta() > Integer.MAX_VALUE) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
        }
        if (order.bushelsToFeed() < 0 || order.bushelsToFeed() > 0xFFFF_FFFFL) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
        }
        if (order.acresToPlant() < 0 || order.acresToPlant() > 0xFFFF_FFFFL) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
        }

        // 2. Bounds & Overflow Guard on Land Trade
        if (order.acresDelta() > 0) {
            long buyCount = order.acresDelta();
            long cost;
            try {
                cost = Math.multiplyExact(buyCount, state.landPrice());
            } catch (ArithmeticException e) {
                return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
            }
            if (cost > state.bushels()) {
                return Result.err(HamurabiError.INSUFFICIENT_BUSHELS_FOR_LAND);
            }
        } else if (order.acresDelta() < 0) {
            // Guard Integer.MIN_VALUE before negation (CWE-190)
            long sellCount = -((long) order.acresDelta());
            if (sellCount > state.acres()) {
                return Result.err(HamurabiError.INSUFFICIENT_ACRES_TO_SELL);
            }
        }

        // 3. Land Trade Balances
        long acresAfterTrade;
        if (order.acresDelta() >= 0) {
            acresAfterTrade = state.acres() + order.acresDelta();
            if (acresAfterTrade > 0xFFFF_FFFFL) {
                return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
            }
        } else {
            long sellCount = -((long) order.acresDelta());
            acresAfterTrade = state.acres() - sellCount;
        }

        long bushelsAfterTrade;
        if (order.acresDelta() > 0) {
            long cost = order.acresDelta() * state.landPrice();
            bushelsAfterTrade = state.bushels() - cost;
        } else if (order.acresDelta() < 0) {
            long sellCount = -((long) order.acresDelta());
            long revenue;
            try {
                revenue = Math.multiplyExact(sellCount, state.landPrice());
            } catch (ArithmeticException e) {
                return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
            }
            bushelsAfterTrade = state.bushels() + revenue;
            if (bushelsAfterTrade > 0xFFFF_FFFFL) {
                return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
            }
        } else {
            bushelsAfterTrade = state.bushels();
        }

        // 4. Food Allocation Check
        if (order.bushelsToFeed() > bushelsAfterTrade) {
            return Result.err(HamurabiError.INSUFFICIENT_BUSHELS_TO_FEED);
        }
        long bushelsAfterFeed = bushelsAfterTrade - order.bushelsToFeed();

        // 5. Planting Land & Labor Checks
        if (order.acresToPlant() > acresAfterTrade) {
            return Result.err(HamurabiError.INSUFFICIENT_ACRES_TO_PLANT);
        }
        long maxLaborAcres;
        try {
            maxLaborAcres = Math.multiplyExact(state.population(), 10L);
        } catch (ArithmeticException e) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
        }
        if (order.acresToPlant() > maxLaborAcres) {
            return Result.err(HamurabiError.INSUFFICIENT_POPULATION_TO_PLANT);
        }

        // 6. Planting Seed Check: each bushel seeds 2 acres (ceil(acres / 2))
        long seedNeeded = (order.acresToPlant() + 1L) / 2L;
        if (seedNeeded > bushelsAfterFeed) {
            return Result.err(HamurabiError.INSUFFICIENT_GRAIN_FOR_SEED);
        }
        long bushelsAfterSeed = bushelsAfterFeed - seedNeeded;

        // 7. Maximum Harvest Overflow Guard (CWE-190)
        long maxHarvest = order.acresToPlant() * 5L;
        long maxStored = bushelsAfterSeed + maxHarvest;
        if (maxStored > 0xFFFF_FFFFL) {
            return Result.err(HamurabiError.INVALID_ORDER_BOUNDS);
        }

        return Result.ok(null);
    }

    /**
     * Advances the simulation by exactly one year according to player orders.
     *
     * @param state Current simulation state
     * @param order Player orders
     * @return Result with new HamurabiState or HamurabiError
     */
    public static Result<HamurabiState, HamurabiError> stepYear(HamurabiState state, TurnOrder order) {
        Result<Void, HamurabiError> validation = validateOrder(state, order);
        if (validation.isErr()) {
            return Result.err(validation.unwrapErr());
        }

        long acresAfterTrade;
        if (order.acresDelta() >= 0) {
            acresAfterTrade = state.acres() + order.acresDelta();
        } else {
            long sellCount = -((long) order.acresDelta());
            acresAfterTrade = state.acres() - sellCount;
        }

        long bushelsAfterTrade;
        if (order.acresDelta() > 0) {
            long cost = order.acresDelta() * state.landPrice();
            bushelsAfterTrade = state.bushels() - cost;
        } else if (order.acresDelta() < 0) {
            long sellCount = -((long) order.acresDelta());
            long revenue = sellCount * state.landPrice();
            bushelsAfterTrade = state.bushels() + revenue;
        } else {
            bushelsAfterTrade = state.bushels();
        }

        long bushelsAfterFeed = bushelsAfterTrade - order.bushelsToFeed();
        long seedNeeded = (order.acresToPlant() + 1L) / 2L;
        long bushelsAfterSeed = bushelsAfterFeed - seedNeeded;

        long[] prngState = new long[]{state.rngState()};
        long prngInc = state.rngInc();

        // Step 1: Harvest Yield Draw
        // Draw 1: pcg.nextIntInRange(1, 5) -> harvestPerAcre
        long harvestPerAcre = pcg32NextIntInRange(prngState, prngInc, 1, 5);
        long totalHarvest = order.acresToPlant() * harvestPerAcre;
        long bushelsStored = bushelsAfterSeed + totalHarvest;

        // Step 2: Rat Infestation Draws
        // Draw 2: pcg.nextIntInRange(1, 100) -> ratRoll
        long ratRoll = pcg32NextIntInRange(prngState, prngInc, 1, 100);
        long ratsEaten = 0L;
        if (ratRoll <= 40) {
            // Draw 3: pcg.nextIntInRange(1, 5) -> ratFactor
            long ratFactor = pcg32NextIntInRange(prngState, prngInc, 1, 5);
            long eaten = bushelsStored / ratFactor;
            ratsEaten = Math.min(eaten, 0xFFFF_FFFFL);
        }
        long bushelsFinal = Math.min(Math.max(0L, bushelsStored - ratsEaten), 0xFFFF_FFFFL);

        // Step 3: Plague Draw
        // Draw 4: pcg.nextIntInRange(1, 100) -> plagueRoll
        long plagueRoll = pcg32NextIntInRange(prngState, prngInc, 1, 100);
        boolean plagueStruck = plagueRoll <= 15;

        // Step 4: Starvation & Impeachment Evaluation
        long peopleFed = order.bushelsToFeed() / 20;
        long starved;
        long survivors;
        if (peopleFed >= state.population()) {
            starved = 0L;
            survivors = state.population();
        } else {
            starved = state.population() - peopleFed;
            survivors = peopleFed;
        }

        long percentStarved = (state.population() > 0)
            ? Math.min(100L, (starved * 100L) / state.population())
            : 0L;
        long newStarvedTotal = Math.min(0xFFFF_FFFFL, state.starvedTotal() + starved);
        long newPercentSum = Math.min(0xFFFF_FFFFL, state.percentStarvedSum() + percentStarved);

        GameStatus newStatus;
        if (starved * 100L > state.population() * 45L) {
            newStatus = new GameStatus.Impeached(starved, percentStarved);
        } else if (survivors == 0) {
            newStatus = new GameStatus.GameOverExtinction();
        } else {
            newStatus = new GameStatus.Playing();
        }

        // Step 5: Plague Resolution
        if (plagueStruck && newStatus instanceof GameStatus.Playing) {
            survivors /= 2;
        }

        // Step 6: Immigration Draw
        // Draw 5: pcg.nextIntInRange(1, 5) -> immFactor
        long immFactor = pcg32NextIntInRange(prngState, prngInc, 1, 5);
        long immigrants = 0L;
        if (starved == 0 && newStatus instanceof GameStatus.Playing) {
            long acresTerm = Math.min(Long.MAX_VALUE, acresAfterTrade * 20L);
            long inner = Math.min(Long.MAX_VALUE, acresTerm + bushelsFinal);
            long numerator = immFactor * inner;
            long denominator = 100L * state.population();
            if (denominator > 0) {
                long q = numerator / denominator;
                immigrants = Math.min(0xFFFF_FFFFL, q + 1L);
            }
        }

        long finalPopulation = Math.min(0xFFFF_FFFFL, survivors + immigrants);

        // Step 7: Next Year Land Price Draw
        // Draw 6: pcg.nextIntInRange(17, 26) -> nextLandPrice
        long nextLandPrice = pcg32NextIntInRange(prngState, prngInc, 17, 26);

        // Step 8: Turn Advance & Rating
        int nextYear = Math.min(255, state.year() + 1);
        if (nextYear == 11 && newStatus instanceof GameStatus.Playing) {
            long avgStarved = newPercentSum / 10L;
            long acresPerPerson = (finalPopulation > 0) ? (acresAfterTrade / finalPopulation) : 0L;
            Rating rating;
            if (avgStarved <= 3 && acresPerPerson >= 10) {
                rating = Rating.CHARLEMAGNE;
            } else if (avgStarved <= 10 && acresPerPerson >= 9) {
                rating = Rating.ACRE;
            } else if (avgStarved <= 33 && acresPerPerson >= 7) {
                rating = Rating.NERO;
            } else {
                rating = Rating.DISASTER;
            }
            newStatus = new GameStatus.Won(rating);
        }

        return Result.ok(new HamurabiState(
            nextYear,
            finalPopulation,
            bushelsFinal,
            acresAfterTrade,
            nextLandPrice,
            harvestPerAcre,
            ratsEaten,
            starved,
            newStarvedTotal,
            newPercentSum,
            immigrants,
            plagueStruck,
            newStatus,
            prngState[0],
            prngInc
        ));
    }

    /**
     * Formats the annual teletype report or final evaluation in authentic 1970s teletype styling.
     *
     * @param state Current simulation state
     * @return Multi-line formatted teletype report string
     */
    public static String formatReport(HamurabiState state) {
        if (state.status() instanceof GameStatus.Impeached imp) {
            return "YOU STARVED " + imp.starvedThisTurn() + " PEOPLE IN ONE YEAR!!!\n" +
                   "DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n";
        }
        if (state.status() instanceof GameStatus.GameOverExtinction) {
            return "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n";
        }
        if (state.status() instanceof GameStatus.Won won) {
            long avgStarved = state.percentStarvedSum() / 10L;
            long acresPerPerson = (state.population() > 0) ? (state.acres() / state.population()) : 0L;
            String ratingMsg = switch (won.rating()) {
                case CHARLEMAGNE -> "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!";
                case ACRE -> "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE.";
                case NERO -> "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU.";
                case DISASTER -> "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!";
            };
            return "IN YOUR 10-YEAR TERM OF OFFICE, " + avgStarved + " PERCENT OF THE\n" +
                   "POPULATION STARVED PER YEAR ON AVERAGE.\n" +
                   "A TOTAL OF " + state.starvedTotal() + " PEOPLE DIED!!\n" +
                   "YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n" +
                   acresPerPerson + " ACRES PER PERSON.\n\n" +
                   ratingMsg + "\n";
        }
        // Playing
        StringBuilder out = new StringBuilder();
        out.append("HAMURABI: I BEG TO REPORT TO YOU,\n");
        out.append("IN YEAR ").append(state.year()).append(", ")
           .append(state.starvedLastYear()).append(" PEOPLE STARVED, ")
           .append(state.immigrantsLastYear()).append(" CAME TO THE CITY.\n");
        if (state.plagueStruck()) {
            out.append("A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n");
        }
        out.append("POPULATION IS NOW ").append(state.population()).append(".\n");
        out.append("THE CITY NOW OWNS ").append(state.acres()).append(" ACRES.\n");
        out.append("YOU HARVESTED ").append(state.harvestPerAcre()).append(" BUSHELS PER ACRE.\n");
        out.append("RATS ATE ").append(state.ratsEaten()).append(" BUSHELS.\n");
        out.append("YOU NOW HAVE ").append(state.bushels()).append(" BUSHELS IN STORE.\n");
        out.append("LAND IS TRADING AT ").append(state.landPrice()).append(" BUSHELS PER ACRE.\n");
        return out.toString();
    }
}
