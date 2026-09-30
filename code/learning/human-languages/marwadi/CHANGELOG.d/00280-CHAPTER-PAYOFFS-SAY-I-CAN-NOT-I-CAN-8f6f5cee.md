## Chapter payoffs say "I can", not "i can"

The payoff line under each chapter's goal lowercased the goal's first letter,
so 107 chapters printed "Complete the last lesson of chapter N: i can say …".
Every one now keeps the capital: "…: I can say …", and a new test
(`payoff-summary-case.test.ts`) fails if the lowercase pronoun comes back. Only
the payoff summary changed; no lesson, word or atom moved.
