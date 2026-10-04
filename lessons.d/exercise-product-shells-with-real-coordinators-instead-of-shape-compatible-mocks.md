---
category: Testing & coverage
---

# Exercise product shells with real coordinators instead of shape-compatible mocks

A shell test can pass with a mock that returns the right TypeScript shape while
still disagreeing with the real coordinator's runtime contract. The Forme
authoring shell originally treated publication identities like hexadecimal
digests even though the publication coordinator emits canonical padded base64.
Keep focused hostile mocks for boundary cases, but add at least one integration
test through the real coordinator and assert the durable product result. When a
DOM test runner and a Node-loaded coordinator use different intrinsic realms,
bridge only the realm-specific primitive in the test and keep its contract
covered separately rather than replacing the real coordinator with another
shape-compatible mock.
