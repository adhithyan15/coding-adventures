# smart-home-runtime-store

`smart-home-runtime-store` persists a versioned `SmartHomeRuntime` snapshot
through the repository-owned `StorageBackend` contract. It restores normalized
topology, state, event and command history, pairing sessions, desired state,
and opaque automation definitions after a process restart.

Live discovery workers and event subscriptions are deliberately process-local
and are rebuilt by their owners.
