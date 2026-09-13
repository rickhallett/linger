# Linger visual language

Linger uses Osaka Jade from Omarchy Quattro, selected by the operator. Keep the instant terminal layout and make close reading distinct through the call selection rail, evidence tabs, clear keyboard focus and quiet timeline.

## Shared vocabulary

| Role | Colour | Osaka source role |
|---|---|---|
| Canvas | `#0C1512` | dark_background |
| Reading surface | `#111C18` | background |
| Focus / wordmark | `#509475` | accent |
| Selection fill | `#32473B` | selection |
| Selected text | `#F7E8B2` | bright_foreground |
| Body text | `#C1C497` | foreground |
| Secondary text | `#81B8A8` | dark_foreground |
| Quiet borders | `#53685B` | muted |
| Live / success | `#63B07A` | bright_green |
| Failure | `#FF5345` | red |
| Canvas pattern / minimap viewport | `#23372B` | lighter_background |

The selected call has a solid left rail and a filled row; the active evidence tab uses the same treatment. Square frames and a lowercase `linger /` wordmark give the views a consistent terminal presentation. Focused pane borders reinforce keyboard depth. Every status also carries a glyph or word; green alone must never mean an operation succeeded.

Terminal fonts remain the user's choice. Typography is expressed through weight, spacing and hierarchy, not bundled fonts. No decorative pre-headings. Preserve readable input/output and the evidence distinction as density increases.

`src/ui/theme.rs` owns the semantic palette, frame shape and selection style. Views consume those roles. Practising uses a yellow `◎` (`#E5C736`, Osaka bright_yellow) in the inspector, library and timeline. It remains distinct from live/success green and failure red.

## Source and adaptation

Colour values come from [Omarchy's Osaka Jade palette](https://github.com/omacom/omarchy/blob/692c02cad5c1ee90fe4188cc2be48e534cbe6e62/themes/osaka-jade/colors.toml), checked on the `quattro` branch on 13 September 2026. The [upstream MIT notice](licenses/OMARCHY-MIT.txt) is retained. This applies its colour vocabulary to Linger; it does not install Omarchy, alter the user's terminal settings or imply affiliation.

The graph and replay layout derive from Zoetrope with attribution. Linger's inspection controls and selection treatment belong to its own interaction design. Keep brand constants out of provider, state and replay code.

## Current limits

The direction targets dark terminals with true-colour support and respects NO_COLOR. It is not yet a light theme or a theme editor. Inspect at laptop dimensions with the fictional fixture; actual font rendering is controlled by the terminal.

Shortcut legends use brighter key glyphs with a 650 ms amber acknowledgement after activation, preserving layout width. Header/footer whitespace separates controls from evidence. Text entry does not trigger shortcut feedback. The inspector names the selected call alongside Preview/Reading focus, and keeps a selected call near the middle of longer call lists.
