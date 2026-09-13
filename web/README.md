# Linger website

The canonical design is **Osaka Jade**, based on the former Orange variant and retaining its copy and interactive inspector. The operator selected it on 13 September 2026.

## Active source

- `linger/pages/index.astro`: canonical homepage.
- `linger/pages/guide.astro`: setup, controls and current limits.
- `linger/styles/site.css`: shared Osaka Jade website styles.
- `linger/public/favicon.svg`: site icon.
- `astro.linger.config.mjs`: active Astro configuration.

`pnpm dev` starts the local site; `pnpm build` generates `dist/`. Only `/` and `/guide/` are published by this build. Production is https://lingerer.xyz on Vercel project `linger-website`. See [the release process](../docs/RELEASING.md) for independent website tags and environments.

The inspector uses `../examples/repair.jsonl` as a fictional fixture at build time. Its reference notes are authored demonstration content. Website interactions do not run terminal commands or make model requests. Check product claims against the current repository documentation when changing copy.

The palette follows `../docs/VISUAL-DESIGN.md`. Preserve readable input/output, keyboard controls, and the distinction between recorded evidence and interpretation. No eyebrow elements.

## Archived design exploration

All earlier variants, their index, extra styles, tour assets, capture scripts and the previous website notes are retained under the ignored local directory:

`../outputs/archive/website-designs-2026-09-13/`

This archive is not canonical, is not included in builds, and is not tracked or backed up by Git. It contains a README and a hash manifest. Consult it only when explicitly revisiting an older design; do not use it as guidance for routine website changes. There are no live `/variants/` routes or gallery links.

The inherited Zoetrope `src/`, `public/`, `wasm/` and `astro.config.mjs` remain upstream material and are excluded from the active site build. `build:upstream` is retained for upstream development only. Preserve upstream MIT attribution.
