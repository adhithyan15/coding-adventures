- Matching HTML `template` end tags now thoroughly generate implied end tags
  and report the required parse error when a non-template element remains
  current. Directly current templates, implied descendants, foreign
  template-named elements, and synthetic template fragment contexts remain
  quiet.
