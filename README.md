If you're allergic to AI, you should probably leave this repo right now — AI was involved. :)

# ApexControl — Windows Alpha

Independent, open-source control app for the **wired SteelSeries Apex Pro Mini**. Early alpha for community testing; not affiliated with SteelSeries.

**[Download Windows x64](https://github.com/vauncent/ApexControl/releases) · [Report a bug](https://github.com/vauncent/ApexControl/issues) · [Changelog](CHANGELOG.md)**

The interface supports English and German. No administrator rights or GG installation are needed for normal use. Fully exit SteelSeries GG before changing keyboard settings to prevent conflicting changes.

### Safety warning — model and firmware compatibility

> [!CAUTION]
> **Hardware testing has only been performed on a wired SteelSeries Apex Pro Mini, USB VID:PID `1038:161E`, running firmware `1.19.7`. No other firmware version or keyboard model has been verified with physical hardware.**
>
> ApexControl is experimental software that sends HID commands and writes onboard profiles. Commands verified on this keyboard may behave differently on another firmware version or model. Incorrect or interrupted writes could corrupt settings or leave the keyboard unusable and require recovery; the risk of bricking on untested combinations cannot be ruled out. This is a precaution about unverified compatibility, not a report of an observed brick.
>
> **ApexControl does not flash or update firmware.** Profile backups, readback checks and rollback attempts reduce some risks, but cannot guarantee recovery or make an untested combination safe. Firmware `1.19.7` is the tested version, not a guarantee that every connected device is automatically checked against that version before writes. Check your exact model and firmware before changing settings. Do not assume that successful device detection means configuration is supported. Configuration for other Apex models is currently disabled; do not bypass that restriction for normal use.

### Supported hardware

| Device | Status |
| --- | --- |
| Wired Apex Pro Mini, USB `1038:161E`, firmware `1.19.7` | Only combination tested on physical hardware |
| Other firmware versions and regional hardware variants | Not verified on physical hardware |
| Apex Pro TKL, full-size, wireless and other Apex models | Configuration not supported yet |
| Linux | Port planned; no supported Linux release yet |

### Features and alpha limits

Actuation, Rapid Trigger, protection mode, five onboard profile slots, local profile drafts, automatic backups with a restore interface, key remapping, basic keyboard macros, per-key static RGB, breathing, rainbow wave and reactive ripple. Includes tray operation, single-instance startup and optional Windows autostart.

The active onboard slot cannot yet be queried reliably; startup shows the last ApexControl selection without activating a different slot. Macros currently support one basic native macro in a previously macro-free profile, up to 1000 events, without mouse actions or loops. RGB is live-only and separate from saved profiles; software effects require ApexControl to keep running. Layout previews include unverified regional variants. Profile transfers have readback checks and a rollback attempt, but an interrupted write can still require recovery. No firmware updates are provided.

The latest development build passes 48 automated tests. Hardware checks and user feedback cover the wired Mini; physical actuation depths and macro playback have not been independently measured. Please report the exact model, firmware and reproduction steps when testing. See [SECURITY.md](SECURITY.md) for safeguards and remaining limitations.

### Build from source

On Windows, install stable Rust with the MSVC toolchain and Visual Studio Build Tools with **Desktop development with C++**, including a Windows SDK. From the repository folder:

```powershell
cargo test --locked
cargo build --release --locked
```

The executable is `target/release/apexcontrol.exe`. A Windows GNU toolchain is also usable with GCC and `windres`; the local `build.ps1` helper uses the development machine's ignored `.tools` directory. The MSVC instructions are a supported build setup description, not a claim of a separately tested MSVC release binary. No private USB captures are required for ordinary builds or unit tests.

Source and original logo artwork are **GPL-3.0-or-later**. Distributed derivative versions must follow that license and provide corresponding source. Contributions and forks for additional devices are welcome.

See [LICENSE](LICENSE) for the full license and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for dependency notices.
