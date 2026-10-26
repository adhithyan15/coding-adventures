## Security — patch Electron certificate handling (#18270)

- Raise the Electron dependency floor beyond the vulnerable range in
  GHSA-jr5w-w6x2-84p9 and refresh the lockfile to Electron 43.8.3.
- Add a regression that rejects a vulnerable declared floor or locked runtime.
