Based on global-hotkey 0.8.0 (https://github.com/tauri-apps/global-hotkey).

Local change: macOS RegisterEventHotKey now passes kEventHotKeyExclusive (1)
instead of 0. Apple's CarbonEvents.h documents that the default permits
multiple applications to register one shortcut, whereas exclusive registration
rejects an already registered combination. This makes conflicts observable and
reserves the configured function keys for Monitor Keys.

Upstream source and license notices are preserved. The upstream example
manifests are present in Cargo.toml, but examples/dev assets are not bundled;
this vendored crate is used as a library dependency only.
