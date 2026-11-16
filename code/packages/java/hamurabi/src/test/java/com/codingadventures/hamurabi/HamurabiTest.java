package com.codingadventures.hamurabi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class HamurabiTest {

    @Test
    void testNewGameInitialState() {
        HamurabiState state = Hamurabi.newGame(42L);
        assertEquals(1, state.year());
        assertEquals(100L, state.population());
        assertEquals(2800L, state.bushels());
        assertEquals(1000L, state.acres());
        assertTrue(state.landPrice() >= 17 && state.landPrice() <= 26);
        assertEquals(3L, state.harvestPerAcre());
        assertEquals(200L, state.ratsEaten());
        assertEquals(0L, state.starvedLastYear());
        assertEquals(0L, state.starvedTotal());
        assertEquals(0L, state.percentStarvedSum());
        assertEquals(5L, state.immigrantsLastYear());
        assertFalse(state.plagueStruck());
        assertInstanceOf(GameStatus.Playing.class, state.status());
        assertEquals(109L, state.rngInc());
    }

    @Test
    void testOrderValidationSuccess() {
        HamurabiState state = Hamurabi.newGame(42L);
        TurnOrder validOrder = new TurnOrder(0, 2000, 1000);
        Result<Void, HamurabiError> res = Hamurabi.validateOrder(state, validOrder);
        assertTrue(res.isOk());
    }

    @Test
    void testValidateGameAlreadyEnded() {
        HamurabiState state = new HamurabiState(
            11, 80, 2000, 1000, 20, 3, 0, 0, 0, 0, 0, false,
            new GameStatus.Won(Rating.CHARLEMAGNE), 12345L, 109L
        );
        TurnOrder order = new TurnOrder(0, 1600, 800);
        Result<Void, HamurabiError> res = Hamurabi.validateOrder(state, order);
        assertTrue(res.isErr());
        assertEquals(HamurabiError.GAME_ALREADY_ENDED, res.unwrapErr());
    }

    @Test
    void testValidateInvalidOrderBoundsOddInc() {
        HamurabiState state = new HamurabiState(
            1, 100, 2800, 1000, 20, 3, 200, 0, 0, 0, 5, false,
            new GameStatus.Playing(), 12345L, 108L // even inc!
        );
        TurnOrder order = new TurnOrder(0, 2000, 1000);
        Result<Void, HamurabiError> res = Hamurabi.validateOrder(state, order);
        assertTrue(res.isErr());
        assertEquals(HamurabiError.INVALID_ORDER_BOUNDS, res.unwrapErr());
    }

    @Test
    void testValidateInvalidOrderBoundsMinInt() {
        HamurabiState state = Hamurabi.newGame(42L);
        // Integer.MIN_VALUE - 1 is outside 32-bit signed bounds
        TurnOrder order = new TurnOrder(((long) Integer.MIN_VALUE) - 1L, 2000, 1000);
        Result<Void, HamurabiError> res = Hamurabi.validateOrder(state, order);
        assertTrue(res.isErr());
        assertEquals(HamurabiError.INVALID_ORDER_BOUNDS, res.unwrapErr());
    }

    @Test
    void testValidateInsufficientBushelsForLand() {
        HamurabiState state = Hamurabi.newGame(42L);
        // Buy 200 acres at e.g. >= 17 bushels/acre requires >= 3400 bushels, but city has 2800
        TurnOrder order = new TurnOrder(200, 0, 0);
        Result<Void, HamurabiError> res = Hamurabi.validateOrder(state, order);
        assertTrue(res.isErr());
        assertEquals(HamurabiError.INSUFFICIENT_BUSHELS_FOR_LAND, res.unwrapErr());
    }

    @Test
    void testValidateInsufficientAcresToSell() {
        HamurabiState state = Hamurabi.newGame(42L);
        TurnOrder order = new TurnOrder(-1001, 0, 0);
        Result<Void, HamurabiError> res = Hamurabi.validateOrder(state, order);
        assertTrue(res.isErr());
        assertEquals(HamurabiError.INSUFFICIENT_ACRES_TO_SELL, res.unwrapErr());
    }

    @Test
    void testValidateInsufficientBushelsToFeed() {
        HamurabiState state = Hamurabi.newGame(42L);
        TurnOrder order = new TurnOrder(0, 2801, 0);
        Result<Void, HamurabiError> res = Hamurabi.validateOrder(state, order);
        assertTrue(res.isErr());
        assertEquals(HamurabiError.INSUFFICIENT_BUSHELS_TO_FEED, res.unwrapErr());
    }

    @Test
    void testValidateInsufficientAcresToPlant() {
        HamurabiState state = Hamurabi.newGame(42L);
        TurnOrder order = new TurnOrder(0, 2000, 1001);
        Result<Void, HamurabiError> res = Hamurabi.validateOrder(state, order);
        assertTrue(res.isErr());
        assertEquals(HamurabiError.INSUFFICIENT_ACRES_TO_PLANT, res.unwrapErr());
    }

    @Test
    void testValidateInsufficientPopulationToPlant() {
        HamurabiState state = Hamurabi.newGame(42L);
        // 100 people can tend at most 1000 acres; buying 10 acres gives 1010 acres, but planting 1001 fails labor check
        TurnOrder order = new TurnOrder(10, 2000, 1001);
        Result<Void, HamurabiError> res = Hamurabi.validateOrder(state, order);
        assertTrue(res.isErr());
        assertEquals(HamurabiError.INSUFFICIENT_POPULATION_TO_PLANT, res.unwrapErr());
    }

    @Test
    void testValidateInsufficientGrainForSeed() {
        HamurabiState state = Hamurabi.newGame(42L);
        // Feeding 2700 leaves 100 bushels. Planting 300 acres requires ceil(300/2) = 150 bushels
        TurnOrder order = new TurnOrder(0, 2700, 300);
        Result<Void, HamurabiError> res = Hamurabi.validateOrder(state, order);
        assertTrue(res.isErr());
        assertEquals(HamurabiError.INSUFFICIENT_GRAIN_FOR_SEED, res.unwrapErr());
    }

    @Test
    void testFormatReportPlaying() {
        HamurabiState state = Hamurabi.newGame(42L);
        String report = Hamurabi.formatReport(state);
        assertTrue(report.startsWith("HAMURABI: I BEG TO REPORT TO YOU,\n"));
        assertTrue(report.contains("POPULATION IS NOW 100.\n"));
        assertTrue(report.contains("THE CITY NOW OWNS 1000 ACRES.\n"));
    }

    @Test
    void testFormatReportImpeached() {
        HamurabiState state = new HamurabiState(
            2, 0, 0, 1000, 22, 3, 2800, 100, 100, 100, 0, false,
            new GameStatus.Impeached(100, 100), 12345L, 109L
        );
        String report = Hamurabi.formatReport(state);
        assertTrue(report.contains("YOU STARVED 100 PEOPLE IN ONE YEAR!!!\n"));
    }

    @Test
    void testFormatReportExtinct() {
        HamurabiState state = new HamurabiState(
            2, 0, 0, 1000, 22, 3, 2800, 100, 100, 100, 0, false,
            new GameStatus.GameOverExtinction(), 12345L, 109L
        );
        String report = Hamurabi.formatReport(state);
        assertEquals("EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n", report);
    }
}
