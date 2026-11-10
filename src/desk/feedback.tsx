import { useId, useState } from "react";
import { IconArrowDown, IconArrowUp, IconCheck, IconX } from "@tabler/icons-react";
import { tauri } from "../lib/ipc";
import type { FeedbackContext, FeedbackSession } from "../lib/types";
import { sessionTitle, shortDate, useCommand, useRemote, useWorkspace } from "./data";
import { t, text } from "./i18n";
import { Button, ErrorNotice } from "./ui";

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
  const command = useCommand();
  const { notify } = useWorkspace();
  const [result, setResult] = useState("");
  const context = feedback.data;
  const adjust = (direction: "easier" | "harder") => command.run(async () => {
    const latest = await tauri.sessionFeedback();
    if (!latest || latest.planId !== planId) throw new Error(t("The plan changed. Refresh before trying again."));
    const outcome = await tauri.adjustPlanDifficulty(planId, latest.revision, direction);
    setResult(text(outcome.reason));
    if (outcome.changed > 0) notify(text(outcome.reason), { label: t("Undo"), run: () => tauri.undoPlanDifficulty(planId) });
  });
  return <section className="edit-plan" aria-label={t("Edit plan")}><h3>{t("Edit plan")}</h3>
    <div className="feedback-actions"><Button disabled={command.busy || !context || context.planId !== planId} onClick={() => void adjust("easier")}><IconArrowDown size={18} />{t("Make plan slightly easier")}</Button>
      <Button disabled={command.busy || !context || context.planId !== planId} onClick={() => void adjust("harder")}><IconArrowUp size={18} />{t("Make plan slightly harder")}</Button></div>
    <ErrorNotice error={command.error ?? feedback.error} retry={feedback.retry} />
    {result && <p role="status">{result}</p>}
  </section>;
}