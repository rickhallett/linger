# Linger website

The canonical website is the **continuous journey**, selected for production on 13 September 2026: accelerating travel through 32 code panels, glyph particles assembling into an inspectable command, then a pale project page with captured terminal interactions and scrolling Phosphor.

## Active source

- `linger/pages/index.astro`: canonical continuous homepage.
- `linger/scripts/journey.ts` and `linger/styles/journey.css`: Three.js 0.186 / PixiJS 8.20 scroll sequence.
- `linger/components/JourneyProject.astro`: approved project copy and captured walkthrough.
- `linger/pages/guide.astro`: setup, controls and current limits.
- `linger/styles/site.css`: shared project-page and guide styles.
- `linger/public/favicon.svg`: site icon.
- `astro.linger.config.mjs`: active Astro configuration.

`pnpm dev` starts the local site; `pnpm build` generates `dist/`. Only `/` and `/guide/` are published by this build. Production is https://lingerer.xyz on Vercel project `linger-website`. See [the release process](../docs/RELEASING.md) for PR previews and automatic production deployment from main.

The demo switches between SVG captures of the actual terminal cell buffer in `linger/public/terminal/`. They were captured on 13 September 2026 from the active development build using `../examples/repair.jsonl`, at 120 columns by 32 rows, with isolated temporary data and model keys removed. The selected command and its documentation connector come from the app itself. The development UI is ahead of the tagged 0.1.0 download. Keep this capture provenance in these developer notes; the operator selected a homepage without demo disclaimers. Refresh captures through the terminal when the UI changes; do not redraw fictional app behavior. Website interactions do not run terminal commands or make model requests. Check product claims against the current repository documentation when changing copy.

The palette follows `../docs/VISUAL-DESIGN.md`. Preserve readable input/output, keyboard controls, and the distinction between recorded evidence and interpretation. No eyebrow elements.

## Archived design exploration

All earlier variants, their index, extra styles, tour assets, capture scripts and the previous website notes are retained under the ignored local directory:

`../outputs/archive/website-designs-2026-09-13/`

This archive is not canonical, is not included in builds, and is not tracked or backed up by Git. It contains a README and a hash manifest. Consult it only when explicitly revisiting an older design; do not use it as guidance for routine website changes. There are no live `/variants/` routes or gallery links.

The inherited Zoetrope `src/`, `public/`, `wasm/` and `astro.config.mjs` remain upstream material and are excluded from the active site build. `build:upstream` is retained for upstream development only. Preserve upstream MIT attribution.

## Background studies

Retained background study query parameters `?background=phosphor`, `?background=fold` and `?background=etching` show image-generated art from `linger/public/backgrounds/`. Phosphor is selected as the default background at 12% opacity, with a seamless 55-second upward scroll and a 1.8-second initial fade. Reduced-motion preference removes both animation and transition. Fold (10%) and etching (9%) remain available by query for comparison. Original PNGs and full built-in image-generation prompts are retained locally in the root checkout at `outputs/background-studies-2026-09-13/`.

## Arrival animation

The camera passes through 32 authored code panels with a curved scroll-to-distance mapping. Particles sampled from the panels form the command, with HTML buttons for inspecting its four parts. The green field opens onto the project page. Native scrolling reverses the sequence; the navigation skip link leads directly to the content. Reduced motion fixes the camera and switches assembled states. GPU startup failure preserves the skip link; without JavaScript, the project content is shown directly. Rendering pauses in hidden tabs and after the project content arrives. Renderer resolution is capped at 1.5x. Three.js uses WebGPU with its WebGL2 fallback. Commands are authored examples and are never executed.

Earlier `/spaces/` experiments remain in the separate local `website/graphics-studies` worktree. They are not part of production routes or navigation. The previous plain homepage is retained in Git history at `a9887d3`.
