---
category: BUILD files & dependency management
---

# Mass BUILD changes trigger a full rebuild

The build tool diffs file paths; touching every BUILD in one commit forces every package to rebuild and exposes pre-existing broken BUILDs. Only edit BUILDs your PR actually needs.
