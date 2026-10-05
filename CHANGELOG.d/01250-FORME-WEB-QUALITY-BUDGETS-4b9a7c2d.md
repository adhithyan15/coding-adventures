### Forme web quality is now a required release gate

- Audit the generated landing page, blog index, and progressively enhanced
  article with exact Lighthouse and Chrome-for-Testing pins before the required
  CI gate can pass.
- Enforce explicit desktop performance, accessibility, resource, and
  no-JavaScript fallback budgets while keeping fallback parsing and browser
  navigation offline and retaining only bounded atomic evidence.
- Build the quality lane from committed local-package locks with dependency
  lifecycle scripts disabled and reviewed package builds invoked explicitly.
- Install Chrome's hosted-runner runtime libraries through the pinned setup
  action while retaining the browser sandbox.
- Preserve a bounded printable Chrome-startup diagnostic when a hosted browser
  exits before exposing its debugging port.
- Document the cross-site quality command and close FM-B070 in the living Forme
  completion roadmap.
