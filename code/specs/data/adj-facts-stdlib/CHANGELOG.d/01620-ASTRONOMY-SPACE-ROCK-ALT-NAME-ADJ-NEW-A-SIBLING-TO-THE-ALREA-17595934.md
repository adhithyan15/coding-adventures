- `astronomy/space-rock-alt-name.adj` (new) — a sibling to the already-shipped `space-rock-stage.adj`
  (`space_rock_stage(stage, description)`, the same rocky object's name at three stages of its
  journey — meteoroid → still_a_rock_in_space, meteor →
  called_a_fireball_or_shooting_star_when_it_burns_up_in_the_atmosphere, meteorite →
  survives_the_atmosphere_and_hits_the_ground). That table's own header already quotes, verbatim, the
  meteor sentence naming TWO everyday alternate terms in one continuous clause — a fact the
  single-description-atom schema had no room to decode as two separate, independently queryable names.
  New `space_rock_alt_name(stage, alt_name)` table: meteor → fireball, meteor → shooting_star. Honestly
  narrower than its parent: honest abstention on meteoroid and meteorite, whose cited spans name only
  the one primary term each. New e2e test file `facts_spacerockaltname_e2e.rs` (3 tests: forward
  recall returning both alternate names with citation, backward recall of the fireball-named stage,
  honest abstention on meteoroid). No manifest objective, matching `space-rock-stage.adj`'s own
  precedent. Third and final slice from the astronomy/ domain sweep — closes out that sweep entirely.
