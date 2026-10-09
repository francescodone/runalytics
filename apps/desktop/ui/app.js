// Runalytics desktop UI — thin controller over the Tauri commands.
// The full React front-end replaces this file; for now it exercises every
// command path the shell exposes so the wiring is verifiable end-to-end.
const { invoke } = window.__TAURI.core;
const { listen } = window.__TAURI.event;

const $ = (id) => document.getElementById(id);
const log = (msg) => {
  const el = $("log");
  el.textContent =
    new Date().toLocaleTimeString() + "  " + msg + "\n" + el.textContent;
};

const fmtPace = (s) =>
  s == null ? "—" : `${Math.floor(s / 60)}:${String(Math.round(s % 60)).padStart(2, "0")}/km`;

function scorePill(v) {
  if (v == null) return `<span class="pill">n/a</span>`;
  const cls = v >= 70 ? "ok" : v >= 40 ? "warn" : "bad";
  return `<span class="pill ${cls}">${Math.round(v)}</span>`;
}

async function refresh() {
  try {
    const s = await invoke("get_status");
    const r = s.readinessToday?.score ?? null;
    const i = s.injuryRiskToday?.score ?? null;
    const load = s.latestMetrics?.weeklyLoadKm ?? null;
    $("cards").innerHTML = `
      <div class="card"><h3>Readiness</h3><div class="big">${scorePill(r)}</div><div class="sub">${s.today}</div></div>
      <div class="card"><h3>Injury risk</h3><div class="big">${scorePill(i)}</div><div class="sub">lower is safer</div></div>
      <div class="card"><h3>Weekly load</h3><div class="big">${load != null ? load.toFixed(1) + " km" : "—"}</div><div class="sub">${s.activePlan ? s.activePlan.name + " · " + s.activePlan.goal : "no active plan"}</div></div>`;
    const n = s.nextSession;
    $("next").innerHTML = n
      ? `<b>${n.title}</b><br>${n.date} ${n.start} · ${(n.targetVolumeKm ?? 0).toFixed(1)} km · ${fmtPace(n.targetPaceSecsPerKm)}`
      : "Nothing scheduled. Generate a plan below.";
    $("accounts").innerHTML = (s.accounts ?? [])
      .map(
        (a) =>
          `<div class="row"><span>${a.provider.toUpperCase()} · ${a.accountLabel}</span><span class="muted">${a.connected ? '<span class="pill ok">connected</span>' : '<span class="pill">offline</span>'}</span></div>`,
      )
      .join("") || `<div class="sub">No wearable connected.</div>`;
  } catch (e) {
    log("status failed: " + e);
  }
  try {
    const m = await invoke("mcp_registration");
    $("mcp").textContent = m.found
      ? "runalytics-mcp found next to the app. Add to your agent:\n" + JSON.stringify(m.config, null, 2)
      : "Sidecar not bundled yet — run `cargo build -p runalytics-mcp --release` and point your agent at target/release/runalytics-mcp --db <database>.";
  } catch (e) {
    log("mcp_registration failed: " + e);
  }
}

$("btn-connect").addEventListener("click", async () => {
  $("btn-connect").disabled = true;
  log("opening COROS consent in the browser…");
  try {
    const res = await invoke("connect_coros", {
      input: { accountLabel: "me", region: null },
    });
    log(`connected: ${res.account} (${(res.capabilities ?? []).join(", ")})`);
    await refresh();
  } catch (e) {
    log("connect failed: " + e);
  } finally {
    $("btn-connect").disabled = false;
  }
});

$("btn-sync").addEventListener("click", async () => {
  $("btn-sync").disabled = true;
  log("sync started…");
  try {
    const res = await invoke("sync_providers");
    for (const r of res.reports ?? []) {
      log(`sync ${r.account}: ${JSON.stringify(r.classes ?? r.error)}`);
    }
    await refresh();
  } catch (e) {
    log("sync failed: " + e);
  } finally {
    $("btn-sync").disabled = false;
  }
});

$("btn-calendar").addEventListener("click", async () => {
  try {
    const res = await invoke("sync_calendar");
    log(`calendar published to ${JSON.stringify(res.publishedTo)} (${res.sessions} sessions)`);
  } catch (e) {
    log("calendar sync failed: " + e);
  }
});

$("plan-form").addEventListener("submit", async (ev) => {
  ev.preventDefault();
  try {
    const template = await invoke("plan_request_template");
    template.goal = $("goal").value;
    template.anchor = { kind: "horizon", value: { weeks: Number($("weeks").value) } };
    const gen = await invoke("generate_plan", { request: template, save: true });
    log(`plan generated: ${gen.name} (${gen.planId})`);
    const act = await invoke("activate_plan", { planId: gen.planId });
    log(`plan active: ${act.name}`);
    const cal = await invoke("sync_calendar").catch((e) => log("calendar: " + e));
    if (cal) log(`published to ${JSON.stringify(cal.publishedTo)}`);
    await refresh();
  } catch (e) {
    log("plan failed: " + e);
  }
});

listen("runalytics://sync", (ev) => log("sync event: " + JSON.stringify(ev.payload)));
listen("runalytics://tray-sync", () => $("btn-sync").click());

await refresh();
