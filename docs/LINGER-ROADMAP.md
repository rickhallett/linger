# Linger roadmap

The product combines live agent observation, replay, command explanation and a personal library of recurring patterns. The aim is to understand what runs, well enough to explain it. It is not a practice curriculum.

## First experience

Implemented in the initial local build:

- Keyboard drill-down into exact recorded input and output.
- Stable selection and scrolling while events arrive.
- Event stepping, prompt stepping and live/replay navigation.
- Bounded deterministic command reference notes.
- Explicit asynchronous Mercury interpretation tied to selected evidence.
- Fictional demo and regression coverage for evidence/time/navigation.

The operator completed a visual pass of the first build. A live Mercury 2.5 request through OpenRouter has been verified with the fictional Python failure; navigation remained responsive. The next visual pass introduces Linger's own theme (see [visual design](VISUAL-DESIGN.md)). Long-result responsiveness and sustained real-session use still need measured profiling beyond basic regression tests.

## Personal attention

- Conservative command/usage/composition grouping, retaining original occurrences.
- Session and cross-session frequency distributions; separate raw occurrences from distinct-session spread.
- User-controlled Unmarked, Want to understand, Practising and Learned states.
- Store personal learning state separately from rebuildable indexes.
- Consistent non-colour as well as colour highlighting for Practising patterns across views and time.
- Hide Learned patterns from learning projections without removing execution context.

## Richer explanation and replay

- Syntax-span selection and documentation matched to command/platform/version.
- Selected output ranges and interpretation follow-ups.
- Episode selection, loops, pacing around Practising patterns and cross-session comparisons.
- Editable workflow-stage annotations with their underlying calls visible.
- Treat concurrent agent activity honestly; temporal proximity is not causation.

## Explicitly deferred

Formal exercises, mastery scoring, automatic learning-state transitions, agent control, direct stdout interception, browser parity and compatibility with the upstream Zoetrope application. Scrubbing is deferred in favour of the useful local inspector and explicit selected-call model path; users need to know when they send content externally.
