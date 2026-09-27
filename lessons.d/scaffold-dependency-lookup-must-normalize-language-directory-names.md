# Scaffold dependency lookup must normalize language directory names

The TypeScript scaffold generator normalized Ruby and Elixir package directories to
snake_case but left Lua in kebab-case. As a result, a valid Lua dependency such as
`der-tlv` was looked up at `lua/der-tlv` instead of the established `lua/der_tlv`
directory and the multi-language generation stopped partway through. Keep dependency
lookup and target-directory naming driven by the same language normalization table,
and cover every snake_case ecosystem with a unit test.
