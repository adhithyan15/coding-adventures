### Note on jsdom
`jsdom` is a devDependency for exactly two tests: the SVG serialiser's escaping is
checked by handing its output to a real parser and asserting a hostile caption
cannot break out of an attribute or smuggle in a `<script>`. A string comparison
would pass on markup no browser accepts, which is the bug those tests exist to
catch — so the environment moved with them rather than the tests being weakened
to fit a Node-only config.

845 tests pass in the package; the app's 725 tests, typecheck, build and
`check:bundle` all pass unchanged.
