# Changelog

## 0.1.1 — 2026-11-02

- Updated GitHub links to vauncent/ApexControl.
- Show vauncent in the Updates panel and remove the outdated pre-launch notice.
- Simplified the README in English and added the AI disclaimer.

## 2026-11-02 — 0.1.0 development build

### Added

- Updates panel with installed version, local release notes and a link to the planned GitHub releases page. Automatic update checks and installation are not enabled yet.
- French AZERTY ISO, Nordic ISO and Japanese JIS keyboard views, alongside German ISO, US ANSI, UK ISO and Spanish ISO. Main and Shift legends were refined using GG screenshots; JIS has its own 65-key geometry.
- Graphical profile backup browser with profile name, original slot, age, format and checksum checks. A backup opens as a draft first; writing to the selected target slot requires Save to keyboard.
- Unsaved profile warnings before switching profiles/devices, refreshing devices, leaving the editor or quitting. Options: save to PC and continue, discard, or keep editing. Failed saves keep the dialog open.
- Revert changes restores the last loaded/saved draft. Local-save status is distinct from onboard-save status. Reverting a draft does not automatically undo settings already applied live.
- Breathing, rainbow wave and reactive ripple RGB modes with brightness and speed controls. These are software effects; ApexControl must remain running, including in the system tray. They are not saved onboard.
- Shift + left-drag box selection; Ctrl + Shift adds to the selection. Shift + click continues to select individual additional keys.

### Fixed

- RGB now targets only selected keys. The default apply-to-all override was removed; Select all explicitly targets the entire keyboard.
- RGB preview fills each key with its last confirmed color while keeping the legend readable. Unrelated actuation and macro markers are hidden in the lighting view.
- Repeated manual launches restore the existing window instead of creating more background instances. Simultaneous launches are guarded per Windows user/session; duplicate autostarts stay quiet.
- Windows startup registration stays visible in Task Manager after being disabled. The app reflects Task Manager changes and only changes the startup preference on an explicit toggle.
- USB status now reflects the last device scan instead of always appearing green. Refresh devices after reconnecting.
- Hardened USB response lengths, output sizes, opened-device identity checks, JSON profile validation, bounded local reads, atomic saves and macro recording limits within a single frame.

### Verified and known limits

- 48 automated tests pass. Manual testing on the user's Apex Pro Mini confirmed single-key colors, RGB modes including reactive ripple, and box selection followed by coloring.
- Reactive lighting uses transient Windows key signals from the wired Mini without text/file recording. Device remapping and Fn layers can shift the ripple origin.
- Other keyboard models, additional firmware versions and the Linux port remain unverified. Some secondary Fn/AltGr legends are not displayed. See SECURITY.md for the focused security review and outstanding dependency maintenance notices.
