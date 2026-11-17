---
description: "Project Prism typography, materials and component architecture"
applyTo: "src/desk/**,desk.html,public/appearance.js"
---

Read `docs/design-system/foundations.md` and `components.md`. Architectural
Spatial Glass is the locked direction: separated navigation, continuous content,
bounded optical controls, dense evidence. Outfit is local, with zero tracking.
Semantic tokens own type, spacing, radii, color, material and elevation. Repeated
items have at most 8px corners; no decorative cards around page sections.
Never blur every row or put transforms/backdrop filters on fixed-element ancestors.
Keep all six palette IDs and light/dark/system appearance. Opaque fallback first.
Route production glyphs through `src/desk/animated-icons.tsx` and its `icons.tsx`
facade. Use selected UseAnimations assets first; unmatched sport, provider and brand
artwork uses original shared recipes from `custom-icons.tsx`, retaining its meaning,
brand geometry and source attribution. Reduced-motion/error/suspended states use
same-source static SVGs, not unrelated Tabler fallbacks. Keep native fields and
platform navigation semantics. Prism owns control and surface motion; both icon
renderers share eligibility/timing and stay inside their existing slots.
Preserve labels, data, permissions and destructive guards.