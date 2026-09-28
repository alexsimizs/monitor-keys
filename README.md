# Monitor Keys

An invisible monitor input switcher for Apple Silicon Macs. Built for one Lenovo Q24h-10 shared by two or three laptops and a Logitech MX Keys Mini keyboard.

| Key | Monitor input |
| --- | --- |
| F1 | USB-C |
| F2 | HDMI |
| F3 | DisplayPort |

Select the laptop using Logitech Easy-Switch, then press the function key for its monitor connection. On MX Keys Mini, hold **Fn** as needed to send an actual F1/F2/F3 key. The function key works while another app has focus. Easy-Switch itself is not monitored or automated.

## Install on each Mac

1. On [the repository page](https://github.com/alexsimizs/monitor-keys), choose **Code → Download ZIP**. Unzip it once. The compiled `monitor-keys` executable is included beside `Install.command`.
2. Double-click `Install.command`. It opens Terminal for the one-time installation.
3. Close Terminal when the installer reports that global shortcuts are registered.
4. Open a new Terminal tab to use the permanent `mk` command. In an already-open tab, run `source ~/.zshrc` (or your custom ZDOTDIR `.zshrc`).

The utility starts immediately and at subsequent logins. There is no application window, menu-bar item, Dock icon, or Terminal window during normal use. No Rust, Homebrew, Python, or other development tools are needed on the receiving Mac.

The package contains a native arm64 executable for the M-series architecture, without model-specific CPU optimizations. Target: macOS Tahoe 26.5.2 or newer (binary minimum 26.0). Support for every future chip, macOS release, or video connection cannot be hardware-tested in this package. DDC/CI must be enabled on the monitor. Direct USB-C, USB-C-to-DisplayPort, and HDMI paths depend on the Mac and monitor's DDC implementation; the ability to send commands through an inactive input is essential to this workflow. The user has confirmed inactive-input switching over HDMI in the reference app.

This is a locally/ad-hoc-signed personal build, not an Apple-notarized download. If macOS blocks a downloaded copy, review the app in System Settings → Privacy & Security and use Open Anyway if available. Organization-managed Macs may require IT approval. Do not disable Gatekeeper globally.

## Configure F-keys

View or change shortcuts directly from Terminal:

```sh
mk shortcuts                     # Show all three bindings
mk shortcuts set USB-C F4         # Assign F4 to USB-C
mk shortcuts set HDMI F5          # Assign F5 to HDMI
mk shortcuts set DisplayPort F6   # Assign F6 to DisplayPort
mk shortcuts set-all F3 F2 F1     # Set USB-C, HDMI, DisplayPort together
mk shortcuts reset               # Restore F1 / F2 / F3
```

Input names are case-insensitive. `mk shortcuts show` also lists the bindings.
Changes apply automatically when the installed listener is running. If the new
keys cannot be registered, the previous configuration is restored and the
listener is restarted with it. Invalid or duplicate keys are rejected before
any change. Use `set-all` to swap keys without a temporary duplicate. If the
listener is stopped, the commands save the changes and leave it stopped.
Reset changes only the keys, preserving input codes and the monitor selector.

You can also edit the file manually:

Configuration is installed at:

```text
~/Library/Application Support/Monitor Keys/config.toml
```

Edit it in a plain-text editor. Use any three different keys from F1 through F20. MX Keys Mini physically provides F1 through F12. Command/Option/Control/Shift combinations, numeric keys, and `Fn+F3` strings are rejected: enter simply `F3` because the keyboard handles Fn.

```toml
display = ""

[USB-C]
key = "F1"
value = 0x1b

[HDMI]
key = "F2"
value = 0x11

[DisplayPort]
key = "F3"
value = 0x0f
```

After manual file edits, double-click `Restart.command` or run `mk restart`. Upgrading migrates the old port names while preserving your keys, input codes, monitor selection, and comments. A pre-upgrade configuration backup is kept on the first migration. Legacy CLI names `usbc1`, `hdmi1`, and `dp1` remain accepted for existing scripts. Invalid or duplicate bindings are reported; they are never silently replaced. A failed registration is logged. For manual file edits, reassign the key or release the conflicting macOS/app shortcut, then restart. The shortcut commands perform this rollback automatically.

Leave `display` empty for the only DDC-capable external monitor. If several are attached, set an exact monitor name or unique serial printed by `displays`. The utility refuses ambiguous matches. Input values are configurable because monitors can use different codes; the defaults match the reference utility's standard codes.

All three bindings are global and reserved while the listener runs. They work in the normal logged-in desktop session; a sleeping or logged-out laptop cannot react. No Accessibility permission is requested for ordinary F-key registration. Function/media-key settings still determine whether the keyboard sends F3 or a system media action.

## Commands

`mk install` reruns setup for the installed version. For the first installation
or an upgrade, run `Install.command` from the downloaded project folder.
Alternatively, run `./monitor-keys install` in the extracted project folder.
It does not download updates.

`mk init` creates the default config only when it is missing. Existing files
are validated, not overwritten. To restore only the default key bindings, use
`mk shortcuts reset`.

`mk check` temporarily registers the keys. The running listener already owns
them, so test with `mk stop`, then `mk check`, then `mk start`. Use `mk status`
for an ordinary health check without stopping the listener. A missing display
serial is harmless when exactly one external monitor is connected.

The installer adds a managed `mk` function to your zsh configuration, so it is
available in every new Terminal tab. To enable it in a tab that was already
open during installation:

```sh
source ~/.zshrc
```

If you use a custom `ZDOTDIR`, reload its `.zshrc` instead; the installer prints
the exact path. Existing unrelated shell settings are preserved, and repeated
installation does not duplicate the managed block. A pre-existing custom `mk`
definition is left untouched and reported for manual resolution.

```sh
mk status           # Running state and registered keys
mk logs             # Recent errors and switch commands
mk restart          # Apply config changes
mk stop             # Stop until started again or next login
mk start            # Start the installed listener
mk displays         # Discover DDC-capable monitors
mk current          # Read current input, if supported
mk switch usb-c     # Select USB-C immediately
mk switch hdmi     # Select HDMI immediately
mk switch displayport       # Select DisplayPort immediately
mk config-path      # Print config location
```

To test physical key delivery without changing the monitor:

```sh
mk stop
mk listen --seconds 30
# During those 30 seconds, focus another app and press Fn+F1/F2/F3.
# Return to Terminal afterward to see the received key events.
mk start
```

`mk check` briefly registers and releases the keys. Stop the service first or it will correctly report that the keys are already occupied.

## Remove

Double-click `Uninstall.command`, or run `mk uninstall`. It stops the listener and removes its executable, login startup entry, and the managed `mk` function from your shell configuration. Existing Terminal sessions may retain the old function until you close them or run `unfunction mk`. Your configuration and logs remain available for reinstallation.

Installed locations:

- Executable/config: `~/Library/Application Support/Monitor Keys/`
- Login startup: `~/Library/LaunchAgents/local.monitor-keys.agent.plist`
- Log: `~/Library/Logs/Monitor Keys/monitor-keys.log`

You can delete the extracted project folder after installation; the running utility uses its installed copy. Download the repository again for an updated build or transfer the same ZIP to another Mac.

## Implementation and source

Source is included in `source/`. It uses the same `ddc-macos` monitor-control library as [chikacya/monitor-switch](https://github.com/chikacya/monitor-switch), and the native `global-hotkey` registration library with a documented local patch for exclusive F-key registration. There is no GUI or browser component. A native macOS event loop waits for registered keys; there is no idle monitor polling. Each key action rediscovers the monitor to handle reconnects. A separate worker serializes monitor commands, limits queued keypresses, and terminates an unresponsive command after four seconds. It sends input writes without first requiring a current-input read, which matters on inactive connections.

“Sent” means the DDC write succeeded, not that the monitor independently confirmed its physical input change. Switching back from an inactive connection still requires the monitor to accept that command. No brightness, volume, USB/KVM, or Logitech settings are changed.

The repository includes a ready-to-run executable, so building is optional. To rebuild from this repository, install Rust and Apple's Command Line Tools, then run `source/scripts/build.sh` from the repository root. It creates `monitor-keys` beside `Install.command`; run `./Install.command` to install your build. For source verification, run `cargo test --locked` inside `source/`. Third-party license notices are included in `licenses/` and `THIRD_PARTY.md`; the reference project's MIT notice is preserved in `source/LICENSE`.
