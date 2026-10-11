# Changelog

## [Unreleased]

### Security

- Replace Jest, ts-jest and the js-yaml 3 override with Vitest 4.1.11. The same numerical assertions now run without the Istanbul / argparse 1 / sprintf-js dependency path, clearing GHSA-hp3w-g68c-fv3c as well as the Jest 29 braces and handlebars advisories.
