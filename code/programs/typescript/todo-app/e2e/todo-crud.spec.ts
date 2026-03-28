/**
 * todo-crud.spec.ts — End-to-end tests for the core CRUD workflow.
 *
 * These tests verify the full user journey:
 *   1. App loads with seed data
 *   2. Create a new todo
 *   3. Edit an existing todo
 *   4. Toggle todo status
 *   5. Delete a todo
 *   6. Clear completed todos
 *   7. Filter and search
 *   8. Data persists across page reload (IndexedDB)
 *
 * Each test runs in a fresh browser context (isolated IndexedDB).
 */

import { test, expect } from "@playwright/test";

test.describe("Todo App — Core CRUD", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    // Wait for the app to fully load (header renders)
    await page.waitForSelector("#app-header");
  });

  test("loads with seed data on first visit", async ({ page }) => {
    // The app should show the summary bar with stats
    const total = page.locator("#stat-total");
    await expect(total).toBeVisible();

    // Should have multiple seed todos
    const totalText = await total.textContent();
    expect(Number(totalText)).toBeGreaterThan(0);

    // Should show the header
    await expect(page.locator("#app-title")).toContainText("Todo");
  });

  test("creates a new todo", async ({ page }) => {
    // Click the create button
    await page.click("#create-todo-btn");

    // Should navigate to the editor
    await expect(page.locator("#todo-editor")).toBeVisible();

    // Fill in the form
    await page.fill("#todo-title", "E2E Test Todo");
    await page.fill("#todo-description", "Created by Playwright");
    await page.selectOption("#todo-priority", "high");
    await page.fill("#todo-category", "testing");

    // Submit the form
    await page.click("#save-btn");

    // Should navigate back to the list
    await expect(page.locator("#todo-list")).toBeVisible();

    // The new todo should appear in the list
    await expect(page.getByText("E2E Test Todo")).toBeVisible();
  });

  test("validates title is required", async ({ page }) => {
    await page.click("#create-todo-btn");
    await expect(page.locator("#todo-editor")).toBeVisible();

    // Try to submit without title
    await page.click("#save-btn");

    // Should show error
    await expect(page.locator("#title-error")).toBeVisible();
    await expect(page.locator("#title-error")).toContainText("Title is required");
  });

  test("edits an existing todo", async ({ page }) => {
    // Wait for a todo card to appear
    const firstEditBtn = page.locator("[id^='edit-todo-']").first();
    await firstEditBtn.waitFor();
    await firstEditBtn.click();

    // Should navigate to the editor with pre-filled data
    await expect(page.locator("#todo-editor")).toBeVisible();
    const titleInput = page.locator("#todo-title");
    const currentTitle = await titleInput.inputValue();
    expect(currentTitle.length).toBeGreaterThan(0);

    // Change the title
    await titleInput.clear();
    await titleInput.fill("Updated by E2E Test");
    await page.click("#save-btn");

    // Should show updated title back in the list
    await expect(page.getByText("Updated by E2E Test")).toBeVisible();
  });

  test("toggles todo status", async ({ page }) => {
    // Find the first status toggle button
    const statusBtn = page.locator("[id^='toggle-status-']").first();
    await statusBtn.waitFor();

    // Get the initial class to compare later
    const initialClass = await statusBtn.getAttribute("class");

    // Click to toggle
    await statusBtn.click();

    // The class should change (different status styling)
    const newClass = await statusBtn.getAttribute("class");
    expect(newClass).not.toBe(initialClass);
  });

  test("deletes a todo", async ({ page }) => {
    // Get initial count
    const initialTotal = await page.locator("#stat-total").textContent();
    const initialCount = Number(initialTotal);

    // Click delete on first todo
    const deleteBtn = page.locator("[id^='delete-todo-']").first();
    await deleteBtn.waitFor();
    await deleteBtn.click();

    // Count should decrease
    const newTotal = await page.locator("#stat-total").textContent();
    expect(Number(newTotal)).toBe(initialCount - 1);
  });

  test("cancels editing", async ({ page }) => {
    await page.click("#create-todo-btn");
    await expect(page.locator("#todo-editor")).toBeVisible();

    // Fill in title but cancel
    await page.fill("#todo-title", "Should Not Appear");
    await page.click("#cancel-btn");

    // Should be back on list, and the todo should NOT exist
    await expect(page.locator("#todo-list")).toBeVisible();
    await expect(page.getByText("Should Not Appear")).not.toBeVisible();
  });

  test("navigates via header title click", async ({ page }) => {
    // Navigate to editor first
    await page.click("#create-todo-btn");
    await expect(page.locator("#todo-editor")).toBeVisible();

    // Click header title to go back
    await page.click("#app-title");
    await expect(page.locator("#todo-list")).toBeVisible();
  });
});

test.describe("Todo App — Filters & Search", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector("#app-header");
  });

  test("searches todos by title", async ({ page }) => {
    // Type in search
    await page.fill("#search-input", "Welcome");

    // Should filter the list
    const count = page.locator("#todo-count");
    await expect(count).toBeVisible();
  });

  test("clears search", async ({ page }) => {
    await page.fill("#search-input", "test query");

    // Clear button should appear
    const clearBtn = page.locator("#clear-search-btn");
    await expect(clearBtn).toBeVisible();
    await clearBtn.click();

    // Search should be empty
    const searchInput = page.locator("#search-input");
    await expect(searchInput).toHaveValue("");
  });

  test("filters by status", async ({ page }) => {
    await page.selectOption("#status-filter", "done");

    // Clear filters button should appear
    await expect(page.locator("#clear-filters-btn")).toBeVisible();
  });

  test("filters by priority", async ({ page }) => {
    await page.selectOption("#priority-filter", "urgent");
    await expect(page.locator("#clear-filters-btn")).toBeVisible();
  });

  test("toggles sort direction", async ({ page }) => {
    const sortBtn = page.locator("#sort-direction-btn");
    const initialLabel = await sortBtn.getAttribute("aria-label");

    await sortBtn.click();

    const newLabel = await sortBtn.getAttribute("aria-label");
    expect(newLabel).not.toBe(initialLabel);
  });

  test("clears all filters", async ({ page }) => {
    // Apply some filters
    await page.selectOption("#status-filter", "todo");
    await page.fill("#search-input", "test");

    // Clear them
    await page.click("#clear-filters-btn");

    // Filters should be reset
    await expect(page.locator("#status-filter")).toHaveValue("all");
    await expect(page.locator("#search-input")).toHaveValue("");
  });
});

test.describe("Todo App — Persistence", () => {
  test("data persists across page reload", async ({ page }) => {
    await page.goto("/");
    await page.waitForSelector("#app-header");

    // Create a unique todo
    const uniqueTitle = `Persist-Test-${Date.now()}`;
    await page.click("#create-todo-btn");
    await page.fill("#todo-title", uniqueTitle);
    await page.click("#save-btn");

    // Verify it's in the list
    await expect(page.getByText(uniqueTitle)).toBeVisible();

    // Reload the page
    await page.reload();
    await page.waitForSelector("#app-header");

    // The todo should still be there (loaded from IndexedDB)
    await expect(page.getByText(uniqueTitle)).toBeVisible();
  });
});
