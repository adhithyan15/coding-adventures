### Changed — Sanskrit A1 aggregate coverage has one owner

- Keep cumulative Sanskrit A1 totals and category totals in the dedicated
  census test while feature shards continue to pin only their own inventory
  probes and coverage points through a point-only helper. A syntax-aware
  capability regression prevents future coverage increments from fanning out
  across historical feature owners.
