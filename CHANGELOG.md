# Changelog

## 1.1.0

- Rename user-facing inputs to USB-C, HDMI, and DisplayPort.
- Install a permanent `mk` command for zsh.
- Add commands to show, change, swap, and reset F-key shortcuts.
- Apply shortcut changes immediately and restore the previous configuration if registration fails.
- Migrate older configuration names while preserving custom input codes, keys, and comments.
- Preserve unrelated shell settings and remove only the managed shell function during uninstall.
- Handle a launchd shutdown race when an agent has already stopped.

## 1.0.0

- Add global F-key switching for three monitor video inputs.
- Run without a window, Dock icon, or menu-bar item.
- Support per-user installation and login startup on Apple Silicon Macs.
- Add monitor discovery, input switching, diagnostics, and configuration commands.
- Detect shortcut conflicts using exclusive macOS hotkey registration.
