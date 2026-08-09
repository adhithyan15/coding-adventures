# chief-of-staff-daemon-secret-file

Race-resistant owner-only secret-file loading for D18 production adapters.

`read_owner_only_secret` accepts one bounded absolute path and an exact non-zero
byte length. It rejects linked or non-regular objects, foreign ownership, broad
access controls, missing or overlong content, and parent-directory link traversal.
The bounded result is always returned in a zeroizing allocation.

Unix walks from `/` through directory file descriptors, opens every component with
`O_NOFOLLOW`, and requires the final file to belong to the effective user with no
group or world access. Windows holds every ancestor without delete sharing, rejects
reparse points, and requires a protected DACL containing exactly one allow ACE for
the current token user.

The package only reads existing operator-owned files. It never creates, repairs,
rewrites, logs, or exposes secret content through its errors.

## Development

```bash
bash BUILD
```
