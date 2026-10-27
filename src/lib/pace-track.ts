import { fmtPaceBand, fmtPaceHeld } from "./ipc";
import { EXECUTION_LABEL } from "./plan-bands";
import type { Execution } from "./types";

/// The band-against-marker bar, and the sentence a screen reader gets with it.
///
/// Two surfaces show a graded session — the Plan page's list of recent quality
/// sessions, and the sheet that opens one of them — and they must not disagree
/// about what the same bar means. Pace runs left = faster, so the bar reads
/// "too fast · in range · too slow" in the order you would say it, and the
/// label spells out both numbers rather than leaving the colour to carry it.
export function paceTrack(
  band: [number, number],
  actual: number | null,
  execution: Execution,
): string {
  const lo = Math.min(band[0], actual ?? band[0]) - 15;
  const hi = Math.max(band[1], actual ?? band[1]) + 15;
  const pos = (p: number) => ((p - lo) / (hi - lo)) * 100; // slower pace → right
  const bandLeft = pos(band[0]).toFixed(1);
  const bandWidth = (pos(band[1]) - pos(band[0])).toFixed(1);
  const marker =
    actual != null
      ? `<span class="pacing-marker" style="left:${pos(actual).toFixed(1)}%" title="held ${fmtPaceHeld(actual)}"></span>`
      : "";
  return `<span class="pacing-track exec-${execution}" role="img" aria-label="Planned ${fmtPaceBand(band[0], band[1])}, reps held ${actual != null ? fmtPaceHeld(actual) : "unknown"} — ${EXECUTION_LABEL[execution]}">
      <span class="pacing-band" style="left:${bandLeft}%;width:${bandWidth}%"></span>
      ${marker}
    </span>`;
}
