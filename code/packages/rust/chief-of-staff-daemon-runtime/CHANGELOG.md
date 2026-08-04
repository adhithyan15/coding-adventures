# Changelog

## 0.1.0

- Add a mandatory startup reconciliation tick before the WebSocket listener serves.
- Add non-zero periodic reconciliation through the daemon API's serialized control plane.
- Stop serving and surface a stable error when background convergence fails.
- Join the scheduler promptly during cooperative external shutdown.
