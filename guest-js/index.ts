/**
 * TypeScript bindings for the `fridge` Tauri plugin.
 *
 * The Rust side lives at `tauri-plugin-fridge/src/`. Mount the plugin in your
 * `tauri::Builder::default().plugin(tauri_plugin_fridge::init())` and add the
 * "fridge:default" permission to your capability file. After that, every
 * function below works.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// ---------- argument shapes ----------

export type Target =
    | { kind: 'name'; value: string }
    | { kind: 'pid'; value: number }
    | { kind: 'spawn'; program: string; args: string[] };

export type Script =
    | { kind: 'source'; value: string }
    | { kind: 'bytes'; value: number[] | Uint8Array };

export type StartOptions = { target: Target; script: Script };

export type CaptureInfo = { id: number; pid: number };

// ---------- event payload shapes ----------

export type FridgeEvent =
    | { kind: 'send'; payload: unknown }
    | {
          kind: 'log';
          level: 'info' | 'debug' | 'warning' | 'error';
          message: string;
      }
    | {
          kind: 'error';
          description: string;
          stack: string;
          file_name: string;
          line_number: number;
          column_number: number;
      }
    | { kind: 'unknown'; [k: string]: unknown };

export type EmittedEvent = {
    capture_id: number;
    event: FridgeEvent;
    data_len: number;
};

export type DetachedEvent = {
    capture_id: number;
    /** `Stopped` | `Detected` | `Error(<msg>)` (frida's DetachReason `Debug` format). */
    reason: string;
};

// ---------- per-capture callbacks (sugar over global listen) ----------

export interface CaptureCallbacks {
    onEvent?: (event: FridgeEvent, dataLen: number) => void;
    onDetached?: (reason: string) => void;
}

/** Handle returned by [`startCapture`]. */
export interface CaptureSession extends CaptureInfo {
    /** Stop the capture and unsubscribe callbacks. Idempotent. */
    stop(): Promise<void>;
}

// ---------- commands ----------

/** Compile JS to QJS/V8 bytecode via frida-core. */
export async function compileScript(source: string): Promise<Uint8Array> {
    const bytes = await invoke<number[]>('plugin:fridge|compile_script', { source });
    return new Uint8Array(bytes);
}

/**
 * Attach + run a script. Returns a session handle with a `stop()` method.
 * Pass `callbacks` to receive events for *just this capture* without the
 * `if (e.capture_id === ...)` filter dance.
 *
 * The race window between IPC start and listener registration is handled
 * internally: events emitted during `script.load()` (which can happen for
 * scripts that call `send()` synchronously in their body) are buffered and
 * replayed once we learn our `id`.
 */
export async function startCapture(
    opts: StartOptions,
    callbacks?: CaptureCallbacks,
): Promise<CaptureSession> {
    // Normalize Uint8Array → number[] for serde.
    const normalized: StartOptions = {
        ...opts,
        script:
            opts.script.kind === 'bytes' && opts.script.value instanceof Uint8Array
                ? { kind: 'bytes', value: Array.from(opts.script.value) }
                : opts.script,
    };

    let resolvedId: number | null = null;
    const buffered: EmittedEvent[] = [];
    const bufferedDetached: DetachedEvent[] = [];
    const unlisteners: UnlistenFn[] = [];

    // Subscribe BEFORE invoking — otherwise sync-`send()` from the script body
    // would fire before our listener is up.
    if (callbacks?.onEvent) {
        unlisteners.push(
            await listen<EmittedEvent>('fridge:event', (msg) => {
                if (resolvedId === null) {
                    buffered.push(msg.payload);
                } else if (msg.payload.capture_id === resolvedId) {
                    callbacks.onEvent!(msg.payload.event, msg.payload.data_len);
                }
            }),
        );
    }
    if (callbacks?.onDetached) {
        unlisteners.push(
            await listen<DetachedEvent>('fridge:detached', (msg) => {
                if (resolvedId === null) {
                    bufferedDetached.push(msg.payload);
                } else if (msg.payload.capture_id === resolvedId) {
                    callbacks.onDetached!(msg.payload.reason);
                }
            }),
        );
    }

    let info: CaptureInfo;
    try {
        info = await invoke<CaptureInfo>('plugin:fridge|start_capture', { opts: normalized });
    } catch (e) {
        for (const u of unlisteners) u();
        throw e;
    }
    resolvedId = info.id;

    // Flush anything that fired during the race window.
    if (callbacks?.onEvent) {
        for (const e of buffered) {
            if (e.capture_id === info.id) callbacks.onEvent(e.event, e.data_len);
        }
    }
    if (callbacks?.onDetached) {
        for (const d of bufferedDetached) {
            if (d.capture_id === info.id) callbacks.onDetached(d.reason);
        }
    }

    let stopped = false;
    return {
        id: info.id,
        pid: info.pid,
        async stop() {
            if (stopped) return;
            stopped = true;
            for (const u of unlisteners) u();
            await invoke('plugin:fridge|stop_capture', { id: info.id });
        },
    };
}

/** Stop a capture by id without holding the session handle. */
export async function stopCapture(id: number): Promise<void> {
    await invoke('plugin:fridge|stop_capture', { id });
}

/** List currently running captures. */
export async function listCaptures(): Promise<CaptureInfo[]> {
    return invoke<CaptureInfo[]>('plugin:fridge|list_captures');
}

// ---------- global event subscriptions (advanced use) ----------

/** Subscribe to *all* captures' `fridge:event` stream. Filter by `capture_id` yourself. */
export async function onFridgeEvent(
    cb: (e: EmittedEvent) => void,
): Promise<UnlistenFn> {
    return listen<EmittedEvent>('fridge:event', (e) => cb(e.payload));
}

/** Subscribe to *all* captures' `fridge:detached` stream. */
export async function onDetached(
    cb: (e: DetachedEvent) => void,
): Promise<UnlistenFn> {
    return listen<DetachedEvent>('fridge:detached', (e) => cb(e.payload));
}
