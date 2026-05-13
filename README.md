# tauri-plugin-fridge

Tauri 2 plugin wrapping [`fridge`](https://github.com/jhasheng/fridge). Drop
it into a Tauri app and get four IPC commands + two event streams for
driving Frida from your frontend.

## ⚠️ Setup — required patch

This plugin transitively depends on `frida` 0.17.2, which on crates.io still
ships with unfixed UB ([frida-rust#189](https://github.com/frida/frida-rust/issues/189))
plus missing `compile_script` / `create_script_from_bytes`. The plugin's own
build redirects to a [patched fork](https://github.com/jhasheng/frida-rust/tree/fridge-fixes)
via `[patch.crates-io]`, but **that redirect only applies to the workspace
root being built** — downstream consumers have to add the same redirect in
their own workspace root.

In your Tauri app's **workspace root** `Cargo.toml` (usually `src-tauri/Cargo.toml`
if you're not using a workspace, or the workspace root if you are):

```toml
[patch.crates-io]
frida = { git = "https://github.com/jhasheng/frida-rust.git", rev = "6a92b72" }
```

Without this, builds fail with `no method named compile_script` and similar.
Once the upstream fixes land in a `frida` release, this block goes away.

## Install

`Cargo.toml` of your Tauri app:
```toml
[dependencies]
tauri-plugin-fridge = { git = "https://github.com/jhasheng/tauri-plugin-fridge" }
```

`src-tauri/src/lib.rs` (or main.rs):
```rust
tauri::Builder::default()
    .plugin(tauri_plugin_fridge::init())
    .run(tauri::generate_context!())
    .expect("run");
```

Capabilities (e.g. `src-tauri/capabilities/default.json`):
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "windows": ["*"],
  "permissions": ["fridge:default"]
}
```

Frontend `package.json`:
```json
"tauri-plugin-fridge-api": "github:jhasheng/tauri-plugin-fridge#main&path:guest-js"
```
(Or clone this repo and use a `file:` reference. npm publishing is pending
upstream `frida` patches.)

## Use

```ts
import { compileScript, startCapture } from 'tauri-plugin-fridge-api';

const bc = await compileScript(jsSource);

const session = await startCapture(
    {
        target: { kind: 'name', value: 'Weixin.exe' },
        script: { kind: 'bytes', value: bc },
    },
    {
        onEvent(event, dataLen) {
            if (event.kind === 'send') console.log('send', event.payload, dataLen);
        },
        onDetached(reason) {
            console.warn('detached:', reason);
        },
    },
);

// later
await session.stop();
```

`startCapture` returns a `{ id, pid, stop }` session. Callbacks are scoped
to this capture — events from other captures are filtered out. The race
between IPC return and listener registration is handled internally (events
fired during `script.load()` get buffered + replayed).

For multi-capture orchestration or working without a session handle, the
global subscription forms still exist:

```ts
import { onFridgeEvent, onDetached, stopCapture, listCaptures } from 'tauri-plugin-fridge-api';
const unlisten = await onFridgeEvent((e) => { /* e.capture_id, e.event, e.data_len */ });
```

## Commands

| Command                      | Args                              | Returns         |
|------------------------------|-----------------------------------|-----------------|
| `compile_script`             | `{ source: string }`              | `Uint8Array`    |
| `start_capture`              | `{ opts: StartOptions }`          | `CaptureInfo`   |
| `stop_capture`               | `{ id: number }`                  | `void`          |
| `list_captures`              | `()`                              | `CaptureInfo[]` |

## Events

- `fridge:event` — payload `{ capture_id, event: FridgeEvent, data_len }`
- `fridge:detached` — payload `{ capture_id, reason }`

## Notes

- Uses `fridge::compile_script` which self-attaches the host process briefly
  (~50ms) to use frida's runtime compiler.
- All running captures live as `CaptureHandle`s inside plugin state; the
  drop in `Drop` makes sure they tear down on app exit even if the JS side
  forgot `stop_capture`.
- Multiple concurrent captures are supported — each gets a unique `id` and
  events carry it for routing.
