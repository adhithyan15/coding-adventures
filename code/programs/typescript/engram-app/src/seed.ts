/**
 * seed.ts — First-launch seed data: all 50 US state capitals.
 *
 * On first launch (no decks in IndexedDB), the app seeds one deck so the
 * user can immediately start studying without any setup. The deck contains
 * all 50 US states and their capitals.
 *
 * Cards are authored as questions to encourage active recall:
 *   Front: "What is the capital of {State}?"
 *   Back:  "{Capital}"
 *
 * This mirrors the natural study direction — you know the state and need
 * to recall the capital. The reverse (capital → state) could be a future
 * second deck.
 *
 * Cards are ordered alphabetically by state. The queue builder will
 * present them in this order for new cards, which is a logical sequence
 * that matches how students typically memorise them.
 *
 * === Dispatching through the store ===
 *
 * seedDeck dispatches DECK_CREATE and CARD_CREATE actions rather than
 * directly mutating state. This means the persistence middleware
 * automatically saves each record to IndexedDB as it is created.
 */

import type { Store } from "@coding-adventures/store";
import { createDeckAction, createCardAction } from "./actions.js";
import type { AppState } from "./reducer.js";

/** All 50 US states and their capitals, sorted alphabetically by state. */
const STATE_CAPITALS: Array<[string, string]> = [
  ["Alabama", "Montgomery"],
  ["Alaska", "Juneau"],
  ["Arizona", "Phoenix"],
  ["Arkansas", "Little Rock"],
  ["California", "Sacramento"],
  ["Colorado", "Denver"],
  ["Connecticut", "Hartford"],
  ["Delaware", "Dover"],
  ["Florida", "Tallahassee"],
  ["Georgia", "Atlanta"],
  ["Hawaii", "Honolulu"],
  ["Idaho", "Boise"],
  ["Illinois", "Springfield"],
  ["Indiana", "Indianapolis"],
  ["Iowa", "Des Moines"],
  ["Kansas", "Topeka"],
  ["Kentucky", "Frankfort"],
  ["Louisiana", "Baton Rouge"],
  ["Maine", "Augusta"],
  ["Maryland", "Annapolis"],
  ["Massachusetts", "Boston"],
  ["Michigan", "Lansing"],
  ["Minnesota", "Saint Paul"],
  ["Mississippi", "Jackson"],
  ["Missouri", "Jefferson City"],
  ["Montana", "Helena"],
  ["Nebraska", "Lincoln"],
  ["Nevada", "Carson City"],
  ["New Hampshire", "Concord"],
  ["New Jersey", "Trenton"],
  ["New Mexico", "Santa Fe"],
  ["New York", "Albany"],
  ["North Carolina", "Raleigh"],
  ["North Dakota", "Bismarck"],
  ["Ohio", "Columbus"],
  ["Oklahoma", "Oklahoma City"],
  ["Oregon", "Salem"],
  ["Pennsylvania", "Harrisburg"],
  ["Rhode Island", "Providence"],
  ["South Carolina", "Columbia"],
  ["South Dakota", "Pierre"],
  ["Tennessee", "Nashville"],
  ["Texas", "Austin"],
  ["Utah", "Salt Lake City"],
  ["Vermont", "Montpelier"],
  ["Virginia", "Richmond"],
  ["Washington", "Olympia"],
  ["West Virginia", "Charleston"],
  ["Wisconsin", "Madison"],
  ["Wyoming", "Cheyenne"],
];

/**
 * seedDeck — create the US State Capitals deck on first launch.
 *
 * Dispatches one DECK_CREATE action followed by 50 CARD_CREATE actions.
 * The persistence middleware saves each record to IndexedDB automatically.
 *
 * @param store - The Flux store instance.
 */
export function seedDeck(store: Store<AppState>): void {
  // ── Create the deck ───────────────────────────────────────────────────────

  store.dispatch(
    createDeckAction(
      "US State Capitals",
      "All 50 US state capitals. Learn to recall each state's capital city.",
    ),
  );

  // ── Find the deck ID we just created ─────────────────────────────────────
  //
  // The deck was appended to state.decks by the reducer. We need its ID
  // to associate the cards with it.

  const state = store.getState();
  const deck = state.decks[state.decks.length - 1];
  if (!deck) return;

  // ── Create one card per state ─────────────────────────────────────────────

  for (const [state_name, capital] of STATE_CAPITALS) {
    store.dispatch(
      createCardAction(
        deck.id,
        `What is the capital of ${state_name}?`,
        capital,
      ),
    );
  }
}
