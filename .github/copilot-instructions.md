# Runalytics

The production UI is React in `src/desk/`. Vite maps all eight HTML entries to
`desk.html`. Legacy controllers and `__*` harnesses are regression fixtures,
not production presentation. Preserve entry URLs, IPC contracts, provider
permissions, draft retention, confirmations and offline behavior.

Project Prism is the visual authority. Read `docs/design-system/foundations.md`,
`components.md`, `motion.md` and `migration-status.md` before UI work. Reuse
`tokens.css`, `ui.tsx`, `interaction.tsx` and `overlays.tsx`. Do not invent
page-specific interaction physics or revive retired visual specifications.

Build with `npm run build`; test with `npm run test:ui`. Use the existing Vite
preview task on port 1500; avoid `npm run dev` because its predev kills listeners.
Browser tests use synthetic IPC; never describe these as native integration tests.
Keep backend training algorithms and real credentials outside presentation work.

For GitHub repository settings or the Runalytics Supabase dashboard, read
`.github/instructions/hosted-services.instructions.md`.