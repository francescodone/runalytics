import { tauri, el, fail, fmtDate, fmtKm, inTauri } from "./lib/ipc";
import { initSyncIndicator } from "./lib/sync";
import { requirePlanOrRedirect } from "./lib/navgate";
// Side-effect import: paints the saved colour scheme before first paint.
import "./lib/theme";
import "./lib/navigation";
import { BODY_REGIONS, regionLabel, renderBodyMap } from "./lib/bodymap";
import { icons } from "./lib/icons";
import { withBusy } from "./lib/feedback";
import { scrollIntoViewMotion } from "./lib/motion";
import { initGroupProgress, initScrollReveal } from "./lib/reveal";
import type { PainEntry, PainMessage, PlanProposal } from "./lib/types";

// The injury view: log pain on a coarse body map, review the plan-change
// offer the backend proposes, and keep a journal thread per body region.

/// Region currently selected on the body map; gates the form and the chat.
let selected: string | null = null;
/// Worst recent intensity per region, for tinting the map.
let tints = new Map<string, number>();
/// All journal entries, newest first (backend order).
let entries: PainEntry[] = [];
/// The unanswered offer, if any.
let proposal: PlanProposal | null = null;
/// Whether the LLM assistant is configured (vs. plain logger replies).
let assistantOn = false;
/// Two-step confirm state for destructive buttons ("yes" arms, second click acts).
let armed: string | null = null;
/// Whether the journal has played its entrance cascade yet (first paint only).
let journalCascaded = false;

function today(): string {
  const d = new Date();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

function intensityColor(v: number): string {
  return v >= 7 ? "sev-high" : v >= 5 ? "sev-mid" : "sev-low";
}

// ---- body map + log form -------------------------------------------------

function refreshBodyMap(): void {
  renderBodyMap(el("bodymap"), {
    selected,
    tints,
    onSelect: (key) => {
      selected = key;
      el("bodymap-selected").textContent = `Selected: ${BODY_REGIONS[key] ?? key}`;
      el<HTMLButtonElement>("pain-save").disabled = !inTauri;
      refreshBodyMap();
    },
  });
}

async function loadTints(): Promise<void> {
  try {
    const pairs = await tauri.painByRegion(28);
    tints = new Map(pairs);
  } catch {
    tints = new Map(); // tinting is decorative; never block the page on it
  }
  refreshBodyMap();
}

async function savePain(): Promise<void> {
  if (!selected) return;
  // Capture the selection in a const: `selected` is a module-level `let`, so
  // TypeScript won't carry the null-check into the async callback below.
  const region = selected;
  const btn = el<HTMLButtonElement>("pain-save");
  try {
    const offer = await withBusy(btn, "Log pain", async () => {
      const intensity = parseInt(el<HTMLInputElement>("pain-intensity").value, 10);
      const startedOn = el<HTMLInputElement>("pain-started").value || today();
      const notes = el<HTMLTextAreaElement>("pain-notes").value.trim();
      const [, offer] = await tauri.savePainEntry(region, intensity, startedOn, notes);
      el<HTMLTextAreaElement>("pain-notes").value = "";
      if (offer) proposal = offer;
      await Promise.all([loadEntries(), loadTints(), loadProposal()]);
      return offer;
    });
    if (offer) {
      el("proposal-section").hidden = false;
      scrollIntoViewMotion(el("proposal-section"), { block: "start" });
    }
  } catch (err) {
    fail(err);
  }
}

// ---- journal list ---------------------------------------------------------

async function loadEntries(): Promise<void> {
  entries = await tauri.painEntries();
  const list = el("pain-list");
  if (entries.length === 0) {
    list.innerHTML = `<p class="muted">Nothing logged yet — pick a spot above.</p>`;
    return;
  }
  list.innerHTML = "";
  // The journal cascades in only on the first render that has anything to
  // show; re-renders after a resolve/delete/save build plain cards, so the
  // entrance never replays for cards the runner was already reading.
  const cascade = !journalCascaded && entries.length > 0;
  if (cascade) journalCascaded = true;
  for (const e of entries) {
    const card = document.createElement("article");
    card.className = `pain-card surface ${e.resolvedOn ? "resolved" : "active"}${cascade ? " pain-enter" : ""}`;
    const status = e.resolvedOn
      ? `resolved ${fmtDate(e.resolvedOn)}`
      : "still hurting";
    card.innerHTML = `
      <header>
        <span class="chip pain-chip ${intensityColor(e.intensity)}">${e.intensity}/10</span>
        <strong>${regionLabel(e.bodyPart)}</strong>
        <span class="muted">since ${fmtDate(e.startedOn)} · ${status}</span>
      </header>
      ${e.notes ? `<p class="pain-notes">${escapeHtml(e.notes)}</p>` : ""}
      <div class="entry-chat" hidden></div>
      <footer class="pain-actions actions"></footer>`;
    const actions = card.querySelector(".pain-actions") as HTMLElement;
    // One chat per log: the thread belongs to this episode, not to the body
    // region, so two knee entries two months apart get two separate
    // conversations. Collapsed until asked — the journal stays a journal.
    const chatBtn = document.createElement("button");
    chatBtn.type = "button";
    chatBtn.className = "secondary";
    const chat = card.querySelector(".entry-chat") as HTMLElement;
    chat.id = `entry-chat-${e.id}`;
    chatBtn.setAttribute("aria-controls", chat.id);
    const showChat = (open: boolean) => {
      chat.hidden = !open;
      chatBtn.textContent = open ? "Hide chat" : "Ask the assistant";
      chatBtn.setAttribute("aria-expanded", String(open));
      if (open) initEntryChat(chat, e, chatBtn);
    };
    showChat(chatStates.get(e.id)?.open ?? false);
    chatBtn.addEventListener("click", () => {
      const open = chat.hidden !== false;
      showChat(open);
      chatStates.get(e.id)!.open = open;
    });
    actions.appendChild(chatBtn);
    if (!e.resolvedOn) {
      actions.appendChild(
        confirmButton("resolve-" + e.id, "It's gone", "Sure? Mark resolved?", () =>
          resolveEntry(e.id),
        ),
      );
    }
    const del = confirmButton("del-" + e.id, "Delete", "Really delete?", async () => {
      await tauri.deletePainEntry(e.id);
      chatStates.delete(e.id);
      await Promise.all([loadEntries(), loadTints()]);
    });
    del.classList.add("danger");
    actions.appendChild(del);
    list.appendChild(card);
  }
}

/// Two-step confirm as a plain button — window.confirm is a no-op in WKWebView.
function confirmButton(
  id: string,
  label: string,
  confirmLabel: string,
  act: () => Promise<void>,
): HTMLButtonElement {
  const btn = document.createElement("button");
  btn.id = id;
  btn.className = "secondary";
  btn.textContent = label;
  btn.addEventListener("click", async () => {
    if (armed !== id) {
      armed = id;
      btn.textContent = confirmLabel;
      btn.classList.add("danger");
      window.setTimeout(() => {
        if (armed === id) {
          armed = null;
          btn.textContent = label;
          btn.classList.remove("danger");
        }
      }, 4000);
      return;
    }
    armed = null;
    try {
      await withBusy(btn, id.startsWith("del-") ? "Deleting…" : "Resolving…", () => act());
    } catch (err) {
      fail(err);
      btn.textContent = label;
      btn.classList.remove("danger");
    }
  });
  return btn;
}

async function resolveEntry(id: string): Promise<void> {
  await tauri.resolvePain(id, today());
  await Promise.all([loadEntries(), loadTints()]);
}

// ---- proposal card ---------------------------------------------------------

async function loadProposal(): Promise<void> {
  proposal = await tauri.pendingProposal();
  const section = el("proposal-section");
  if (!proposal) {
    section.hidden = true;
    return;
  }
  section.hidden = false;
  renderProposal();
}

function renderProposal(): void {
  if (!proposal) return;
  const card = el("proposal-card");
  // One row per touched day, shaped as a diff: the day, what the plan says
  // today (struck through), and what it becomes after accepting. A bare
  // "Easy run (was: …)" line left the runner guessing which half was real.
  const edits = proposal.edits
    .map((e) => {
      const day = e.date
        ? new Date(`${e.date}T12:00:00`).toLocaleDateString(undefined, {
            weekday: "short",
            month: "short",
            day: "numeric",
          })
        : "";
      const was = e.oldTitle ?? e.title;
      const wasKm =
        e.oldKind != null && e.oldKind !== "rest" && e.oldDistanceM != null
          ? ` · ${fmtKm(e.oldDistanceM)}`
          : "";
      const nowKm = e.kind === "rest" ? "" : e.distanceM != null ? ` · ${fmtKm(e.distanceM)}` : "";
      const nowMin = e.kind === "rest" || e.durationMin == null ? "" : ` · ${Math.round(e.durationMin)} min`;
      // The stored title carries its own "(was: …)" suffix; the struck line
      // above already says that, so don't print it twice.
      const nowTitle = e.title.replace(/\s*\(was: .*\)$/, "");
      return `<li>${day ? `<span class="proposal-edit-day">${escapeHtml(day)}</span>` : ""}<span class="proposal-edit-was">${escapeHtml(was)}${wasKm}</span><span class="proposal-edit-now"><span class="proposal-edit-arrow" aria-hidden="true">→</span> ${escapeHtml(nowTitle)}${nowKm}${nowMin}</span></li>`;
    })
    .join("");
  card.innerHTML = `
    <p class="proposal-summary">${escapeHtml(proposal.summary)}</p>
    <p class="muted">${escapeHtml(proposal.reason)}</p>
    <ul class="proposal-edits">${edits}</ul>
    <div class="actions">
      <button id="proposal-accept" class="primary">Accept change</button>
      <button id="proposal-dismiss" class="secondary">Not now</button>
    </div>`;
  el<HTMLButtonElement>("proposal-accept").addEventListener("click", async (ev) => {
    const btn = ev.currentTarget as HTMLButtonElement;
    try {
      await withBusy(btn, "Applying…", async () => {
        await tauri.applyProposal("coros", proposal!.id);
        proposal = null;
        el("proposal-section").hidden = true;
        el("proposal-card").innerHTML = "";
      });
    } catch (err) {
      fail(err);
    }
  });
  el<HTMLButtonElement>("proposal-dismiss").addEventListener("click", async (ev) => {
    try {
      await withBusy(ev.currentTarget as HTMLButtonElement, "Dismissing…", () => tauri.dismissProposal(proposal!.id, null));
      proposal = null;
      el("proposal-section").hidden = true;
    } catch (err) {
      fail(err);
    }
  });
}

// ---- assistant thread ------------------------------------------------------

async function loadAssistantStatus(): Promise<void> {
  try {
    const status = await tauri.llmStatus();
    assistantOn = status.configured;
    el("assistant-mode-note").textContent = assistantOn
      ? `Chatting with ${status.model ?? "your model"}`
      : "No assistant configured — replies here come from the built-in logger. Set an endpoint in Settings to chat.";
  } catch {
    assistantOn = false;
    el("assistant-mode-note").textContent =
      "Replies come from the built-in logger. Set an assistant endpoint in Settings to chat.";
  }
  document.querySelectorAll<HTMLElement>(".entry-chat .chat-mode").forEach((mode) => {
    mode.textContent = assistantOn ? "AI assistant" : "Built-in";
  });
}

// ---- per-entry assistant chat ----------------------------------------------
// Each journal card owns its thread. The chat DOM is built on first open and
// then reused (messages cached in `state`), so collapsing and re-expanding
// doesn't refetch, and a reply in flight survives the toggle.

interface EntryChatState {
  msgs: PainMessage[];
  loaded: boolean;
  loading: Promise<void> | null;
  sending: boolean;
  clearing: boolean;
  draft: string;
  error: string | null;
  announcement: string;
  open: boolean;
  render?: () => void;
}
const chatStates = new Map<string, EntryChatState>();

function chatBubble(m: PainMessage): HTMLElement {
  const div = document.createElement("div");
  div.className = `chat-msg ${m.role === "user" ? "user" : "assistant"}`;
  const who = document.createElement("span");
  who.className = "chat-who";
  who.textContent = m.role === "user" ? "You" : "Assistant";
  const body = document.createElement("p");
  body.textContent = m.content;
  div.append(who, body);
  return div;
}

function initEntryChat(host: HTMLElement, entry: PainEntry, opener: HTMLButtonElement): void {
  let state = chatStates.get(entry.id);
  if (!state) {
    state = { msgs: [], loaded: false, loading: null, sending: false, clearing: false, draft: "", error: null, announcement: "", open: true };
    chatStates.set(entry.id, state);
  }

  const header = document.createElement("header");
  header.className = "chat-header";
  const mark = document.createElement("span");
  mark.className = "chat-mark";
  mark.setAttribute("aria-hidden", "true");
  mark.innerHTML = icons.sparkles;
  const heading = document.createElement("span");
  heading.className = "chat-heading";
  const title = document.createElement("strong");
  title.textContent = "Coach chat";
  const context = document.createElement("small");
  context.textContent = `${regionLabel(entry.bodyPart)} entry`;
  heading.append(title, context);
  const mode = document.createElement("span");
  mode.className = "chat-mode";
  mode.textContent = assistantOn ? "AI assistant" : "Built-in";
  header.append(mark, heading, mode);

  const thread = document.createElement("div");
  thread.className = "chat-thread";
  thread.setAttribute("role", "log");
  thread.setAttribute("aria-label", `Conversation about ${regionLabel(entry.bodyPart)}`);
  thread.setAttribute("aria-live", "off");
  const composer = document.createElement("form");
  composer.className = "chat-composer";
  const input = document.createElement("textarea");
  input.id = `chat-question-${entry.id}`;
  input.name = "question";
  input.rows = 2;
  input.maxLength = 2000;
  input.value = state.draft;
  input.placeholder = `Ask about your ${regionLabel(entry.bodyPart).toLowerCase()}…`;
  input.setAttribute("aria-label", `Ask about ${regionLabel(entry.bodyPart).toLowerCase()}`);
  const send = document.createElement("button");
  send.type = "submit";
  send.textContent = "Send";
  send.disabled = true;
  composer.append(input, send);
  const feedback = document.createElement("p");
  feedback.className = "chat-feedback";
  feedback.setAttribute("role", "alert");
  feedback.hidden = true;
  const footer = document.createElement("div");
  footer.className = "chat-footer";
  const hint = document.createElement("span");
  hint.className = "chat-hint";
  hint.textContent = "Enter to send · Shift+Enter for a new line";
  const clear = document.createElement("button");
  clear.type = "button";
  clear.className = "secondary chat-clear";
  clear.textContent = "Clear history";
  footer.append(hint, clear);
  const status = document.createElement("p");
  status.className = "sr-only";
  status.setAttribute("role", "status");
  host.replaceChildren(header, thread, composer, feedback, footer, status);
  const fitInput = () => {
    input.style.height = "auto";
    if (input.scrollHeight) input.style.height = `${Math.min(input.scrollHeight, 160)}px`;
  };
  fitInput();

  function paint(): void {
    thread.replaceChildren();
    if (!state!.loaded) {
      const message = document.createElement("p");
      message.className = "chat-empty-message";
      message.textContent = state!.error ?? "Loading conversation…";
      thread.append(message);
      if (state!.error) {
        const retry = document.createElement("button");
        retry.type = "button";
        retry.className = "secondary chat-retry";
        retry.textContent = "Try again";
        retry.addEventListener("click", loadHistory);
        thread.append(retry);
      } else {
        const spinner = document.createElement("span");
        spinner.className = "chat-loading-indicator";
        spinner.setAttribute("aria-hidden", "true");
        thread.prepend(spinner);
      }
    } else if (!state!.msgs.length && !state!.sending) {
      const empty = document.createElement("div");
      empty.className = "chat-empty";
      const invitation = document.createElement("p");
      invitation.textContent = "Ask about this entry, or start with a question:";
      const suggestions = document.createElement("div");
      suggestions.className = "chat-suggestions";
      for (const prompt of ["How should I adjust my next run?", "What should I watch for?"]) {
        const button = document.createElement("button");
        button.type = "button";
        button.className = "secondary chat-suggestion";
        button.textContent = prompt;
        button.addEventListener("click", () => {
          input.value = prompt;
          state!.draft = prompt;
          fitInput();
          send.disabled = false;
          input.focus();
        });
        suggestions.append(button);
      }
      empty.append(invitation, suggestions);
      thread.append(empty);
    } else {
      for (const m of state!.msgs) thread.append(chatBubble(m));
    }
    if (state!.sending && !state!.clearing) {
      const pending = document.createElement("div");
      pending.className = "chat-msg assistant chat-pending";
      pending.setAttribute("aria-hidden", "true");
      const who = document.createElement("span");
      who.className = "chat-who";
      who.textContent = "Assistant";
      const dots = document.createElement("span");
      dots.className = "chat-thinking";
      dots.setAttribute("aria-hidden", "true");
      dots.append(document.createElement("i"), document.createElement("i"), document.createElement("i"));
      const text = document.createElement("p");
      text.textContent = "Preparing a reply…";
      pending.append(who, dots, text);
      thread.append(pending);
    }
    if (state!.clearing) {
      const clearing = document.createElement("p");
      clearing.className = "chat-clearing";
      clearing.textContent = "Clearing conversation…";
      thread.append(clearing);
    }
    if (input.value !== state!.draft) {
      input.value = state!.draft;
      fitInput();
    }
    input.disabled = !state!.loaded || state!.clearing;
    send.disabled = !state!.loaded || state!.sending || !input.value.trim();
    clear.disabled = !state!.loaded || state!.sending || state!.msgs.length === 0;
    feedback.textContent = state!.loaded ? state!.error ?? "" : "";
    feedback.hidden = !feedback.textContent;
    status.textContent = state!.announcement;
    thread.scrollTop = thread.scrollHeight;
  }

  function loadHistory(): void {
    if (state!.loading) return;
    state!.error = null;
    state!.announcement = "Loading conversation";
    state!.loading = tauri.painMessages(entry.id)
      .then((msgs) => {
        state!.msgs = msgs;
        state!.loaded = true;
        state!.announcement = "Conversation loaded";
      })
      .catch(() => {
        state!.error = "Couldn't load this conversation. Try again.";
        state!.announcement = state!.error;
      })
      .finally(() => {
        state!.loading = null;
        // The card may have been re-rendered while loading; its current host
        // owns the paint, so a stale request never redraws a detached card.
        state!.render?.();
        if (state!.loaded && host.isConnected && !host.hidden && document.activeElement === opener) input.focus({ preventScroll: true });
      });
    state!.render?.();
  }

  async function ask(): Promise<void> {
    const question = input.value.trim();
    if (!question || !state!.loaded || state!.sending) return;
    const before = state!.msgs.length;
    const asked: PainMessage = {
      id: "pending",
      bodyPart: entry.bodyPart,
      entryId: entry.id,
      role: "user",
      content: question,
      ts: new Date().toISOString(),
    };
    state!.draft = "";
    input.value = "";
    fitInput();
    state!.sending = true;
    state!.error = null;
    state!.announcement = "Preparing a reply";
    state!.msgs.push(asked);
    state!.render?.();
    try {
      const reply = await withBusy(send, "Asking…", () => tauri.askAssistant(entry.id, question));
      state!.msgs.push(reply);
      state!.announcement = "Reply ready";
    } catch {
      // The backend may save the question before a reply fails; reload the
      // persisted thread before deciding what the runner needs to retry.
      try {
        const saved = await tauri.painMessages(entry.id);
        state!.msgs = saved;
        const last = saved[saved.length - 1];
        if (saved.length > before && last?.role === "user" && last.content === question) {
          state!.error = "Your question was saved, but no reply arrived. Check your assistant in Settings before asking again.";
        } else if (!(saved.length >= before + 2 && last?.role === "assistant")) {
          state!.error = "Couldn't get a reply. Try sending your question again.";
          if (!state!.draft.trim()) state!.draft = question;
        }
      } catch {
        state!.msgs = state!.msgs.filter((m) => m !== asked);
        state!.error = "Couldn't confirm whether this question was saved. Check your connection, then reopen the chat.";
      }
      state!.announcement = state!.error ? "" : "Reply ready";
    } finally {
      state!.sending = false;
      state!.render?.();
      if (host.isConnected && !host.hidden) input.focus({ preventScroll: true });
    }
  }

  composer.addEventListener("submit", (ev) => {
    ev.preventDefault();
    void ask();
  });
  input.addEventListener("input", () => {
    state!.draft = input.value;
    fitInput();
    if (state!.error && state!.loaded) {
      state!.error = null;
      state!.announcement = "";
      feedback.hidden = true;
      status.textContent = "";
    }
    send.disabled = state!.sending || !input.value.trim();
  });
  input.addEventListener("keydown", (ev) => {
    if (ev.key === "Enter" && !ev.shiftKey && !ev.isComposing) {
      ev.preventDefault();
      void ask();
    }
  });
  let clearArmed = false;
  let clearTimer = 0;
  const resetClear = () => {
    clearArmed = false;
    window.clearTimeout(clearTimer);
    clear.textContent = "Clear history";
    clear.classList.remove("is-confirming");
  };
  clear.addEventListener("click", async () => {
    if (state!.sending || !state!.msgs.length) return;
    if (!clearArmed) {
      clearArmed = true;
      clear.textContent = "Clear this chat?";
      clear.classList.add("is-confirming");
      clearTimer = window.setTimeout(resetClear, 4000);
      return;
    }
    window.clearTimeout(clearTimer);
    state!.sending = true;
    state!.clearing = true;
    state!.error = null;
    state!.announcement = "Clearing conversation";
    state!.render?.();
    try {
      await withBusy(clear, "Clearing…", () => tauri.clearPainMessages(entry.id));
      state!.msgs = [];
      state!.announcement = "Conversation cleared";
    } catch {
      state!.error = "Couldn't clear the conversation. Your messages are still here.";
      state!.announcement = "";
    } finally {
      state!.sending = false;
      state!.clearing = false;
      resetClear();
      state!.render?.();
    }
  });

  state.render = () => { if (host.isConnected && !host.hidden) paint(); };
  if (!state.loaded && !state.loading && !state.error) loadHistory();
  paint();
  // Land the keyboard in the composer once the thread is ready to use.
  if (state.loaded && host.isConnected) input.focus({ preventScroll: true });
}

// ---- follow-up ("is it gone?") ----------------------------------------------
// Once per episode, 3+ days after it was logged, ask whether the pain went
// away. mark_pain_followup persists the ask so it never nags twice.

function checkFollowup(): void {
  const open = entries.find((e) => !e.resolvedOn && !e.followupAt);
  if (!open) return;
  const ageDays =
    (Date.now() - new Date(open.createdAt).getTime()) / 86_400_000;
  if (ageDays < 3) return;
  void tauri.markPainFollowup(open.id);
  const banner = document.createElement("div");
  banner.className = "notice soft";
  banner.innerHTML = `<p>Is the ${regionLabel(open.bodyPart)} pain gone yet?</p>`;
  const yes = document.createElement("button");
  yes.className = "primary";
  yes.textContent = "Yes, it's gone";
  yes.addEventListener("click", () =>
    resolveEntry(open.id).then(() => banner.remove()),
  );
  const later = document.createElement("button");
  later.className = "secondary";
  later.textContent = "Still hurting";
  later.addEventListener("click", () => banner.remove());
  banner.appendChild(yes);
  banner.appendChild(later);
  el("journal-section").prepend(banner);
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

// ---- wiring -----------------------------------------------------------------

function init(): void {
  el<HTMLInputElement>("pain-started").value = today();
  el<HTMLInputElement>("pain-started").max = today();
  const slider = el<HTMLInputElement>("pain-intensity");
  slider.addEventListener("input", () => {
    el("intensity-value").textContent = slider.value;
  });
  el<HTMLButtonElement>("pain-save").addEventListener("click", savePain);
  refreshBodyMap();
}

async function load(): Promise<void> {
  try {
    await Promise.all([loadEntries(), loadTints(), loadProposal(), loadAssistantStatus()]);
    checkFollowup();
  } catch (err) {
    fail(err);
  }
}

// The injury journal proposes plan changes, so with no plan it has nothing to
// act on — send the user to Dashboard + Settings instead.
// Same motion layer as the dashboard, armed at module time rather than after
// the gate: the rail and the reveal hook must exist before first paint, or
// the panels above the fold would fade out and back in when the gate
// resolves. The journal usually lands below the fold once entries exist, so
// it arrives when reached, not by having burned its entrance off-screen.
initGroupProgress();
initScrollReveal();
void requirePlanOrRedirect("gated").then((redirected) => {
  if (!redirected) init();
});
if (inTauri) {
  initSyncIndicator();
  void load();
} else {
  el("bodymap").innerHTML = "";
  const note = document.createElement("p");
  note.className = "muted";
  note.textContent = "Logging is available in the desktop app.";
  el("pain-save").after(note);
  el("pain-list").innerHTML =
    `<div class="empty-state"><h3>Your pain journal is on your Mac</h3><p>Open Runalytics in the desktop app to see previous entries and track recovery.</p></div>`;
}
