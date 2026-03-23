/**
 * Tests for groups -- print the groups a user is in.
 *
 * We test the exported business logic functions: getCurrentGroups
 * and getUserGroups. Since these depend on the system state, we
 * verify structure and basic properties.
 */

import { describe, it, expect } from "vitest";
import { getCurrentGroups, getUserGroups } from "../src/groups.js";

// ---------------------------------------------------------------------------
// getCurrentGroups.
// ---------------------------------------------------------------------------

describe("getCurrentGroups", () => {
  it("should return a non-empty array", () => {
    const groups = getCurrentGroups();
    expect(Array.isArray(groups)).toBe(true);
    expect(groups.length).toBeGreaterThan(0);
  });

  it("should return strings (group names)", () => {
    const groups = getCurrentGroups();
    for (const group of groups) {
      expect(typeof group).toBe("string");
      expect(group.length).toBeGreaterThan(0);
    }
  });

  it("should return consistent results on repeated calls", () => {
    const groups1 = getCurrentGroups();
    const groups2 = getCurrentGroups();
    expect(groups1).toEqual(groups2);
  });

  it("should not contain empty strings", () => {
    const groups = getCurrentGroups();
    for (const group of groups) {
      expect(group.trim()).not.toBe("");
    }
  });
});

// ---------------------------------------------------------------------------
// getUserGroups.
// ---------------------------------------------------------------------------

describe("getUserGroups", () => {
  it("should return groups for the current user by username", () => {
    const username = process.env.USER;
    if (!username) return; // Skip if USER not set.

    const groups = getUserGroups(username);
    expect(groups).not.toBeNull();
    expect(Array.isArray(groups)).toBe(true);
    expect(groups!.length).toBeGreaterThan(0);
  });

  it("should return null for a non-existent user", () => {
    const groups = getUserGroups("nonexistent_user_xyz_12345");
    expect(groups).toBeNull();
  });

  it("should return the same groups as getCurrentGroups for current user", () => {
    const username = process.env.USER;
    if (!username) return;

    const currentGroups = getCurrentGroups();
    const userGroups = getUserGroups(username);

    expect(userGroups).not.toBeNull();
    // The groups should be the same set (though order might vary).
    expect(userGroups!.sort()).toEqual(currentGroups.sort());
  });

  it("should return string group names", () => {
    const username = process.env.USER;
    if (!username) return;

    const groups = getUserGroups(username);
    if (groups) {
      for (const group of groups) {
        expect(typeof group).toBe("string");
        expect(group.length).toBeGreaterThan(0);
      }
    }
  });
});
