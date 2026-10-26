---
category: CI & GitHub Actions
---

# A repeated SDK download failure is bootstrap not product test evidence

TaskApp PR #18251 triggered the XAML typography job, but setup-dotnet failed
twice fetching SDK 9.0.320 with HTTP 409 before compiling or executing tests.
Do not classify that as a Qt regression or remove the native gate. Verify a
compatible SDK in official release metadata and verify archive availability,
then pin it and require the actual native job to pass. A successful HEAD
request alone proves availability, not build or runtime acceptance.
