---
category: Cryptography & security review
---

# Borrowed secret views must not claim their owners drop behavior

I described a borrowed `BaseSecrets` view as wipe-on-drop even though only its
caller-owned `BaseSecretStorage` backing has zeroizing drop behavior. Security
documentation must name the object that owns the drop implementation: say
"view into caller-owned wipe-on-drop storage" unless the view itself wipes its
backing bytes when dropped.
