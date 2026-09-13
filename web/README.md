# Linger website

This branch contains a **project-page exploration**, requested on 13 September 2026: plain prose, restrained typography and captured terminal interactions. It is a preview awaiting selection. Production remains the Osaka Jade design on main.

## Active source

- `linger/pages/index.astro`: exploration homepage.
- `linger/pages/guide.astro`: setup, controls and current limits.
- `linger/styles/site.css`: shared project-page and guide styles.
- `linger/public/favicon.svg`: site icon.
- `astro.linger.config.mjs`: active Astro configuration.

`pnpm dev` starts the local site; `pnpm build` generates `dist/`. Only `/` and `/guide/` are published by this build. Production is https://lingerer.xyz on Vercel project `linger-website`. See [the release process](../docs/RELEASING.md) for PR previews and automatic production deployment from main.

The demo switches between SVG captures of the actual terminal cell buffer in `linger/public/terminal/`. They were captured on 13 September 2026 from the active development build using `../examples/repair.jsonl`, at 120 columns by 32 rows, with isolated temporary data and model keys removed. The selected command and its documentation connector come from the app itself. The page captions are authored explanations. The development UI is ahead of the tagged 0.1.0 download, which the page identifies. Refresh captures through the terminal when the UI changes; do not redraw fictional app behavior. Website interactions do not run terminal commands or make model requests. Check product claims against the current repository documentation when changing copy.

The palette follows `../docs/VISUAL-DESIGN.md`. Preserve readable input/output, keyboard controls, and the distinction between recorded evidence and interpretation. No eyebrow elements.

## Archived design exploration

All earlier variants, their index, extra styles, tour assets, capture scripts and the previous website notes are retained under the ignored local directory:

`../outputs/archive/website-designs-2026-09-13/`

This archive is not canonical, is not included in builds, and is not tracked or backed up by Git. It contains a README and a hash manifest. Consult it only when explicitly revisiting an older design; do not use it as guidance for routine website changes. There are no live `/variants/` routes or gallery links.

The inherited Zoetrope `src/`, `public/`, `wasm/` and `astro.config.mjs` remain upstream material and are excluded from the active site build. `build:upstream` is retained for upstream development only. Preserve upstream MIT attribution.

## Background studies

Opt-in preview query parameters `?background=phosphor`, `?background=fold` and `?background=etching` show image-generated art from `linger/public/backgrounds/`. They fade in once over 1.8 seconds at 12%, 10% and 9% opacity respectively. Reduced-motion preference removes the transition. The default page stays plain; these studies are awaiting selection. Original PNGs and full built-in image-generation prompts are retained locally in the root checkout at `outputs/background-studies-2026-09-13/`.
