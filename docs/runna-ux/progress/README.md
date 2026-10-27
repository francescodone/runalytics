# Progress-tab and activity-review captures — evidence folder

**Status: images NOT archived.** The five captures for
[`../RUNNA-PROGRESS-AND-REVIEW.md`](../RUNNA-PROGRESS-AND-REVIEW.md) were supplied
inline in the conversation, not as files. Recovery was attempted and failed:

- `find` over `~/Downloads`, `~/Desktop`, `/tmp`, `/var/folders` → no matching PNG/JPEG
  (only two unrelated Oct-26 Desktop crops, 1534×295 and 840×486)
- transcript JSONL scanned for base64 / `data:image` payloads → **0 hits**
  (the transcript stores text, not attachment bytes)
- VS Code `workspaceStorage` / `globalStorage/GitHub.copilot-chat` searched for images
  newer than 2026-10-26, and for any `*attach*` directory → nothing

So every figure in the analysis is transcribed from the inline images and is
**reproducible only from the transcription**, not from a repo artefact. That is stated
in §1 and §12 of the document.

## To archive them

Save the five screenshots anywhere (e.g. Desktop), then drop them here with these names:

| file | content | status bar |
| --- | --- | --- |
| `01-performance-all-time.png` | Progress → Performance, All-Time Stats card | 07:51, airplane, 97 % |
| `02-performance-week-prs.png` | Performance scrolled: weekly chart + Personal Records | 07:51, airplane, 98 % |
| `03-workouts-october.png` | Progress → Workouts: filters, October header, activity cards | 07:51, airplane, 97 % |
| `04-activity-laps-splits.png` | Tempo 3-2-1 detail: laps bar chart + splits table | 23:06, cellular, 64 % |
| `05-activity-hero.png` | Tempo 3-2-1 detail: map, metrics, description | 23:06, cellular, 64 % |

The status bar is the reliable way to tell the two capture sessions apart.
