### Forme web quality is now a required release gate

- Audit the generated landing page, blog index, and progressively enhanced
  article with exact Lighthouse and Chrome-for-Testing pins before the required
  CI gate can pass.
- Enforce explicit desktop performance, accessibility, resource, and
  no-JavaScript fallback budgets while keeping fallback parsing and browser
  navigation offline and retaining only bounded atomic evidence.
- Document the cross-site quality command and close FM-B070 in the living Forme
  completion roadmap.
