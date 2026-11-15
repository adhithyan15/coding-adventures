package com.codingadventures.hamurabi;

/**
 * Player orders for a single year.
 *
 * @param acresDelta Acres to buy (positive) or sell (negative). 0 for no trade.
 * @param bushelsToFeed Bushels of grain allocated to feed the population (20 bushels/person).
 * @param acresToPlant Acres of owned land to sow with grain (1 bushel seeds 2 acres).
 */
public record TurnOrder(
    long acresDelta,
    long bushelsToFeed,
    long acresToPlant
) {}
