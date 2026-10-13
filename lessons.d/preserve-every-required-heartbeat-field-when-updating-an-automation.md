---
category: Repo policy / workflow reminders
---

# Preserve every required heartbeat field when updating an automation

Two automation update attempts sent only the id, mode, and new prompt. The
first omitted the heartbeat discriminator; the second omitted required existing
fields such as name, recurrence rule, and status. Both were rejected. Before
updating an existing heartbeat, read its `automation.toml`, preserve the full
current definition (including notification policy and target task), and change
only the intended prompt or schedule field in one complete update call.
