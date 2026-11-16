package com.codingadventures.hamurabi;

/**
 * Immutable value record capturing the complete simulation state.
 *
 * @param year Current year of reign: 1..=10 are playable; 11 is terminal summary state.
 * @param population Current population of the city-state (initial: 100).
 * @param bushels Bushels of grain in storage (initial: 2800).
 * @param acres Acres of farmland owned by the city (initial: 1000).
 * @param landPrice Current market price of land in bushels per acre (17..=26).
 * @param harvestPerAcre Yield from the previous harvest in bushels per acre (1..=5).
 * @param ratsEaten Bushels consumed by rats during previous year (initial: 200).
 * @param starvedLastYear Citizens starved during previous year (initial: 0).
 * @param starvedTotal Cumulative citizens starved across the entire reign.
 * @param percentStarvedSum Sum of yearly starvation percentages (for 10-year average computation).
 * @param immigrantsLastYear New citizens arriving during previous year (initial: 5).
 * @param plagueStruck Whether a plague occurred in the previous year.
 * @param status Current game lifecycle status.
 * @param rngState Internal 64-bit state of the PCG32 generator.
 * @param rngInc Internal 64-bit sequence increment of PCG32 (always odd).
 */
public record HamurabiState(
    int year,
    long population,
    long bushels,
    long acres,
    long landPrice,
    long harvestPerAcre,
    long ratsEaten,
    long starvedLastYear,
    long starvedTotal,
    long percentStarvedSum,
    long immigrantsLastYear,
    boolean plagueStruck,
    GameStatus status,
    long rngState,
    long rngInc
) {}
