import { test } from "node:test";
import assert from "node:assert/strict";
import { createNativeChromeBridge } from "../src/desk/native-chrome.ts";

const input = {
  tabs: [
    { id: "training", title: "Training" },
    { id: "journal", title: "Activities" },
    { id: "health", title: "Health" },
  ],
  selected: "training",
  theme: "light",
};

function deferred() {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
}

function fixture(overrides = {}) {
  const calls = [];
  const states = [];
  const routes = [];
  let receive;
  let latest;
  const transport = {
    async listen(handler) {
      calls.push("listen");
      receive = handler;
      return async () => { calls.push("unlisten"); };
    },
    async attach(snapshot) {
      calls.push("attach");
      latest = snapshot;
      return { ...snapshot, bottomInset: 83 };
    },
    async detach() { calls.push("detach"); },
    ...overrides,
  };
  const bridge = createNativeChromeBridge(transport, {
    onNavigate: route => routes.push(route),
    onState: state => states.push(state),
  });
  return {
    bridge, calls, states, routes,
    latest: () => latest,
    send: patch => receive({ ...latest, action: "navigate", eventId: "tap-1", workspace: "journal", ...patch }),
  };
}

test("native chrome stays inert until explicitly updated and listens before attaching", async () => {
  const host = fixture();
  assert.deepEqual(host.calls, []);
  assert.equal(await host.bridge.update(input), true);
  assert.deepEqual(host.calls, ["listen", "attach"]);
  assert.deepEqual(host.states.at(-1), { ready: true, bottomInset: 83 });
  await host.bridge.close();
});

test("native chrome rejects stale, malformed, cross-session and duplicate intents", async () => {
  const host = fixture();
  await host.bridge.update(input);
  host.send({ session: "previous-document" });
  host.send({ revision: 0 });
  host.send({ workspace: "settings" });
  host.send({ action: "delete" });
  host.send({ eventId: "" });
  host.send({ eventId: 7 });
  assert.deepEqual(host.routes, []);
  host.send({});
  host.send({ eventId: "tap-2" });
  assert.deepEqual(host.routes, ["journal"]);
  assert.deepEqual(host.states.at(-1), { ready: true, bottomInset: 83 });
  await host.bridge.update({ ...input, selected: "journal" });
  host.send({ workspace: "health" });
  assert.deepEqual(host.routes, ["journal"]);
  host.send({ eventId: "tap-3", workspace: "health" });
  assert.deepEqual(host.routes, ["journal", "health"]);
  await host.bridge.close();
});

test("native chrome never becomes ready before listener registration completes", async () => {
  const listener = deferred();
  let attachments = 0;
  const host = fixture({
    listen: () => listener.promise,
    attach: async snapshot => { attachments += 1; return { ...snapshot, bottomInset: 83 }; },
  });
  const updating = host.bridge.update(input);
  await Promise.resolve();
  assert.equal(attachments, 0);
  assert.equal(host.states.at(-1).ready, false);
  listener.resolve(() => {});
  assert.equal(await updating, true);
  await host.bridge.close();
});

test("native chrome retains acknowledged ownership during appearance and route updates", async () => {
  const replacement = deferred();
  const started = deferred();
  let attachments = 0;
  const host = fixture({ attach: async snapshot => {
    attachments += 1;
    if (attachments === 1) return { ...snapshot, bottomInset: 83 };
    started.resolve(snapshot);
    return replacement.promise;
  } });
  await host.bridge.update(input);
  host.states.length = 0;
  const updating = host.bridge.update({ ...input, selected: "journal", theme: "dark" });
  const snapshot = await started.promise;
  assert.deepEqual(host.states, []);
  host.send({ eventId: "during-update" });
  assert.deepEqual(host.routes, []);
  replacement.resolve({ ...snapshot, bottomInset: 83 });
  assert.equal(await updating, true);
  assert.ok(host.states.every(state => state.ready));
  await host.bridge.close();
});

for (const failure of ["listener", "attach", "protocol", "session", "revision", "inset"]) {
  test(`native chrome preserves fallback after ${failure} failure`, async () => {
    const host = fixture({
      ...(failure === "listener" ? { listen: async () => { throw new Error("Unavailable"); } } : {}),
      attach: async snapshot => {
        if (failure === "attach") throw new Error("Unavailable");
        return {
          ...snapshot, bottomInset: 83,
          ...(failure === "protocol" ? { protocolVersion: 2 } : {}),
          ...(failure === "session" ? { session: "old" } : {}),
          ...(failure === "revision" ? { revision: 0 } : {}),
          ...(failure === "inset" ? { bottomInset: NaN } : {}),
        };
      },
    });
    assert.equal(await host.bridge.update(input), false);
    assert.equal(host.states.at(-1).ready, false);
    assert.ok(host.calls.includes("detach"));
    await host.bridge.close();
  });
}

test("native chrome blocks intents and detaches when a modal owner suspends tabs", async () => {
  const host = fixture();
  await host.bridge.update(input);
  const suspending = host.bridge.update(null);
  host.send({});
  assert.deepEqual(host.routes, []);
  assert.equal(await suspending, false);
  assert.equal(host.calls.at(-1), "detach");
  assert.equal(await host.bridge.update({ ...input, selected: null }), true);
  assert.equal(host.latest().selected, null);
  await host.bridge.close();
});

test("native chrome supersedes queued snapshots and copies mutable input", async () => {
  const host = fixture();
  const first = host.bridge.update(input);
  const mutable = structuredClone(input);
  mutable.colors = { surface: [1, 1, 1, 1], selected: [.2, .4, .6, 1], unselected: [.3, .3, .3, 1] };
  const second = host.bridge.update(mutable);
  mutable.tabs[0].title = "Changed later";
  mutable.colors.selected[0] = 1;
  assert.equal(await first, false);
  assert.equal(await second, true);
  assert.equal(host.latest().tabs[0].title, "Training");
  assert.deepEqual(host.latest().colors.selected, [.2, .4, .6, 1]);
  assert.equal(host.calls.filter(call => call === "attach").length, 1);
  await host.bridge.close();
});

test("native chrome detaches a late attachment after shutdown and unregisters once", async () => {
  const attachment = deferred();
  const started = deferred();
  const host = fixture({ attach: async snapshot => { started.resolve(snapshot); return attachment.promise; } });
  const updating = host.bridge.update(input);
  const snapshot = await started.promise;
  const closing = host.bridge.close();
  assert.equal(host.bridge.close(), closing);
  attachment.resolve({ ...snapshot, bottomInset: 83 });
  assert.equal(await updating, false);
  await closing;
  assert.equal(host.states.at(-1).ready, false);
  assert.equal(host.calls.filter(call => call === "detach").length, 1);
  assert.equal(host.calls.filter(call => call === "unlisten").length, 1);
  assert.equal(await host.bridge.update(input), false);
});