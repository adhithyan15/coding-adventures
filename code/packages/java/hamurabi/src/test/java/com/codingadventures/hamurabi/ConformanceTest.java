package com.codingadventures.hamurabi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Locale;
import org.junit.jupiter.api.Test;

/**
 * Conformance test running the shared Hamurabi corpus (code/specs/fixtures/hamurabi-v1/cases.txt).
 */
class ConformanceTest {

    private static Path findCorpusPath() {
        Path[] candidates = new Path[]{
            Path.of("../../../specs/fixtures/hamurabi-v1/cases.txt"),
            Path.of("code/specs/fixtures/hamurabi-v1/cases.txt"),
            Path.of("../../../../code/specs/fixtures/hamurabi-v1/cases.txt"),
            Path.of("../../code/specs/fixtures/hamurabi-v1/cases.txt")
        };
        for (Path p : candidates) {
            if (Files.exists(p)) {
                return p.toAbsolutePath().normalize();
            }
        }
        throw new IllegalStateException("Could not find hamurabi-v1/cases.txt in candidate locations");
    }

    private static GameStatus parseStatus(String statusStr) {
        if ("Playing".equals(statusStr)) {
            return new GameStatus.Playing();
        }
        if ("GameOverExtinction".equals(statusStr)) {
            return new GameStatus.GameOverExtinction();
        }
        if (statusStr.startsWith("Won(")) {
            String ratingStr = statusStr.substring(4, statusStr.length() - 1);
            Rating rating = Rating.valueOf(ratingStr.toUpperCase(Locale.ROOT));
            return new GameStatus.Won(rating);
        }
        if (statusStr.startsWith("Impeached(")) {
            // format: Impeached(starved=50, percent=50)
            String inner = statusStr.substring(10, statusStr.length() - 1);
            String[] parts = inner.split(", ");
            long starved = Long.parseLong(parts[0].split("=")[1]);
            long percent = Long.parseLong(parts[1].split("=")[1]);
            return new GameStatus.Impeached(starved, percent);
        }
        throw new IllegalArgumentException("Unknown status string: " + statusStr);
    }

    private static HamurabiState parseStateLine(String line) {
        String[] tokens = line.split(" ");
        int year = Integer.parseInt(tokens[1]);
        long population = Long.parseLong(tokens[2]);
        long bushels = Long.parseLong(tokens[3]);
        long acres = Long.parseLong(tokens[4]);
        long landPrice = Long.parseLong(tokens[5]);
        long harvestPerAcre = Long.parseLong(tokens[6]);
        long ratsEaten = Long.parseLong(tokens[7]);
        long starvedLastYear = Long.parseLong(tokens[8]);
        long starvedTotal = Long.parseLong(tokens[9]);
        long percentStarvedSum = Long.parseLong(tokens[10]);
        long immigrantsLastYear = Long.parseLong(tokens[11]);
        boolean plagueStruck = "1".equals(tokens[12]);

        long rngInc = Long.parseUnsignedLong(tokens[tokens.length - 1]);
        long rngState = Long.parseUnsignedLong(tokens[tokens.length - 2]);

        StringBuilder sb = new StringBuilder();
        for (int i = 13; i <= tokens.length - 3; i++) {
            if (sb.length() > 0) sb.append(" ");
            sb.append(tokens[i]);
        }
        GameStatus status = parseStatus(sb.toString());

        return new HamurabiState(
            year,
            population,
            bushels,
            acres,
            landPrice,
            harvestPerAcre,
            ratsEaten,
            starvedLastYear,
            starvedTotal,
            percentStarvedSum,
            immigrantsLastYear,
            plagueStruck,
            status,
            rngState,
            rngInc
        );
    }

    private static HamurabiError parseError(String name) {
        return switch (name) {
            case "GameAlreadyEnded" -> HamurabiError.GAME_ALREADY_ENDED;
            case "InvalidOrderBounds" -> HamurabiError.INVALID_ORDER_BOUNDS;
            case "InsufficientBushelsForLand" -> HamurabiError.INSUFFICIENT_BUSHELS_FOR_LAND;
            case "InsufficientAcresToSell" -> HamurabiError.INSUFFICIENT_ACRES_TO_SELL;
            case "InsufficientBushelsToFeed" -> HamurabiError.INSUFFICIENT_BUSHELS_TO_FEED;
            case "InsufficientAcresToPlant" -> HamurabiError.INSUFFICIENT_ACRES_TO_PLANT;
            case "InsufficientPopulationToPlant" -> HamurabiError.INSUFFICIENT_POPULATION_TO_PLANT;
            case "InsufficientGrainForSeed" -> HamurabiError.INSUFFICIENT_GRAIN_FOR_SEED;
            default -> throw new IllegalArgumentException("Unknown error: " + name);
        };
    }

    private static void assertStateEquals(HamurabiState expected, HamurabiState actual, String msg) {
        assertEquals(expected.year(), actual.year(), msg + " [year]");
        assertEquals(expected.population(), actual.population(), msg + " [population]");
        assertEquals(expected.bushels(), actual.bushels(), msg + " [bushels]");
        assertEquals(expected.acres(), actual.acres(), msg + " [acres]");
        assertEquals(expected.landPrice(), actual.landPrice(), msg + " [landPrice]");
        assertEquals(expected.harvestPerAcre(), actual.harvestPerAcre(), msg + " [harvestPerAcre]");
        assertEquals(expected.ratsEaten(), actual.ratsEaten(), msg + " [ratsEaten]");
        assertEquals(expected.starvedLastYear(), actual.starvedLastYear(), msg + " [starvedLastYear]");
        assertEquals(expected.starvedTotal(), actual.starvedTotal(), msg + " [starvedTotal]");
        assertEquals(expected.percentStarvedSum(), actual.percentStarvedSum(), msg + " [percentStarvedSum]");
        assertEquals(expected.immigrantsLastYear(), actual.immigrantsLastYear(), msg + " [immigrantsLastYear]");
        assertEquals(expected.plagueStruck(), actual.plagueStruck(), msg + " [plagueStruck]");
        assertEquals(expected.status(), actual.status(), msg + " [status]");
        assertEquals(expected.rngState(), actual.rngState(), msg + " [rngState]");
        assertEquals(expected.rngInc(), actual.rngInc(), msg + " [rngInc]");
    }

    @Test
    void testEveryCorpusCaseMatches() throws IOException {
        Path path = findCorpusPath();
        List<String> lines = Files.readAllLines(path, StandardCharsets.UTF_8);

        int totalCases = 0;
        int executedTurns = 0;
        int executedRefusals = 0;

        String caseId = "";
        Long caseSeed = null;
        HamurabiState currentState = null;
        TurnOrder pendingOrder = null;

        for (String line : lines) {
            line = line.trim();
            if (line.isEmpty() || line.startsWith("#")) {
                continue;
            }

            if (line.startsWith("COUNT ")) {
                continue;
            }

            if (line.startsWith("CASE ")) {
                String[] tokens = line.split(" ");
                caseId = tokens[1];
                caseSeed = "null".equals(tokens[2]) ? null : Long.parseUnsignedLong(tokens[2]);
                currentState = null;
                pendingOrder = null;
                totalCases++;
            } else if (line.startsWith("INIT ")) {
                currentState = parseStateLine(line);
            } else if (line.startsWith("TURN ")) {
                String[] tokens = line.split(" ");
                long acresDelta = Long.parseLong(tokens[2]);
                long bushelsToFeed = Long.parseUnsignedLong(tokens[3]);
                long acresToPlant = Long.parseUnsignedLong(tokens[4]);
                pendingOrder = new TurnOrder(acresDelta, bushelsToFeed, acresToPlant);
            } else if (line.startsWith("STATE ")) {
                HamurabiState expectedState = parseStateLine(line);
                Result<HamurabiState, HamurabiError> stepRes = Hamurabi.stepYear(currentState, pendingOrder);
                assertTrue(stepRes.isOk(), "Case " + caseId + " stepYear failed with error: " + (stepRes.isErr() ? stepRes.unwrapErr() : "none"));
                HamurabiState actualState = stepRes.unwrap();
                assertStateEquals(expectedState, actualState, "Case " + caseId + " state after turn");
                currentState = actualState;
                executedTurns++;
            } else if (line.startsWith("REPORT ")) {
                String expectedReport = line.substring(7).replace("\\n", "\n");
                String actualReport = Hamurabi.formatReport(currentState);
                assertEquals(expectedReport, actualReport, "Case " + caseId + " report mismatch");
            } else if (line.startsWith("REFUSAL ")) {
                String[] tokens = line.split(" ");
                long acresDelta = Long.parseLong(tokens[1]);
                long bushelsToFeed = Long.parseUnsignedLong(tokens[2]);
                long acresToPlant = Long.parseUnsignedLong(tokens[3]);
                HamurabiError expectedError = parseError(tokens[4]);

                TurnOrder badOrder = new TurnOrder(acresDelta, bushelsToFeed, acresToPlant);

                Result<Void, HamurabiError> valRes = Hamurabi.validateOrder(currentState, badOrder);
                assertTrue(valRes.isErr(), "Case " + caseId + " validateOrder expected error but succeeded");
                assertEquals(expectedError, valRes.unwrapErr(), "Case " + caseId + " validateOrder error mismatch");

                Result<HamurabiState, HamurabiError> stepRes = Hamurabi.stepYear(currentState, badOrder);
                assertTrue(stepRes.isErr(), "Case " + caseId + " stepYear expected error but succeeded");
                assertEquals(expectedError, stepRes.unwrapErr(), "Case " + caseId + " stepYear error mismatch");

                executedRefusals++;
            }
        }

        assertTrue(totalCases >= 31, "Total cases executed: " + totalCases);
        assertTrue(executedTurns >= 40, "Executed turns: " + executedTurns);
        assertTrue(executedRefusals >= 14, "Executed refusals: " + executedRefusals);
    }
}
