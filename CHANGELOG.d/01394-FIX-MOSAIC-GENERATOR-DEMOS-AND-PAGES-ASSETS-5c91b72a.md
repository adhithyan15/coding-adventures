### Fix Mosaic project emitters and GitHub Pages demo assets

- Fixed `mosaic-compile` React `--emit-project` omission of `tsconfig.json` so generated projects can build with `npm run build`.
- Made emitted React projects relocatable by changing `/src/main.tsx` script tag to `./src/main.tsx` in `mosaic-emit-react`.
- Replaced illegal `--emit-project` inside XML/HTML comment banners (`<!-- ... -->`) with `in emit-project mode` across `mosaic-emit-react`, `mosaic-emit-html`, `mosaic-emit-webcomponent`, `mosaic-emit-flutter`, `mosaic-emit-qt`, `mosaic-emit-swiftui`, and `mosaic-package-artifact-builder`.
- Fixed root-absolute `/calculator-icon.svg` in Busicom calculator `index.html` to `./calculator-icon.svg` to prevent 404s on GitHub Pages deployment.
- Added Windows `.exe` executable detection in `code/programs/typescript/visicalc/scripts/build.sh`, and configured `base: "./"` and `./src/app/main.tsx` in `visicalc` for relocatable preview builds.
