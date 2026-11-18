---
description: "Project Prism responsive geometry and verification rules"
applyTo: "src/desk/**,tests/*prism*,tests/desk.spec.mjs"
---

Read `docs/codebase/CONVENTIONS.md` and `docs/codebase/TESTING.md`. At <=800px use
compact date selection, itinerary, bottom navigation and full-screen inspectors.
Above 800px keep a separate navigation rail and a continuous evidence workspace.
Use minmax(0,1fr), min-width:0 and wrapping, not page overflow clipping. Controls
must remain reachable at 320px, short heights, landscape and enlarged text.
Safe-area padding belongs to the viewport edge. Scroll dense tables locally with
labelled keyboard-focusable regions. Preserve focus outlines and hidden states.
Run Prism's width/theme matrix plus existing workflow tests; inspect screenshots.
Record exact verified widths/states and native/device limitations with the change
or release evidence. Update context docs when current contracts change; do not
accumulate historical test results there.