# Design tokens

One JSON file feeds every platform, so desktop and Android always share the same colors, type, spacing and motion.

```
design/tokens/
├── tokens.json          ← edit this (the only source of truth)
├── build.mjs            ← generates the files in build/
├── check-contrast.mjs   ← checks every text/UI color pair against WCAG AA
├── preview.html         ← visual preview of every token and spring
└── build/               ← generated, do not edit
    ├── tokens.css       desktop / prototype (CSS variables + type classes)
    ├── tokens.js        desktop JS (springs in Motion library format)
    ├── tokens.d.ts      types for tokens.js
    └── NhakoTokens.kt   Android Jetpack Compose
```

## Commands

Node 18 or newer, no dependencies.

```bash
node design/tokens/build.mjs           # regenerate build/
node design/tokens/check-contrast.mjs  # exits 1 if any pair fails
```

## Preview

`preview.html` loads ES modules, so it needs a local server:

```bash
cd design/tokens && python3 -m http.server 8765
# open http://localhost:8765/preview.html
```

## Changing a value

1. Edit `tokens.json`.
2. Run both commands above.
3. If a contrast pair fails, adjust the color until it passes. Don't remove the pair.
4. Update `docs/design/` if the meaning of a token changed.
