import { describe, it, expect, beforeEach } from "vitest";
import { Store } from "@coding-adventures/store";
import { reducer, initialState } from "../reducer.js";
import { seedDeck } from "../seed.js";
import type { AppState } from "../types.js";

describe("seedDeck", () => {
  let store: Store<AppState>;

  beforeEach(() => {
    store = new Store<AppState>(initialState, reducer);
  });

  it("creates exactly 1 deck", () => {
    seedDeck(store);
    expect(store.getState().decks).toHaveLength(1);
  });

  it("names the deck 'US State Capitals'", () => {
    seedDeck(store);
    expect(store.getState().decks[0]!.name).toBe("US State Capitals");
  });

  it("creates exactly 50 cards", () => {
    seedDeck(store);
    expect(store.getState().cards).toHaveLength(50);
  });

  it("all cards belong to the seeded deck", () => {
    seedDeck(store);
    const state = store.getState();
    const deckId = state.decks[0]!.id;
    expect(state.cards.every((c) => c.deckId === deckId)).toBe(true);
  });

  it("all card IDs are unique", () => {
    seedDeck(store);
    const ids = store.getState().cards.map((c) => c.id);
    expect(new Set(ids).size).toBe(50);
  });

  it("contains Alabama → Montgomery", () => {
    seedDeck(store);
    const card = store
      .getState()
      .cards.find((c) => c.front.includes("Alabama"));
    expect(card).toBeDefined();
    expect(card!.back).toBe("Montgomery");
  });

  it("contains California → Sacramento", () => {
    seedDeck(store);
    const card = store
      .getState()
      .cards.find((c) => c.front.includes("California"));
    expect(card).toBeDefined();
    expect(card!.back).toBe("Sacramento");
  });

  it("contains Wyoming → Cheyenne (last state alphabetically)", () => {
    seedDeck(store);
    const card = store
      .getState()
      .cards.find((c) => c.front.includes("Wyoming"));
    expect(card).toBeDefined();
    expect(card!.back).toBe("Cheyenne");
  });

  it("contains Hawaii → Honolulu", () => {
    seedDeck(store);
    const card = store
      .getState()
      .cards.find((c) => c.front.includes("Hawaii"));
    expect(card).toBeDefined();
    expect(card!.back).toBe("Honolulu");
  });

  it("contains Texas → Austin", () => {
    seedDeck(store);
    const card = store.getState().cards.find((c) => c.front.includes("Texas"));
    expect(card).toBeDefined();
    expect(card!.back).toBe("Austin");
  });

  it("all fronts follow the 'What is the capital of X?' format", () => {
    seedDeck(store);
    const cards = store.getState().cards;
    expect(
      cards.every((c) => c.front.startsWith("What is the capital of ")),
    ).toBe(true);
  });

  it("no card has an empty back", () => {
    seedDeck(store);
    expect(store.getState().cards.every((c) => c.back.length > 0)).toBe(true);
  });

  it("does not create any CardProgress records on seed", () => {
    seedDeck(store);
    expect(store.getState().cardProgress).toHaveLength(0);
  });

  it("does not create any sessions on seed", () => {
    seedDeck(store);
    expect(store.getState().sessions).toHaveLength(0);
  });
});
