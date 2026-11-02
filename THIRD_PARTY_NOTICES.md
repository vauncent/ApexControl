# Third-party notices

ApexControl uses open-source Rust crates, including eframe/egui (MIT OR Apache-2.0), hidapi (MIT), serde and serde_json (MIT OR Apache-2.0), tray-icon/muda, winreg, and their dependencies. `Cargo.lock` records the exact resolved versions. The release license directory includes the license texts and package metadata collected from the local crate registry for the Windows dependency graph.

The Windows GNU build uses the MinGW-w64 runtime. Its notices are included as `COPYING.MinGW-w64-runtime.txt` in the release license directory.

Windows icon resources are compiled using the open-source winresource build helper; its package notices are included with the other dependencies. The ApexControl logo artwork in `assets/` was created for this project and does not reuse SteelSeries logo artwork.

On Windows the UI loads the system-installed Segoe UI font; no Microsoft font file is redistributed. egui's embedded fallback fonts retain their upstream notices in the dependency license directory.

Protocol research consulted publicly available documentation at https://github.com/Rikearon/apex-control and https://github.com/Ven0m0/steelseriesgg-rs. The Mini implementation was checked against local device captures. No vendor software, user configuration, USB capture or keyboard firmware is included in the release.
