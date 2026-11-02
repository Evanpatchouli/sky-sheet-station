# Audio component notices

SkySheet includes the following unmodified dependencies/model. Preserve this
directory when distributing the CLI; Tauri bundles it as `audio-notices`.

- Basic Pitch model, copyright 2022 Spotify AB, and basic-pitch-rs, copyright
  2026 Adrien Thevon: Apache License 2.0. Full license and upstream NOTICE are
  reproduced here. The NOTICE describes upstream provenance; SkySheet does not
  install or run the Python packages named in that historical notice.
- tract: MIT OR Apache-2.0, distributed under the Apache-2.0 option here.
  Source: https://github.com/sonos/tract/tree/v0.23.9
- Symphonia 0.5.5 and its codec/core components: Mozilla Public License 2.0.
  Corresponding unmodified source is available at
  https://github.com/pdeljanov/Symphonia/tree/v0.5.5 and crates.io version 0.5.5.
  The full license is included. No Symphonia source modifications are made.
- Rubato 0.16.2: MIT; full copyright/license notice included.

The model is embedded by the pinned dependency, not downloaded at runtime.
Its SHA-256 is `2c3c1d144bfa61ad236e92e169c13535c880469a12a047d4e73451f2c059a0ec`.
The Cargo lockfile identifies transitive dependencies; their original licenses
continue to apply. No upstream audio test recordings are copied into this tree.
