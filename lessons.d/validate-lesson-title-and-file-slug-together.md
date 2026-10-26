# Validate lesson title and file slug together

Every `lessons.d` filename must equal the validator's slug of its level-one
heading. A clear title can still fail if articles or expanded wording appear in
only one place. Run `code/scripts/lessons.py validate` after adding a shard and
change the title and filename as one unit.
