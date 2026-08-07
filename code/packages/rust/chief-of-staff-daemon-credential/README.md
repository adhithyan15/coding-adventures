# chief-of-staff-daemon-credential

Race-resistant owner-only local credential persistence for the D18 Chief daemon.

`load_or_create_credential` accepts one bounded absolute path whose parent already
exists. It loads a canonical 64-byte lowercase-hex credential or claims the absent
file name without ever truncating or replacing an existing object. Returned secret
text is wiped on drop.

The Unix implementation walks from `/` with directory file descriptors and
`openat(O_NOFOLLOW)`. New content is written and synchronized at mode `000`, then
published as `0600`; existing files must be regular, owned by the effective user,
and grant no group/world access.

The Windows implementation holds every ancestor directory without delete sharing,
rejects reparse points, and supplies an explicit protected security descriptor to
`CreateFileW(CREATE_NEW)`. The descriptor contains exactly one allow ACE for the
current token user. Existing files must have the same owner and protected one-ACE
DACL; inherited directory permissions are never trusted.

## Dependencies

- chief-of-staff-daemon-policy
- coding-adventures-zeroize

## Development

```bash
# Run tests
bash BUILD
```
