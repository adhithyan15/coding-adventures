# Downloaded web release acceptance

The release workflow uploads these pinned test tools separately from the app.
Its verification job has no repository checkout: it extracts the downloaded ZIP,
checks archive and file hashes, serves that directory, and runs these Chromium
checks against the production JavaScript and bundled Rust WASM.

The two theme cases edit a formula, verify dependent totals, save bytes to disk,
close the page, and reopen those bytes in a fresh Rust application via Open.
They also exercise rapid keyboard navigation and the named inline editor.
Only browser file-picker handles are simulated. The application file executor,
serializer, runtime and controls are real. This does not certify OS dialogs,
screen readers, touch devices or native platform bundles.

To use a separately served distribution: `npm ci`,
`npx playwright install chromium`, then `npm test`. Set `VISICALC_URL` to change
the default localhost port 8080. Failure traces, screenshots and the saved test
workbook go under `test-results/`; CI retains these as browser evidence.

Before publishing a web prerelease, download its GitHub Release assets, verify
SHA256SUMS, extract and launch according to README.txt. Keep publication scoped
to the tested web preview. Native artifacts and full acceptance remain #14282.

Local evidence for this change: downloaded CI archive from run 35501818582,
verified the ZIP checksum and every release.json file hash, served the extracted
app outside the checkout, and entered =2+3 through the in-app browser. A1 displayed
5 and E1 recalculated to 28. This is CI-artifact evidence; it is not yet evidence
of a published GitHub Release. The new persistence gate runs in GitHub CI.
