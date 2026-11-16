package com.codingadventures.hamurabi

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertTrue

class HamurabiTest {

    @Test
    fun testNewGameInitialState() {
        val state = newGame(42L)
        assertEquals(1, state.year)
        assertEquals(100L, state.population)
        assertEquals(2800L, state.bushels)
        assertEquals(1000L, state.acres)
        assertTrue(state.landPrice in 17L..26L)
        assertEquals(3L, state.harvestPerAcre)
        assertEquals(200L, state.ratsEaten)
        assertEquals(0L, state.starvedLastYear)
        assertEquals(0L, state.starvedTotal)
        assertEquals(0L, state.percentStarvedSum)
        assertEquals(5L, state.immigrantsLastYear)
        assertFalse(state.plagueStruck)
        assertIs<GameStatus.Playing>(state.status)
        assertEquals(109L, state.rngInc)
    }

    @Test
    fun testOrderValidationSuccess() {
        val state = newGame(42L)
        val validOrder = TurnOrder(0L, 2000L, 1000L)
        val res = validateOrder(state, validOrder)
        assertTrue(res.isOk)
    }

    @Test
    fun testValidateGameAlreadyEnded() {
        val state = HamurabiState(
            year = 11, population = 80L, bushels = 2000L, acres = 1000L,
            landPrice = 20L, harvestPerAcre = 3L, ratsEaten = 0L, starvedLastYear = 0L,
            starvedTotal = 0L, percentStarvedSum = 0L, immigrantsLastYear = 0L,
            plagueStruck = false, status = GameStatus.Won(Rating.CHARLEMAGNE),
            rngState = 12345L, rngInc = 109L
        )
        val order = TurnOrder(0L, 1600L, 800L)
        val res = validateOrder(state, order)
        assertTrue(res.isErr)
        assertEquals(HamurabiError.GAME_ALREADY_ENDED, res.unwrapErr())
    }

    @Test
    fun testValidateInvalidOrderBoundsOddInc() {
        val state = HamurabiState(
            year = 1, population = 100L, bushels = 2800L, acres = 1000L,
            landPrice = 20L, harvestPerAcre = 3L, ratsEaten = 200L, starvedLastYear = 0L,
            starvedTotal = 0L, percentStarvedSum = 0L, immigrantsLastYear = 5L,
            plagueStruck = false, status = GameStatus.Playing,
            rngState = 12345L, rngInc = 108L // even increment
        )
        val order = TurnOrder(0L, 2000L, 1000L)
        val res = validateOrder(state, order)
        assertTrue(res.isErr)
        assertEquals(HamurabiError.INVALID_ORDER_BOUNDS, res.unwrapErr())
    }

    @Test
    fun testValidateInvalidOrderBoundsMinInt() {
        val state = newGame(42L)
        val order = TurnOrder(Int.MIN_VALUE.toLong() - 1L, 2000L, 1000L)
        val res = validateOrder(state, order)
        assertTrue(res.isErr)
        assertEquals(HamurabiError.INVALID_ORDER_BOUNDS, res.unwrapErr())
    }

    @Test
    fun testValidateInsufficientBushelsForLand() {
        val state = newGame(42L)
        val order = TurnOrder(200L, 0L, 0L)
        val res = validateOrder(state, order)
        assertTrue(res.isErr)
        assertEquals(HamurabiError.INSUFFICIENT_BUSHELS_FOR_LAND, res.unwrapErr())
    }

    @Test
    fun testValidateInsufficientAcresToSell() {
        val state = newGame(42L)
        val order = TurnOrder(-1001L, 0L, 0L)
        val res = validateOrder(state, order)
        assertTrue(res.isErr)
        assertEquals(HamurabiError.INSUFFICIENT_ACRES_TO_SELL, res.unwrapErr())
    }

    @Test
    fun testValidateInsufficientBushelsToFeed() {
        val state = newGame(42L)
        val order = TurnOrder(0L, 2801L, 0L)
        val res = validateOrder(state, order)
        assertTrue(res.isErr)
        assertEquals(HamurabiError.INSUFFICIENT_BUSHELS_TO_FEED, res.unwrapErr())
    }

    @Test
    fun testValidateInsufficientAcresToPlant() {
        val state = newGame(42L)
        val order = TurnOrder(0L, 2000L, 1001L)
        val res = validateOrder(state, order)
        assertTrue(res.isErr)
        assertEquals(HamurabiError.INSUFFICIENT_ACRES_TO_PLANT, res.unwrapErr())
    }

    @Test
    fun testValidateInsufficientPopulationToPlant() {
        val state = newGame(42L)
        val order = TurnOrder(10L, 2000L, 1001L)
        val res = validateOrder(state, order)
        assertTrue(res.isErr)
        assertEquals(HamurabiError.INSUFFICIENT_POPULATION_TO_PLANT, res.unwrapErr())
    }

    @Test
    fun testValidateInsufficientGrainForSeed() {
        val state = newGame(42L)
        val order = TurnOrder(0L, 2700L, 300L)
        val res = validateOrder(state, order)
        assertTrue(res.isErr)
        assertEquals(HamurabiError.INSUFFICIENT_GRAIN_FOR_SEED, res.unwrapErr())
    }

    @Test
    fun testFormatReportPlaying() {
        val state = newGame(42L)
        val report = formatReport(state)
        assertTrue(report.startsWith("HAMURABI: I BEG TO REPORT TO YOU,\n"))
        assertTrue(report.contains("POPULATION IS NOW 100.\n"))
        assertTrue(report.contains("THE CITY NOW OWNS 1000 ACRES.\n"))
    }

    @Test
    fun testFormatReportImpeached() {
        val state = HamurabiState(
            year = 2, population = 0L, bushels = 0L, acres = 1000L,
            landPrice = 22L, harvestPerAcre = 3L, ratsEaten = 2800L,
            starvedLastYear = 100L, starvedTotal = 100L, percentStarvedSum = 100L,
            immigrantsLastYear = 0L, plagueStruck = false,
            status = GameStatus.Impeached(100L, 100L), rngState = 12345L, rngInc = 109L
        )
        val report = formatReport(state)
        assertTrue(report.contains("YOU STARVED 100 PEOPLE IN ONE YEAR!!!\n"))
    }

    @Test
    fun testFormatReportExtinct() {
        val state = HamurabiState(
            year = 2, population = 0L, bushels = 0L, acres = 1000L,
            landPrice = 22L, harvestPerAcre = 3L, ratsEaten = 2800L,
            starvedLastYear = 100L, starvedTotal = 100L, percentStarvedSum = 100L,
            immigrantsLastYear = 0L, plagueStruck = false,
            status = GameStatus.GameOverExtinction, rngState = 12345L, rngInc = 109L
        )
        val report = formatReport(state)
        assertEquals("EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n", report)
    }
}
