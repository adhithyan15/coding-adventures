# windows-job-backend-task-xml

Pure Rust rendering of Windows Task Scheduler XML.

## What it does

- Converts a portable `JobSpec` into Task Scheduler XML
- Produces an `InstallPlan` that registers the task with `schtasks /Create`
- Maps concurrency policy into Task Scheduler's multiple-instance policy

## Supported triggers

- `once`
- `interval` with a minimum period of 60 seconds
- `daily`
- `weekly`
- `monthly`
- `at_login`
- `at_boot`

## Notes

- Environment injection is rejected in this pure XML backend until a shell-free
  Windows launcher exists.
- stdin payloads are rejected explicitly.

## Dependencies

- os-job-core

## Development

```bash
bash BUILD
```
