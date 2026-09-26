- An accepted `</body>` token now enters the after-body insertion mode even
  when `html`, `head`, and `body` were all implied, closing 5 previously silent
  malformed corpus cases for following head-only and frameset start tags.
