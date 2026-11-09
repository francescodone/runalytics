export type NativeWorkspace = "training" | "journal" | "health";
type ChromeInput = {
  tabs: readonly { id: NativeWorkspace; title: string }[];
  selected: NativeWorkspace | null;
  theme: "light" | "dark";
};
type Snapshot = ChromeInput & { protocolVersion: 1; session: string; revision: number };
type ChromeState = { ready: boolean; bottomInset: number };
type Transport = {
  listen: (receive: (intent: unknown) => void) => Promise<() => void | Promise<void>>;
  attach: (snapshot: Snapshot) => Promise<unknown>;
  detach: () => Promise<unknown>;
};

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

export function createNativeChromeBridge(transport: Transport, options: {
  onNavigate: (workspace: NativeWorkspace) => void;
  onState: (state: ChromeState) => void;
}) {
  const session = crypto.randomUUID();
  let revision = 0;
  let active: Snapshot | null = null;
  let ready = false;
  let stopped = false;
  let queue: Promise<unknown> = Promise.resolve();
  let listening: Promise<() => void | Promise<void>> | undefined;
  let closing: Promise<void> | undefined;
  const seen = new Set<string>();
  const unavailable = () => {
    ready = false;
    active = null;
    options.onState({ ready: false, bottomInset: 0 });
  };
  const receive = (intent: unknown) => {
    if (stopped || !ready || !active || !record(intent) || intent.session !== session ||
      intent.revision !== active.revision || intent.action !== "navigate" ||
      typeof intent.eventId !== "string" || !intent.eventId || seen.has(intent.eventId) ||
      !active.tabs.some(tab => tab.id === intent.workspace)) return;
    seen.add(intent.eventId);
    unavailable();
    options.onNavigate(intent.workspace as NativeWorkspace);
  };
  const detach = async () => {
    try { await transport.detach(); } catch {}
  };
  const update = (input: ChromeInput | null): Promise<boolean> => {
    if (stopped) return Promise.resolve(false);
    const version = ++revision;
    const next: Snapshot | null = input ? {
      protocolVersion: 1, session, revision: version,
      tabs: input.tabs.map(tab => ({ ...tab })), selected: input.selected, theme: input.theme,
    } : null;
    unavailable();
    const operation = queue.then(async () => {
      if (stopped || version !== revision) return false;
      if (!next) { await detach(); return false; }
      try {
        listening ??= transport.listen(receive);
        await listening;
        if (stopped || version !== revision) return false;
        const ack = await transport.attach(next);
        if (stopped || version !== revision) return false;
        if (!record(ack) || ack.protocolVersion !== 1 || ack.session !== session ||
          ack.revision !== version || typeof ack.bottomInset !== "number" ||
          !Number.isFinite(ack.bottomInset) || ack.bottomInset <= 0 || ack.bottomInset > 4096) {
          await detach();
          return false;
        }
        active = next;
        ready = true;
        options.onState({ ready: true, bottomInset: ack.bottomInset });
        return true;
      } catch {
        await detach();
        return false;
      }
    });
    queue = operation;
    return operation;
  };
  const close = (): Promise<void> => {
    if (closing) return closing;
    stopped = true;
    ++revision;
    unavailable();
    closing = queue.then(async () => {
      await detach();
      if (listening) {
        try { await (await listening)(); } catch {}
      }
    });
    return closing;
  };
  return { update, close };
}

export async function createTauriChromeTransport(): Promise<Transport> {
  const { addPluginListener, invoke } = await import("@tauri-apps/api/core");
  return {
    listen: async receive => {
      const listener = await addPluginListener("native-chrome", "intent", receive);
      return async () => {
        try { await listener.unregister(); }
        catch {
          await invoke("plugin:native-chrome|removeListener", { event: listener.event, channelId: listener.channelId });
        }
      };
    },
    attach: snapshot => invoke("plugin:native-chrome|attach", snapshot),
    detach: () => invoke("plugin:native-chrome|detach"),
  };
}