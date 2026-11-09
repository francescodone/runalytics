---
description: "Use when working with Runalytics GitHub repository settings, GitHub Actions secrets, Supabase project settings, credentials, deployments, or Chrome DevTools MCP."
applyTo: "supabase/**,CREDENTIALS.md,docs/supabase-backend.md,.github/workflows/**"
---

# Hosted Services

- GitHub repository: https://github.com/francescodone/runalytics
- Repository settings: https://github.com/francescodone/runalytics/settings
- Actions secrets: https://github.com/francescodone/runalytics/settings/secrets/actions
- Supabase project dashboard: https://supabase.com/dashboard/project/zyqgoepypenqoxemammd
- Local setup and credential destinations: `CREDENTIALS.md` and
  `docs/supabase-backend.md`. The cloud integration is incomplete; check the
  current implementation before assuming a feature or secret exists.

When the user asks to inspect or operate these dashboards using Chrome MCP,
use the Chrome DevTools MCP tools. List open pages first, reuse the matching
tab when possible, and take a fresh accessibility snapshot before interacting.
Verify the repository/project and environment before making a change. If a
tool or login is unavailable, explain the blocker; do not claim an action ran.

Dashboard access is context, not blanket authorization. Inspecting settings is
read-only; create, update, delete, deploy, change permissions, or configure
secrets only when the user's current request authorizes that specific action.
Never overwrite an existing secret or change a production project as a side
effect of local testing. Local Supabase commands must explicitly target the
local stack; do not push migrations or deploy functions without authorization.

Never read, print, log, screenshot, commit, or send real credentials through
the assistant. Ask the user to enter actual secret values directly in the
appropriate dashboard or secure terminal prompt. Do not use placeholder values
as working credentials. Google OAuth secrets belong in Supabase Auth; Strava
and server-side encryption secrets belong in Supabase runtime secret storage
when implemented. GitHub Actions secrets are for workflows that need them,
not an automatic source of runtime environment variables. Confirm names and
destinations against current code before requesting or adding secrets.