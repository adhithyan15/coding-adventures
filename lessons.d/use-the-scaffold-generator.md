---
category: BUILD files & dependency management
---

# Use the scaffold generator

(`code/programs/go/scaffold-generator/`) for every new package. It produces correct BUILD/BUILD_windows, metadata, leaf-to-root install order, language-specific dir naming (Ruby/Elixir/Lua use snake_case), and includes README/CHANGELOG. If output is wrong, fix the generator first.
