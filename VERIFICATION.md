# Verification — 28 September 2026

## Version 1.1.0 update

- Eight automated tests passed, including legacy-name migration, preserved custom input codes/comments, batch swaps, managed-shell-block preservation, and rollback after registration failure.
- CLI checks passed for USB-C/HDMI/DisplayPort (case-insensitive), legacy aliases, changing one key, setting all keys, restoring defaults, and rejecting invalid/duplicate keys without modifying the configuration.
- Updated the installed service and migrated its configuration to the new port names.
- Added the permanent `mk` function to `.zshrc` and verified a fresh interactive zsh session resolves it and displays the correct bindings.
- Live service tests passed for changing a key, applying it immediately, and restoring the original bindings.
- A temporary competing listener reserved different F-keys. Assigning one of those keys failed as expected; the exact original configuration was restored and the listener restarted successfully.
- Resolved a launchd shutdown race: a bootout error is tolerated only when a follow-up check confirms the job is already absent.
- Final running bindings: F1 → USB-C, F2 → HDMI, F3 → DisplayPort.
- After the original release, the user confirmed that all input shortcuts work on the physical keyboard.

## Original 1.0.0 checks

- Native arm64 release built successfully, targeting macOS 26.0+; requested deployment baseline is Tahoe 26.5.2.
- Three automated tests passed: F-key-only parsing, invalid/duplicate configuration rejection, and XML escaping for login-startup paths.
- macOS registered F1/F2/F3 globally.
- A competing test listener was rejected with an explicit shortcut conflict after enabling exclusive registration.
- Keys were available again after the temporary listener terminated.
- Detected the connected Lenovo Q24h-10 and read DisplayPort input 0x0f.
- Sent a DisplayPort input command successfully. An immediate read after that write returned a transient DDC checksum error; a later read succeeded. Switching does not require a readback.
- Installed and started the per-user login agent. Stop/start and status checks passed.
- A duplicate background instance was rejected.
- The installed listener logged two DisplayPort shortcut-triggered writes successfully. Focus state and physical keyboard sequence were not independently observed.
- The executable signature and login-agent plist validated. Linked libraries are macOS system libraries/frameworks only.

Test host: Apple Silicon, macOS 27.0. This build was not executed on Tahoe 26.5.2 during development. No claim is made that every M-series chip, sleep/wake scenario, or cable combination was hardware-tested. The user previously verified inactive-input HDMI switching with the reference app; this build's local hardware check used DisplayPort. USB-C input and real sleep/wake/reconnect behavior remain user hardware checks. Login startup is configured and the agent was started using the same mechanism, but no logout/reboot was performed.

The package is ad-hoc signed, not Developer ID signed or notarized. It includes the source and license notices.
