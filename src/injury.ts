import { tauri, el, fail, fmtDate, fmtKm, inTauri } from "./lib/ipc";
import { initSyncIndicator } from "./lib/sync";
import { requirePlanOrRedirect } from "./lib/navgate";
// Side-effect import: paints the saved colour scheme before first paint.
import "./lib/theme";
import "./lib/navigation";
import { BODY_REGIONS, regionLabel, renderBodyMap } from "./lib/bodymap";
import { withBusy } from "./lib/feedback";
import { scrollIntoViewMotion } from "./lib/motion";
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
      el<HTMLButtonElement>("pain-save").disabled = false;
      el<HTMLInputElement>("chat-question").disabled = false;
      el<HTMLButtonElement>("chat-send").disabled = false;
      refreshBodyMap();
      void loadThread();
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
  for (const e of entries) {
    const card = document.createElement("article");
    card.className = `pain-card surface ${e.resolvedOn ? "resolved" : "active"}`;
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
      <footer class="pain-actions actions"></footer>`;
    const actions = card.querySelector(".pain-actions") as HTMLElement;
    if (!e.resolvedOn) {
      actions.appendChild(
        confirmButton("resolve-" + e.id, "It's gone", "Sure? Mark resolved?", () =>
          resolveEntry(e.id),
        ),
      );
    }
    const del = confirmButton("del-" + e.id, "Delete", "Really delete?", async () => {
      await tauri.deletePainEntry(e.id);
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
    btn.disabled = true;
    try {
      await act();
    } catch (err) {
      fail(err);
      btn.disabled = false;
      btn.textContent = label;
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
      await withBusy(btn, "Accept change", async () => {
        await tauri.applyProposal("coros", proposal!.id);
        proposal = null;
        el("proposal-section").hidden = true;
        el("proposal-card").innerHTML = "";
      });
    } catch (err) {
      fail(err);
    }
  });
  el<HTMLButtonElement>("proposal-dismiss").addEventListener("click", async () => {
    try {
      await tauri.dismissProposal(proposal!.id, null);
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
      ? `Chatting with ${status.model ?? "your model"} at ${status.endpoint}`
      : "No assistant configured — replies here come from the built-in logger. Set an endpoint in Settings to chat.";
  } catch {
    assistantOn = false;
    el("assistant-mode-note").textContent =
      "Replies come from the built-in logger. Set an assistant endpoint in Settings to chat.";
  }
}

async function loadThread(): Promise<void> {
  const thread = el("chat-thread");
  if (!selected) {
    thread.innerHTML = `<p class="muted">Pick a spot on the body map to start a thread about it.</p>`;
    return;
  }
  try {
    const msgs = await tauri.painMessages(selected);
    thread.innerHTML = msgs.length
      ? msgs
          .map(
            (m) =>
              `<div class="chat-msg ${m.role}"><span class="chat-who">${m.role === "user" ? "You" : "Assistant"}</span><p>${escapeHtml(m.content)}</p></div>`,
          )
          .join("")
      : `<p class="muted">No messages about the ${regionLabel(selected)} yet.</p>`;
    thread.scrollTop = thread.scrollHeight;
  } catch (err) {
    fail(err);
  }
}

async function sendQuestion(): Promise<void> {
  if (!selected) return;
  const input = el<HTMLInputElement>("chat-question");
  const question = input.value.trim();
  if (!question) return;
  input.value = "";
  const btn = el<HTMLButtonElement>("chat-send");
  btn.disabled = true;
  btn.textContent = "…";
  try {
    const reply = await tauri.askAssistant(selected, question);
    await appendTurn({
      id: "pending",
      bodyPart: selected,
      role: "user",
      content: question,
      ts: reply.ts,
    });
    await appendTurn(reply);
  } catch (err) {
    fail(err);
  } finally {
    btn.disabled = false;
    btn.textContent = "Ask";
  }
}

async function appendTurn(msg: PainMessage): Promise<void> {
  const thread = el("chat-thread");
  const placeholder = thread.querySelector(".muted");
  if (placeholder) placeholder.remove();
  const div = document.createElement("div");
  div.className = `chat-msg ${msg.role}`;
  div.innerHTML = `<span class="chat-who">${msg.role === "user" ? "You" : "Assistant"}</span><p>${escapeHtml(msg.content)}</p>`;
  thread.appendChild(div);
  thread.scrollTop = thread.scrollHeight;
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
  el<HTMLButtonElement>("chat-send").addEventListener("click", sendQuestion);
  el<HTMLInputElement>("chat-question").addEventListener("keydown", (ev) => {
    if (ev.key === "Enter") sendQuestion();
  });
  el<HTMLButtonElement>("chat-clear").addEventListener("click", async () => {
    if (!selected) return;
    try {
      await tauri.clearPainMessages(selected);
      await loadThread();
    } catch (err) {
      fail(err);
    }
  });
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
void requirePlanOrRedirect("gated").then((redirected) => {
  if (!redirected) init();
});
if (inTauri) {
  initSyncIndicator();
  void load();
} else {
  el("bodymap").innerHTML = "";
  el("pain-list").innerHTML =
    `<p class="muted">Runalytics needs the desktop app for data — run \`npm run tauri dev\` and open it there.</p>`;
}
