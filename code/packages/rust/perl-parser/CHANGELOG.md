# Changelog

## Unreleased

- Require separate partial Perl 5.004_50 and 5.004_51 token/grammar pairs and apply
  existing leading-zero, carriage-return, unsupported-character, and
  250-digit decimal probes to it.

- Test the plain-decimal boundary for all versioned release pairs from 1.000
  through 5.003_03, including positive `0` and nonzero decimal forms and
  negative octal-shaped forms.

## 0.1.0

- Initial bounded Perl-language parser for LANG81.
