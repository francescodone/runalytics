import { useState } from "react";
import { tauri, fmtDateDMY, fmtKm, fmtPace, estimatedTimeRange, fmtTimeRange } from "../lib/ipc";
import type { PlanProposal } from "../lib/types";
import { useCommand, useRemote } from "./data";
import { Button, Dialog, ErrorNotice, Loading } from "./ui";
import { RaceEstimates } from "./race-estimates";
import { Presence } from "./interaction";

export function PlanAdjustments({ source }: { source: "pain" | "pacing" }) {
  const pending = useRemote(source === "pain" ? tauri.pendingProposal : tauri.pendingPacingProposal, [source]);
  const history = useRemote(() => tauri.proposals(20));
  const pacing = useRemote(() => source === "pacing" ? tauri.pacingReport() : Promise.resolve(null), [source]);
  const [selected, setSelected] = useState<PlanProposal | null>(null);
  return <section className="adjustment-evidence"><h3>{source === "pain" ? "Recovery adjustments" : "Pacing evidence"}</h3>
    <ErrorNotice error={pending.error ?? pacing.error ?? history.error} retry={pending.retry} />
    {pacing.data && <><p className="muted">{pacing.data.streak} of {pacing.data.needed} consecutive quality sessions {pacing.data.direction ? `in the ${pacing.data.direction} band` : "reviewed"}.</p><div className="data-table-scroll" tabIndex={0} role="region" aria-label="Quality session pacing evidence"><table><caption className="sr-only">Quality session pacing evidence</caption><thead><tr><th>Session</th><th>Target /km</th><th>Held /km</th><th>Result</th></tr></thead><tbody>{pacing.data.reviews.map(review => <tr key={review.workoutId}><th>{review.title}<small className="table-date">{fmtDateDMY(review.date)}</small></th><td>{fmtPace(review.bandLoSKm)} - {fmtPace(review.bandHiSKm)}</td><td>{fmtPace(review.actualSKm)}</td><td>{review.execution === "in" ? "On target" : review.execution}</td></tr>)}</tbody></table></div></>}
    {pending.loading ? <Loading label="Checking plan adjustments" /> : pending.data ? <div className="proposal-prompt"><div><strong>{pending.data.summary}</strong><p>{pending.data.reason}</p></div><Button tone="primary" onClick={() => setSelected(pending.data!)}>Review changes</Button></div> : <p className="muted">No changes awaiting your review.</p>}
    {(history.data ?? []).some(proposal => proposal.source === source && proposal.status !== "pending") && <details><summary>Adjustment history</summary><ul className="plain-list">{history.data?.filter(proposal => proposal.source === source && proposal.status !== "pending").map(proposal => <li key={proposal.id}><button className="text-button" onClick={() => setSelected(proposal)}>{proposal.summary}</button><span>{proposal.status} / {fmtDateDMY(proposal.createdAt)}</span></li>)}</ul></details>}
    <Presence>{selected && <ProposalReview proposal={selected} onClose={() => setSelected(null)} />}</Presence>
  </section>;
}

function ProposalReview({ proposal, onClose }: { proposal: PlanProposal; onClose: () => void }) {
  const command = useCommand();
  const current = useRemote(tauri.currentPlan);
  const [reason, setReason] = useState("");
  const proposedPaceOffset = proposal.source === "pacing" ? proposal.planQualityOffsetSecKm ?? 0 : 0;
  return <Dialog title="Review plan changes" wide onClose={onClose}><h3>{proposal.summary}</h3><p>{proposal.reason}</p>
    {proposal.source === "pacing" && <p className="context-note">Quality pace offset: {proposal.planQualityOffsetSecKm ?? 0} seconds/km from {proposal.planQualityOffsetFrom ? fmtDateDMY(proposal.planQualityOffsetFrom) : "the next quality session"}.</p>}
    {proposal.source === "pacing" && proposal.status === "pending" && current.data?.[0] && <RaceEstimates plan={current.data[0]} proposedOffset={proposal.planQualityOffsetSecKm ?? undefined} />}
    <div className="data-table-scroll" tabIndex={0} role="region" aria-label="Proposed session changes"><table><caption className="sr-only">Proposed changes to planned sessions</caption><thead><tr><th>Date</th><th>Current session</th><th>Proposed session</th></tr></thead><tbody>{proposal.edits.map(edit => <tr key={edit.workoutId}><td>{edit.date ? fmtDateDMY(edit.date) : "Upcoming"}</td><td>{edit.oldTitle ?? edit.oldKind ?? "Original session"}<br />{edit.oldDistanceM != null ? fmtKm(edit.oldDistanceM) : ""}</td><td>{edit.title}<br />{edit.distanceM != null ? fmtKm(edit.distanceM) : fmtTimeRange(estimatedTimeRange(edit)) ?? (edit.durationMin != null ? `${Math.round(edit.durationMin)} min` : "Rest")}{edit.paceLoSKm != null && edit.paceHiSKm != null ? ` / ${fmtPace(edit.paceLoSKm + proposedPaceOffset)} - ${fmtPace(edit.paceHiSKm + proposedPaceOffset)}/km` : ""}</td></tr>)}</tbody></table></div>
    {proposal.status === "pending" && <><p className="context-note">Only these changes will be applied. Runalytics will update the plan and attempt to publish it to COROS.</p><label>Reason for dismissing (optional)<input value={reason} onChange={event => setReason(event.target.value)} /></label></>}
    <ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={onClose}>Close</Button>{proposal.status === "pending" && <><Button busy={command.busy} onClick={async () => { if (await command.run(() => tauri.dismissProposal(proposal.id, reason || null), "Proposed changes dismissed.")) onClose(); }}>Dismiss changes</Button><Button tone="primary" busy={command.busy} onClick={async () => { if (await command.run(() => tauri.applyProposal("coros", proposal.id), "Plan changes applied.")) onClose(); }}>Apply changes</Button></>}</div>
  </Dialog>;
}