package com.codingadventures.hamurabi;

/**
 * Simulation lifecycle status.
 */
public sealed interface GameStatus permits
        GameStatus.Playing,
        GameStatus.Won,
        GameStatus.Impeached,
        GameStatus.GameOverExtinction {

    record Playing() implements GameStatus {
        @Override
        public String toString() {
            return "Playing";
        }
    }

    record Won(Rating rating) implements GameStatus {
        @Override
        public String toString() {
            String r = switch (rating) {
                case CHARLEMAGNE -> "Charlemagne";
                case ACRE -> "Acre";
                case NERO -> "Nero";
                case DISASTER -> "Disaster";
            };
            return "Won(" + r + ")";
        }
    }

    record Impeached(long starvedThisTurn, long percent) implements GameStatus {
        @Override
        public String toString() {
            return "Impeached(starved=" + starvedThisTurn + ", percent=" + percent + ")";
        }
    }

    record GameOverExtinction() implements GameStatus {
        @Override
        public String toString() {
            return "GameOverExtinction";
        }
    }
}
