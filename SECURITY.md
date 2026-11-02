# Security review and reporting

ApexControl is an early development release. This document records a focused
review on 2026-11-02; it is not a security certification or a claim that all
vulnerabilities have been found. Only the wired Apex Pro Mini protocol has been
checked against local hardware and captured SteelSeries GG traffic.

## Findings, comparison and feature impact

| Area | Finding and change | Comparison | Feature impact |
| --- | --- | --- | --- |
| USB input | Feature responses previously retained the unused zero-filled tail of the receive buffer. A short successful-looking response could be treated as a full profile chunk. The parser now keeps the actual length, rejects missing report headers, and requires all 500 chunk bytes before accepting a profile read. Missing live-command subcodes fail safely. | OpenRazer's keyboard driver checks input sizes and matches response command IDs/classes against the request. This is the same defensive approach, not evidence of the same bug in OpenRazer. | Complete existing replies and generated command bytes are unchanged. |
| USB output | Oversized internal Output reports could panic on slicing. Length is checked before copying; partial writes are errors. | OpenRazer rejects unexpected command payload lengths. | All existing commands fit the same report sizes. |
| Device selection | Worker commands previously trusted a path from an earlier scan. The opened handle is now rechecked for vendor, Mini product ID, vendor usage page and usage. | OpenRazer dispatches device-specific operations by supported model. | No additional model or firmware lock. USB IDs are not cryptographic authentication; a malicious physical USB device can spoof them. |
| Local JSON | Unbounded reads could consume excess memory. Settings now allow 64 KiB; individual profiles/session files 1 MiB; the macro library 32 MiB. Reads also enforce the limit if a file grows and reject links/non-regular files. Only `.json` files are considered in the profile directory. | General parser hardening; no equivalent OpenRazer/other-app size protection was established in this review. | Files produced within these limits remain compatible, including pretty-printed profiles. An individual macro retains the existing 1000-event editor limit. Oversized/corrupt inputs are ignored by the existing load fallbacks; the files themselves are not deleted. These are explicit resource ceilings, not a guarantee against all local denial of service (for example, an enormous number of small profile files). |
| Profile construction | A derived JSON deserializer could create unchecked `Profile` objects. JSON now goes through the same exact size, schema and CRC checks as device reads. | Comparable boundary validation; no claim about SteelSeries GG's private parser. | Unknown vendor fields, existing bindings, native macro blocks and trailing metadata remain byte-preserved. CRC detects corruption; it does not establish trust or validate every firmware-specific field. |
| Local saves | Predictable `.tmp` files could be preplaced as links; macro saving truncated the destination directly. Saves now use exclusive temporary files in the destination directory, flush before rename, and leave the old destination intact on a failed write. Macro entries enter the in-memory library only after successful persistence. Backups use this path before profile erasure. | General safe file handling, not a verified corresponding bug in another app. | File formats and save/overwrite behavior stay compatible. An error is reported when the output exceeds its load limit. A same-user attacker who controls the parent directory is outside the protection of these file checks. |
| Macro recording | The event limit was checked only after processing an entire GUI frame. A large event batch could exceed the existing device/editor limit. Recording now reserves space for all key-release events before accepting each input. | General bounded event processing. | Existing 1000-event limit is retained; modifiers, F1–F24, ordinary keys and pauses remain available. Partial macros may still be saved as editable local drafts. |

The reviewed comparison sources are
[OpenRazer's keyboard driver](https://github.com/openrazer/openrazer/blob/master/driver/razerkbd_driver.c),
[OpenRazer's macro implementation](https://github.com/openrazer/openrazer/blob/master/daemon/openrazer_daemon/misc/macro.py),
and [ckb-next's profile loader](https://github.com/ckb-next/ckb-next/blob/master/src/gui/kbprofile.cpp).
The latter demonstrates external profile handling, not a proof of parser safety.
The reviewed OpenRazer macro code supports key, URL and script actions; this does
not mean those are vulnerabilities. ApexControl currently supports keyboard
macro events, not script execution. SteelSeries GG's implementation was not
audited, and no equivalence in internal validation is claimed.

## Intentional compatibility choices and remaining risks

- Macro key combinations are not blacklisted. A valid macro can type unwanted
  input when activated. Inspect profiles/macros from other people before saving
  them to a device. Loading a local draft does not activate it or execute a macro.
- Unknown GG profile fields are preserved. Rejecting everything not understood
  would discard legitimate vendor features. Full semantic validation of the
  opaque native format remains incomplete; well-formed data with a recomputed
  CRC is not necessarily safe or appropriate for every firmware.
- Firmware 1.19.7 is the tested version, not an authentication mechanism. No
  blanket firmware-version ban was added. Other versions remain unverified.
- Do not run the normal app as administrator/root. It uses ordinary user HID
  access on Windows. A hostile process already running as the same user can
  modify that user's files or access the keyboard independently of ApexControl.
- Backups/readback and attempted rollback do not guarantee recovery after power
  loss, USB removal, simultaneous GG writes or a failing/malicious device.
- The UI and storage tests do not prove USB-driver, GUI-library or firmware
  safety. Device I/O may block in the underlying OS/driver.

## Verification

- 28 unit/UI/storage regression tests pass on Windows, including a single-frame
  batch of 4000 macro key events, truncated USB responses, checked JSON profile
  construction, bounded streams and failed/atomic saves.
- The release executable still matches six captured GG live reports and a full
  native macro profile byte for byte; all five reference profiles pass CRC checks.
- A read-only hardware test on the connected Mini (firmware 1.19.7) successfully
  reads all five profiles, region and physical layout using the new response
  length and opened-device checks. The original profile is unchanged.
- No new persistent write, macro playback or RGB write test was performed during
  this review. Previously verified functionality is covered by the retained
  protocol/reference and UI tests; this is not a claim of complete GG parity.

## Dependency check

`tools/audit_dependencies.py` (Python 3.11+) queries OSV using the public crates.io
package names and versions from `Cargo.lock`. It sends no source code, profile
contents or credentials. Run from the project directory:

```sh
python tools/audit_dependencies.py --report local-data/dependency-security.json
```

The 2026-11-02 check covered 375 locked crates.io package versions, including
platform-specific dependencies. It returned these maintenance notices:

- `ttf-parser 0.25.1`: RUSTSEC-2026-0192 (unmaintained).
- `zbus_names 4.3.5`: RUSTSEC-2026-0371 (merged into a newer upstream project).
- `zvariant 5.15.1`: RUSTSEC-2026-0370 (unmaintained standalone crate).
- `zvariant_derive 5.15.1`: RUSTSEC-2026-0372 (unmaintained standalone crate).

No non-informational vulnerability match was returned by this check. These
maintenance warnings remain open dependency migration work, and must not be
represented as a clean bill of health. The script exits nonzero on any advisory
match, including maintenance notices, and fails on network errors.

## Reactive lighting input

Reactive RGB uses a message hook in this application's existing winit event loop,
always passing native messages through to winit. Background Raw Input is enabled
only for an explicitly started reactive effect and restores the previous target
and flags on stop (retaining winit's device notification setting). It does not
register a competing keyboard window or suppress legacy input. Device names are
used transiently to filter the wired Mini's VID/PID. Events are converted to
bounded, short-lived lighting impulses, never to text or an input log. This is
not an authentication boundary: physical devices can spoof USB identifiers.
Both the input queue and effect history are bounded, and the preview retains
only one frame while the window is hidden. Effects run through the serial HID
worker, not concurrently with profile writes. No new network service is added.

The registration behavior is covered by a native test on an owned test window.
On 2026-11-02 the user confirmed working single-key colors, reactive RGB modes
and box selection with coloring on the Mini. This is practical user feedback,
not an exhaustive audit of physical input behavior or other firmware versions.
See [Microsoft Raw Input](https://learn.microsoft.com/en-us/windows/win32/inputdev/using-raw-input)
and [registration rules](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerrawinputdevices).

## Reporting vulnerabilities

Until a private vulnerability-reporting channel is configured on the public
repository, do not post credentials, personal profiles or weaponized payloads
in public issues. A general bug report can describe the affected version,
component and impact without publishing sensitive material.
