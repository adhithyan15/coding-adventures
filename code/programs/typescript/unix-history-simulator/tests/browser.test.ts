// @vitest-environment jsdom
import { expect, it } from "vitest";
import html from "../index.html?raw";

it("loads commands and updates the trace, console and descriptors through the controls", async () => {
  document.documentElement.innerHTML = html;
  await import("../src/index.js");

  const click = (selector: string) => document.querySelector<HTMLButtonElement>(selector)!.click();
  const text = (selector: string) => document.querySelector(selector)!.textContent;
  click("#step");
  expect(text("#trace")).toContain("open(/hello)");
  expect(text("#descriptors")).toContain("fd 0 · inode 2 · offset 0");
  click("#step");
  expect(text("#output")).toBe("Hello, U");
  expect(text("#descriptors")).toContain("offset 8");
  click("#run");
  expect(text("#output")).toBe("Hello, Unix!\n");
  expect(text("#trace")).toContain("EOF · 0 bytes");
  expect(text("#trace")).toContain("close(0)");
  expect(text("#descriptors")).toBe("None");

  click('[data-command="ls /etc"]');
  click("#run");
  expect(text("#output")).toBe("notes\n");

  const command = document.querySelector<HTMLInputElement>("#command")!;
  command.value = "cat /<strong>";
  click('#command-form button[type="submit"]');
  click("#run");
  expect(text("#output")).toBe("error: ENOENT\n");
  expect(document.querySelector("#trace strong")).toBeNull();

  click("#reset");
  expect(text("#status")).toBe("Idle");
  expect(text("#trace")).toBe("");
  expect(text("#output")).toBe("");
});
