- A `template` end tag with no authored open HTML template now reports the
  required parse error and remains ignored. Matching HTML templates, foreign
  template-named elements, and synthetic template fragment contexts retain
  their distinct closure and diagnostic behavior.
