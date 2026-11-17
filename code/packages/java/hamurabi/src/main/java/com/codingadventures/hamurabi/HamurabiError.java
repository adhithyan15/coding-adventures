package com.codingadventures.hamurabi;

/**
 * Closed enumeration of order rejection errors.
 */
public enum HamurabiError {
    GAME_ALREADY_ENDED("Game has already ended"),
    INVALID_ORDER_BOUNDS("Order parameters exceed valid numerical bounds or overflow"),
    INSUFFICIENT_BUSHELS_FOR_LAND("Insufficient bushels to buy specified acreage"),
    INSUFFICIENT_ACRES_TO_SELL("Cannot sell more acres than the city owns"),
    INSUFFICIENT_BUSHELS_TO_FEED("Insufficient bushels in store to feed population"),
    INSUFFICIENT_ACRES_TO_PLANT("Cannot plant more acres than owned after land trade"),
    INSUFFICIENT_POPULATION_TO_PLANT("Insufficient population to tend planted acreage"),
    INSUFFICIENT_GRAIN_FOR_SEED("Insufficient grain to seed planted acreage");

    private final String message;

    HamurabiError(String message) {
        this.message = message;
    }

    public String getMessage() {
        return message;
    }

    @Override
    public String toString() {
        return message;
    }
}
