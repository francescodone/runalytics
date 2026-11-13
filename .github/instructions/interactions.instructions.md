---
description: "Project Prism shared interaction and overlay contracts"
applyTo: "src/desk/**/*.tsx,src/desk/interaction.css,src/desk/tokens.css"
---

Read `docs/design-system/motion.md`. All action families consume the same state
and motion tokens. Variants only change appearance, not press/release physics.
Use native disabled, checked, selected and current semantics. Keep pending and
result content in the original footprint. Retain failed drafts and visible errors.
Use shared Dialog, Inspector, Presence, SelectionGroup and NotificationViewport.
Maintain Escape, focus return, inert background, scroll ownership and Undo.
Ambient animation is limited to the existing bounded material-aura surfaces;
pause offscreen and in hidden documents, and cancel for accessibility preferences.
Reduced motion removes spatial movement, not feedback.
Validate real keyboard, pointer and touch behavior after shared primitive edits.