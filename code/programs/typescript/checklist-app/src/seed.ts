/**
 * seed.ts — Pre-loaded example templates for the first-visit experience.
 *
 * Three templates demonstrate the three complexity levels:
 *
 *   1. Morning Routine — flat check list. No decisions. Shows the simplest
 *      case: just work through the items in order.
 *
 *   2. Deployment Runbook — mix of checks and one decision. The classic
 *      "did it work?" branch that every runbook has. Shows how a single
 *      decision changes what you do next.
 *
 *   3. Troubleshooting Guide — nested decisions. Two levels deep. Shows
 *      that decisions can appear inside other decision branches, and that
 *      the visible list expands progressively as you answer each question.
 *
 * These templates are registered directly into the AppState singleton at
 * app startup, before the first render.
 */

import { createTemplate } from "./state.js";
import type { AppState } from "./state.js";
import type { TemplateItem } from "./types.js";

export function seedTemplates(state: AppState): void {
  // ── 1. Morning Routine ───────────────────────────────────────────────────
  // A flat procedural checklist — the bread and butter of any checklist app.
  // Pilots call this a "do-list": items to be completed in sequence with no
  // conditional logic.

  const morningItems: TemplateItem[] = [
    { id: "m1", type: "check", label: "Drink a glass of water" },
    { id: "m2", type: "check", label: "15 minutes of stretching" },
    { id: "m3", type: "check", label: "Review today's priorities" },
    { id: "m4", type: "check", label: "Prepare breakfast" },
    { id: "m5", type: "check", label: "Check calendar for the day" },
    { id: "m6", type: "check", label: "Clear email inbox to zero" },
    { id: "m7", type: "check", label: "Set a timer for deep-work block" },
  ];

  createTemplate(
    state,
    "Morning Routine",
    "A flat daily routine — no decisions, just a procedural sequence to start the day right.",
    morningItems,
  );

  // ── 2. Deployment Runbook ────────────────────────────────────────────────
  // Mixes check items (steps to execute) with a decision (did it work?).
  // The yes-branch continues with monitoring; the no-branch triggers a
  // rollback. This is the canonical decision-tree checklist pattern.

  const deployItems: TemplateItem[] = [
    { id: "d1", type: "check", label: "Merge PR to main" },
    { id: "d2", type: "check", label: "Confirm CI pipeline is green" },
    { id: "d3", type: "check", label: "Run deployment script" },
    { id: "d4", type: "check", label: "Wait for pods to restart (watch kubectl)" },
    {
      id: "d5",
      type: "decision",
      label: "Did smoke tests pass?",
      yesBranch: [
        { id: "d6", type: "check", label: "Open dashboard and monitor error rate" },
        { id: "d7", type: "check", label: "Monitor for 10 minutes, no spike" },
        { id: "d8", type: "check", label: "Post success notice in #deployments" },
      ],
      noBranch: [
        { id: "d9", type: "check", label: "Run rollback script immediately" },
        { id: "d10", type: "check", label: "Verify rollback succeeded" },
        { id: "d11", type: "check", label: "Alert on-call engineer" },
        { id: "d12", type: "check", label: "File incident report" },
      ],
    },
  ];

  createTemplate(
    state,
    "Deployment Runbook",
    "Step-by-step deployment with a smoke-test decision. Success path monitors; failure path triggers rollback.",
    deployItems,
  );

  // ── 3. Troubleshooting Guide ─────────────────────────────────────────────
  // Two levels of nested decisions. The first question diagnoses whether the
  // service is running; if not, it asks whether a restart fixed it. This
  // pattern is common in IT runbooks and medical triage protocols.

  const troubleshootItems: TemplateItem[] = [
    { id: "t1", type: "check", label: "Open monitoring dashboard" },
    { id: "t2", type: "check", label: "Check for recent deployments in last 24h" },
    {
      id: "t3",
      type: "decision",
      label: "Is the service showing as healthy in the dashboard?",
      yesBranch: [
        { id: "t4", type: "check", label: "Check downstream services for errors" },
        { id: "t5", type: "check", label: "Review logs for the last 30 minutes" },
        {
          id: "t6",
          type: "decision",
          label: "Did you find the root cause in the logs?",
          yesBranch: [
            { id: "t7", type: "check", label: "Document the root cause" },
            { id: "t8", type: "check", label: "Open a ticket with steps to reproduce" },
          ],
          noBranch: [
            { id: "t9", type: "check", label: "Escalate to senior engineer with log excerpt" },
          ],
        },
      ],
      noBranch: [
        { id: "t10", type: "check", label: "Attempt service restart" },
        {
          id: "t11",
          type: "decision",
          label: "Did the restart resolve the issue?",
          yesBranch: [
            { id: "t12", type: "check", label: "Monitor for 5 minutes to confirm stability" },
            { id: "t13", type: "check", label: "Log the incident and resolution" },
          ],
          noBranch: [
            { id: "t14", type: "check", label: "Check disk space and memory usage" },
            { id: "t15", type: "check", label: "Page the on-call engineer immediately" },
          ],
        },
      ],
    },
  ];

  createTemplate(
    state,
    "Troubleshooting Guide",
    "Two-level nested decision tree for diagnosing service issues. Demonstrates how visible items expand progressively.",
    troubleshootItems,
  );
}
