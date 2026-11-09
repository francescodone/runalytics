import { t, text, date as fmtDateDMY, distance as fmtKm, timeRange as fmtTimeRange } from "./i18n";
import { useState } from "react";
import { IconCheck } from "@tabler/icons-react";
import { tauri, fmtPace, estimatedTimeRange } from "../lib/ipc";
import type { PlanProposal } from "../lib/types";
import { useCommand, useRemote } from "./data";
import { Button, Dialog, ErrorNotice, InfoDialog, Loading } from "./ui";
import { RaceEstimates } from "./race-estimates";
import { Presence } from "./interaction";

export function PlanAdjustments({ source }: { source: "pain" | "pacing" }) {
  const pending = useRemote(source === "pain" ? tauri.pendingProposal : tauri.pendingPacingProposal, [source]);
  const history = useRemote(() => tauri.proposals(20));
  const pacing = useRemote(() => source === "pacing" ? tauri.pacingReport() : Promise.resolve(null), [source]);
  const [selected, setSelected] = useState<PlanProposal | null>(null);
  return <section className="adjustment-evidence"><h3>{source === "pain" ? t("Recovery adjustments") : t("Pacing evidence")}</h3>
    <ErrorNotice error={pending.error ?? pacing.error ?? history.error} retry={pending.retry} />
    {pacing.data && <><p className="muted">{t("{{value0}} of {{value1}} consecutive quality sessions {{value2}}.", { value0: pacing.data.streak, value1: pacing.data.needed, value2: pacing.data.direction ? t("in the {{value0}} band", { value0: pacing.data.direction }) : "reviewed" })}</p><div className="data-table-scroll" tabIndex={0} role="region" aria-label={t("Quality session pacing evidence")}><table><caption className="sr-only">{t("Quality session pacing evidence")}</caption><thead><tr><th>{t("Session")}</th><th>{t("Target /km")}</th><th>{t("Held /km")}</th><th>{t("Result")}</th></tr></thead><tbody>{pacing.data.reviews.map(review => <tr key={review.workoutId}><th>{text(review.title)}<small className="table-date">{fmtDateDMY(review.date)}</small></th><td>{fmtPace(review.bandLoSKm)} - {fmtPace(review.bandHiSKm)}</td><td>{fmtPace(review.actualSKm)}</td><td>{review.execution === "in" ? t("On target") : review.execution}</td></tr>)}</tbody></table></div></>}
    {pending.loading ? <Loading label={t("Checking plan adjustments")} /> : pending.data ? <div className="proposal-prompt"><div><strong>{text(pending.data.summary)}</strong><p>{text(pending.data.reason)}</p></div><Button tone="primary" onClick={() => setSelected(pending.data!)}>{t("Review changes")}</Button></div> : !pending.error && <div className="adjustment-status" role="status"><IconCheck size={18} aria-hidden="true" /><span>{t("No pending adjustments")}</span></div>}
    {(history.data ?? []).some(proposal => proposal.source === source && proposal.status !== "pending") && <InfoDialog title={t("Adjustment history")}><ul className="plain-list">{history.data?.filter(proposal => proposal.source === source && proposal.status !== "pending").map(proposal => <li key={proposal.id}><Button onClick={() => setSelected(proposal)}>{text(proposal.summary)}</Button><span>{proposal.status} / {fmtDateDMY(proposal.createdAt)}</span></li>)}</ul></InfoDialog>}
    <Presence>{selected && <ProposalReview proposal={selected} onClose={() => setSelected(null)} />}</Presence>
  </section>;
}

function ProposalReview({ proposal, onClose }: { proposal: PlanProposal; onClose: () => void }) {
  const command = useCommand();
  const current = useRemote(tauri.currentPlan);
  const [reason, setReason] = useState("");
  const proposedPaceOffset = proposal.source === "pacing" ? proposal.planQualityOffsetSecKm ?? 0 : 0;
  return <Dialog title={t("Review plan changes")} wide onClose={onClose}><h3>{text(proposal.summary)}</h3><p>{text(proposal.reason)}</p>
    {proposal.source === "pacing" && <p className="context-note">{t("Quality pace offset: {{value0}} seconds/km from {{value1}}.", { value0: proposal.planQualityOffsetSecKm ?? 0, value1: proposal.planQualityOffsetFrom ? fmtDateDMY(proposal.planQualityOffsetFrom) : t("the next quality session") })}</p>}
    {proposal.source === "pacing" && proposal.status === "pending" && current.data?.[0] && <RaceEstimates plan={current.data[0]} proposedOffset={proposal.planQualityOffsetSecKm ?? undefined} />}
    <div className="data-table-scroll" tabIndex={0} role="region" aria-label={t("Proposed session changes")}><table><caption className="sr-only">{t("Proposed changes to planned sessions")}</caption><thead><tr><th>{t("Date")}</th><th>{t("Current session")}</th><th>{t("Proposed session")}</th></tr></thead><tbody>{proposal.edits.map(edit => <tr key={edit.workoutId}><td>{edit.date ? fmtDateDMY(edit.date) : t("Upcoming")}</td><td>{edit.oldTitle ?? text(edit.oldKind) ?? t("Original session")}<br />{edit.oldDistanceM != null ? fmtKm(edit.oldDistanceM) : ""}</td><td>{text(edit.title)}<br />{edit.distanceM != null ? fmtKm(edit.distanceM) : fmtTimeRange(estimatedTimeRange(edit)) ?? (edit.durationMin != null ? `${Math.round(edit.durationMin)} min` : t("Rest"))}{edit.paceLoSKm != null && edit.paceHiSKm != null ? ` / ${fmtPace(edit.paceLoSKm + proposedPaceOffset)} - ${fmtPace(edit.paceHiSKm + proposedPaceOffset)}/km` : ""}</td></tr>)}</tbody></table></div>
    {proposal.status === "pending" && <><p className="context-note">{t("Only these changes will be applied. Runalytics will update the plan and attempt to publish it to COROS.")}</p><label>{t("Reason for dismissing (optional)")}<input value={reason} onChange={event => setReason(event.target.value)} /></label></>}
    <ErrorNotice error={command.error} /><div className="form-actions"><Button onClick={onClose}>{t("Close")}</Button>{proposal.status === "pending" && <><Button busy={command.busy} onClick={async () => { if (await command.run(() => tauri.dismissProposal(proposal.id, reason || null), t("Proposed changes dismissed."))) onClose(); }}>{t("Dismiss changes")}</Button><Button tone="primary" busy={command.busy} onClick={async () => { if (await command.run(() => tauri.applyProposal("coros", proposal.id), t("Plan changes applied."))) onClose(); }}>{t("Apply changes")}</Button></>}</div>
  </Dialog>;
}