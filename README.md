If you're allergic to AI, you should probably leave this repo right now — AI was involved. :)

# ApexControl — Windows Alpha

Independent, open-source control app for the **wired SteelSeries Apex Pro Mini**. Early alpha for community testing; not affiliated with SteelSeries.

**[Download Windows x64](https://github.com/vincentmittermaier-jpg/ApexControl/releases) · [Report a bug](https://github.com/vincentmittermaier-jpg/ApexControl/issues) · [Changelog](CHANGELOG.md)**

The interface supports English and German. No administrator rights or GG installation are needed for normal use. Fully exit SteelSeries GG before changing keyboard settings to prevent conflicting changes.

### Supported hardware

| Device | Status |
| --- | --- |
| Wired Apex Pro Mini, USB `1038:161E`, firmware `1.19.7` | Tested development device |
| Other firmware versions and regional hardware variants | Not fully verified |
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
