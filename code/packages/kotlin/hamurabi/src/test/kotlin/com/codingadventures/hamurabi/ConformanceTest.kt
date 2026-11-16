package com.codingadventures.hamurabi

import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class ConformanceTest {

    private fun findCorpusPath(): File {
        val candidates = listOf(
            File("../../../specs/fixtures/hamurabi-v1/cases.txt"),
            File("code/specs/fixtures/hamurabi-v1/cases.txt"),
            File("../../../../code/specs/fixtures/hamurabi-v1/cases.txt"),
            File("../../code/specs/fixtures/hamurabi-v1/cases.txt")
        )
        for (c in candidates) {
            if (c.exists()) {
                return c.canonicalFile
            }
        }
        error("Could not find hamurabi-v1/cases.txt in candidate locations")
    }

    private fun parseStatus(statusStr: String): GameStatus {
        if (statusStr == "Playing") return GameStatus.Playing
        if (statusStr == "GameOverExtinction") return GameStatus.GameOverExtinction
        if (statusStr.startsWith("Won(")) {
            val ratingStr = statusStr.substring(4, statusStr.length - 1)
            val rating = Rating.valueOf(ratingStr.uppercase())
            return GameStatus.Won(rating)
        }
        if (statusStr.startsWith("Impeached(")) {
            val inner = statusStr.substring(10, statusStr.length - 1)
            val parts = inner.split(", ")
            val starved = parts[0].split("=")[1].toLong()
            val percent = parts[1].split("=")[1].toLong()
            return GameStatus.Impeached(starved, percent)
        }
        error("Unknown status string: $statusStr")
    }

    private fun parseStateLine(line: String): HamurabiState {
        val tokens = line.split(" ")
        val year = tokens[1].toInt()
        val population = tokens[2].toLong()
        val bushels = tokens[3].toLong()
        val acres = tokens[4].toLong()
        val landPrice = tokens[5].toLong()
        val harvestPerAcre = tokens[6].toLong()
        val ratsEaten = tokens[7].toLong()
        val starvedLastYear = tokens[8].toLong()
        val starvedTotal = tokens[9].toLong()
        val percentStarvedSum = tokens[10].toLong()
        val immigrantsLastYear = tokens[11].toLong()
        val plagueStruck = tokens[12] == "1"

        val rngInc = tokens[tokens.size - 1].toULong().toLong()
        val rngState = tokens[tokens.size - 2].toULong().toLong()

        val statusStr = tokens.subList(13, tokens.size - 2).joinToString(" ")
        val status = parseStatus(statusStr)

        return HamurabiState(
            year = year,
            population = population,
            bushels = bushels,
            acres = acres,
            landPrice = landPrice,
            harvestPerAcre = harvestPerAcre,
            ratsEaten = ratsEaten,
            starvedLastYear = starvedLastYear,
            starvedTotal = starvedTotal,
            percentStarvedSum = percentStarvedSum,
            immigrantsLastYear = immigrantsLastYear,
            plagueStruck = plagueStruck,
            status = status,
            rngState = rngState,
            rngInc = rngInc
        )
    }

    private fun parseError(name: String): HamurabiError {
        return when (name) {
            "GameAlreadyEnded" -> HamurabiError.GAME_ALREADY_ENDED
            "InvalidOrderBounds" -> HamurabiError.INVALID_ORDER_BOUNDS
            "InsufficientBushelsForLand" -> HamurabiError.INSUFFICIENT_BUSHELS_FOR_LAND
            "InsufficientAcresToSell" -> HamurabiError.INSUFFICIENT_ACRES_TO_SELL
            "InsufficientBushelsToFeed" -> HamurabiError.INSUFFICIENT_BUSHELS_TO_FEED
            "InsufficientAcresToPlant" -> HamurabiError.INSUFFICIENT_ACRES_TO_PLANT
            "InsufficientPopulationToPlant" -> HamurabiError.INSUFFICIENT_POPULATION_TO_PLANT
            "InsufficientGrainForSeed" -> HamurabiError.INSUFFICIENT_GRAIN_FOR_SEED
            else -> error("Unknown error: $name")
        }
    }

    private fun assertStateEquals(expected: HamurabiState, actual: HamurabiState, msg: String) {
        assertEquals(expected.year, actual.year, "$msg [year]")
        assertEquals(expected.population, actual.population, "$msg [population]")
        assertEquals(expected.bushels, actual.bushels, "$msg [bushels]")
        assertEquals(expected.acres, actual.acres, "$msg [acres]")
        assertEquals(expected.landPrice, actual.landPrice, "$msg [landPrice]")
        assertEquals(expected.harvestPerAcre, actual.harvestPerAcre, "$msg [harvestPerAcre]")
        assertEquals(expected.ratsEaten, actual.ratsEaten, "$msg [ratsEaten]")
        assertEquals(expected.starvedLastYear, actual.starvedLastYear, "$msg [starvedLastYear]")
        assertEquals(expected.starvedTotal, actual.starvedTotal, "$msg [starvedTotal]")
        assertEquals(expected.percentStarvedSum, actual.percentStarvedSum, "$msg [percentStarvedSum]")
        assertEquals(expected.immigrantsLastYear, actual.immigrantsLastYear, "$msg [immigrantsLastYear]")
        assertEquals(expected.plagueStruck, actual.plagueStruck, "$msg [plagueStruck]")
        assertEquals(expected.status, actual.status, "$msg [status]")
        assertEquals(expected.rngState, actual.rngState, "$msg [rngState]")
        assertEquals(expected.rngInc, actual.rngInc, "$msg [rngInc]")
    }

    @Test
    fun testEveryCorpusCaseMatches() {
        val file = findCorpusPath()
        val lines = file.readLines()

        var totalCases = 0
        var executedTurns = 0
        var executedRefusals = 0

        var caseId = ""
        var currentState: HamurabiState? = null
        var pendingOrder: TurnOrder? = null

        for (rawLine in lines) {
            val line = rawLine.trim()
            if (line.isEmpty() || line.startsWith("#")) continue

            if (line.startsWith("COUNT ")) continue

            if (line.startsWith("CASE ")) {
                val tokens = line.split(" ")
                caseId = tokens[1]
                currentState = null
                pendingOrder = null
                totalCases++
            } else if (line.startsWith("INIT ")) {
                currentState = parseStateLine(line)
            } else if (line.startsWith("TURN ")) {
                val tokens = line.split(" ")
                val acresDelta = tokens[2].toLong()
                val bushelsToFeed = tokens[3].toULong().toLong()
                val acresToPlant = tokens[4].toULong().toLong()
                pendingOrder = TurnOrder(acresDelta, bushelsToFeed, acresToPlant)
            } else if (line.startsWith("STATE ")) {
                val expectedState = parseStateLine(line)
                val stepRes = stepYear(currentState!!, pendingOrder!!)
                assertTrue(stepRes.isOk, "Case $caseId stepYear failed: ${if (stepRes.isErr) stepRes.unwrapErr() else "none"}")
                val actualState = stepRes.unwrap()
                assertStateEquals(expectedState, actualState, "Case $caseId state after turn")
                currentState = actualState
                executedTurns++
            } else if (line.startsWith("REPORT ")) {
                val expectedReport = line.substring(7).replace("\\n", "\n")
                val actualReport = formatReport(currentState!!)
                assertEquals(expectedReport, actualReport, "Case $caseId report mismatch")
            } else if (line.startsWith("REFUSAL ")) {
                val tokens = line.split(" ")
                val acresDelta = tokens[1].toLong()
                val bushelsToFeed = tokens[2].toULong().toLong()
                val acresToPlant = tokens[3].toULong().toLong()
                val expectedError = parseError(tokens[4])

                val badOrder = TurnOrder(acresDelta, bushelsToFeed, acresToPlant)

                val valRes = validateOrder(currentState!!, badOrder)
                assertTrue(valRes.isErr, "Case $caseId validateOrder expected error but succeeded")
                assertEquals(expectedError, valRes.unwrapErr(), "Case $caseId validateOrder error mismatch")

                val stepRes = stepYear(currentState!!, badOrder)
                assertTrue(stepRes.isErr, "Case $caseId stepYear expected error but succeeded")
                assertEquals(expectedError, stepRes.unwrapErr(), "Case $caseId stepYear error mismatch")

                executedRefusals++
            }
        }

        assertTrue(totalCases >= 31, "Total cases executed: $totalCases")
        assertTrue(executedTurns >= 40, "Executed turns: $executedTurns")
        assertTrue(executedRefusals >= 14, "Executed refusals: $executedRefusals")
    }
}
