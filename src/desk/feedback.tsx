import { useId, useState } from "react";
import { IconArrowDown, IconArrowUp, IconCheck, IconX } from "@tabler/icons-react";
import { fmtPace, tauri } from "../lib/ipc";
import type { DifficultyDetails, FeedbackContext, FeedbackSession } from "../lib/types";
import { sessionTitle, shortDate, useCommand, useRemote, useWorkspace } from "./data";
import { distance, t, text } from "./i18n";
import { Presence } from "./interaction";
import { Button, Dialog, ErrorNotice, Loading } from "./ui";

const labels = ["Much too easy", "Slightly easy", "About right", "Slightly hard", "Much too hard"] as const;

function Rating({ context, session }: { context: FeedbackContext; session: FeedbackSession }) {
  const id = useId();
  const command = useCommand();
  const { notify } = useWorkspace();
  const [rating, setRating] = useState<number | null>(session.rating);
  const [result, setResult] = useState("");
  const submit = (value: number | null, dismiss = false) => command.run(async () => {
    const outcome = await tauri.saveSessionFeedback(context.planId, context.revision, session.workoutId, value, dismiss);
    setResult(dismiss ? t("Feedback dismissed.") : text(outcome.reason));
    if (outcome.changed > 0) notify(text(outcome.reason), { label: t("Undo"), run: () => tauri.undoPlanDifficulty(context.planId) });
  });
  return <section className="session-feedback" aria-labelledby={`${id}-heading`}>
    <h3 id={`${id}-heading`}>{t("How was this session?")}</h3>
    <form onSubmit={event => { event.preventDefault(); if (rating !== null) void submit(rating); }}>
      <fieldset disabled={command.busy}><legend>{t("Compared with the prescription (optional)")}</legend>
        <div className="feedback-scale">{labels.map((label, index) => <label key={label}>
          <input type="radio" name={`${id}-rating`} value={index + 1} checked={rating === index + 1} onChange={() => setRating(index + 1)} />
          <strong>{index + 1}</strong><span>{t(label)}</span>
        </label>)}</div>
      </fieldset>
      <div className="feedback-actions"><Button type="submit" busy={command.busy} disabled={rating === null || rating === session.rating && session.submitted}><IconCheck size={18} />{t("Save feedback")}</Button>
        {!session.submitted && <Button type="button" disabled={command.busy} onClick={() => void submit(null, true)}><IconX size={18} />{t("Not now")}</Button>}
        {session.rating !== null && <Button type="button" disabled={command.busy} onClick={async () => { if (await submit(null)) setRating(null); }}>{t("Clear rating")}</Button>}
      </div>
    </form>
    <ErrorNotice error={command.error} />
    {result && <p role="status">{result}</p>}
  </section>;
}

export function SessionFeedback({ workoutId, activityId }: { workoutId?: string | null; activityId?: number }) {
  const feedback = useRemote(tauri.sessionFeedback);
  const context = feedback.data;
  const session = context?.sessions.find(session => workoutId ? session.workoutId === workoutId : session.activityId === activityId);
  return context && session ? <Rating key={session.workoutId} context={context} session={session} /> : null;
}

export function FeedbackReminder() {
  const feedback = useRemote(tauri.sessionFeedback);
  const [open, setOpen] = useState<string | null>(null);
  const context = feedback.data;
  const sessions = context?.sessions.filter(session => session.pending) ?? [];
  if (!context || !sessions.length) return null;
  return <section className="feedback-reminder" aria-label={t("Session feedback")}>
    <h3>{t("Session feedback")}</h3>
    {sessions.map(session => <div key={session.workoutId}>
      <Button aria-expanded={open === session.workoutId} onClick={() => setOpen(current => current === session.workoutId ? null : session.workoutId)}>{sessionTitle(session.title)} · {shortDate(session.date)}</Button>
      {open === session.workoutId && <Rating context={context} session={session} />}
    </div>)}
  </section>;
}

export function EditPlan({ planId }: { planId: string }) {
  const feedback = useRemote(tauri.sessionFeedback);
  const { notify } = useWorkspace();
  const [result, setResult] = useState("");
  const [direction, setDirection] = useState<"easier" | "harder" | null>(null);
  const context = feedback.data;
  return <section className="edit-plan" aria-label={t("Edit plan")}><h3>{t("Edit plan")}</h3>
    <div className="feedback-actions"><Button disabled={!context || context.planId !== planId} onClick={() => setDirection("easier")}><IconArrowDown size={18} />{t("Make plan slightly easier")}</Button>
      <Button disabled={!context || context.planId !== planId} onClick={() => setDirection("harder")}><IconArrowUp size={18} />{t("Make plan slightly harder")}</Button></div>
    <ErrorNotice error={feedback.error} retry={feedback.retry} />
    {result && <p role="status">{result}</p>}
    <Presence>{direction && <DifficultyReview planId={planId} direction={direction} onClose={() => setDirection(null)} onApplied={reason => {
      setResult(text(reason));
      notify(text(reason), { label: t("Undo"), run: () => tauri.undoPlanDifficulty(planId) });
    }} />}</Presence>
  </section>;
}

function Prescription({ details, previous }: { details: DifficultyDetails; previous?: DifficultyDetails }) {
  return <div>{details.steps.map((step, index) => {
    const original = previous?.steps.find((candidate, candidateIndex) => candidateIndex === index && candidate.label === step.label && candidate.openEnded === step.openEnded)
      ?? previous?.steps.find(candidate => candidate.label === step.label && candidate.openEnded === step.openEnded);
    const changed = (value: boolean, content: string) => previous && value ? <strong>{content}</strong> : content;
    const pace = step.paceLoSKm != null ? `${fmtPace(step.paceLoSKm)}${!step.openEnded && step.paceHiSKm != null ? ` - ${fmtPace(step.paceHiSKm)}` : "+"} /km` : "-";
    const recoveryPace = step.recoveryPaceLoSKm != null ? `${fmtPace(step.recoveryPaceLoSKm)}${step.recoveryPaceHiSKm != null ? ` - ${fmtPace(step.recoveryPaceHiSKm)}` : "+"} /km` : "-";
    return <p key={index}>
      {text(step.label)}: {step.repeats > 1 || original && original.repeats !== step.repeats ? <>{changed(!original || original.repeats !== step.repeats, `${step.repeats} ×`)} </> : null}{changed(!original || original.distanceM !== step.distanceM, distance(step.distanceM))}
      {(step.paceLoSKm != null || original?.paceLoSKm != null) && <> · {changed(!original || original.paceLoSKm !== step.paceLoSKm || original.paceHiSKm !== step.paceHiSKm || original.openEnded !== step.openEnded, pace)}</>}
      {(step.recoveryM > 0 || (original?.recoveryM ?? 0) > 0) && <> · {t("Recovery")}: {changed(!original || original.recoveryM !== step.recoveryM, distance(step.recoveryM))}{(step.recoveryPaceLoSKm != null || original?.recoveryPaceLoSKm != null) && <> · {changed(!original || original.recoveryPaceLoSKm !== step.recoveryPaceLoSKm || original.recoveryPaceHiSKm !== step.recoveryPaceHiSKm, recoveryPace)}</>}</>}
    </p>;
  })}</div>;
}

function DifficultyReview({ planId, direction, onClose, onApplied }: { planId: string; direction: "easier" | "harder"; onClose: () => void; onApplied: (reason: string) => void }) {
  const command = useCommand();
  const proposal = useRemote(async () => {
    const latest = await tauri.sessionFeedback();
    if (!latest || latest.planId !== planId) throw new Error(t("The plan changed. Refresh before trying again."));
    return tauri.previewPlanDifficulty(planId, latest.revision, direction);
  }, [planId, direction]);
  const preview = proposal.data;
  return <Dialog title={t("Review plan changes")} wide onClose={() => { if (!command.busy) onClose(); }}>
    <h3>{t(direction === "easier" ? "Make plan slightly easier" : "Make plan slightly harder")}</h3>
    {proposal.loading && <Loading />}
    {preview && <><p>{text(preview.outcome.reason)}</p>
      {preview.changes.length > 0 && <div className="data-table-scroll" tabIndex={0} role="region" aria-label={t("Proposed session changes")}><table><thead><tr><th>{t("Date")}</th><th>{t("Current session")}</th><th>{t("Proposed session")}</th></tr></thead><tbody>{preview.changes.map(change => <tr key={change.workoutId}><th>{shortDate(change.date)}<small className="table-date">{sessionTitle(change.title)}</small></th><td><Prescription details={change.before} /></td><td><Prescription details={change.after} previous={change.before} /></td></tr>)}</tbody></table></div>}</>}
    <ErrorNotice error={command.error ?? proposal.error} retry={proposal.error ? proposal.retry : undefined} />
    <div className="form-actions"><Button disabled={command.busy} onClick={onClose}>{t("Dismiss changes")}</Button>{preview && preview.changes.length > 0 && <Button tone="primary" busy={command.busy} onClick={async () => {
      if (await command.run(async () => { const outcome = await tauri.approvePlanDifficulty(preview); if (outcome.changed > 0) onApplied(outcome.reason); })) onClose();
    }}>{t("Apply changes")}</Button>}</div>
  </Dialog>;
}