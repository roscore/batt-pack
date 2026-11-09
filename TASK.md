# Task: Robot battery pack planner (Rust → WebAssembly, static web app)

Original open-source tool. Write all code yourself; do not copy anyone's code, tables or assets.
Use textbook battery arithmetic only. No vendor cell databases — presets are generic, clearly labelled
"typical" values that users can edit.

## Product (SMALL scope — keep the whole tool ~400-800 lines of Rust)
A single-page web app (GitHub Pages, sub-path `/batt-pack/`): "how many cells, in what S×P, and how long
will my robot run?"

1. **Cell + pack**: chemistry preset (Li-ion NMC: 3.6 V nominal / 4.2 full / 3.0 empty; LiFePO4: 3.2 /
   3.65 / 2.5; LiPo: 3.7 / 4.2 / 3.3), cell capacity (Ah), max continuous discharge (C), internal
   resistance (mΩ), cell mass (g); series S and parallel P; pack overhead mass % (BMS, wiring, case,
   default 15 %). Two cell presets: "generic 18650" and "generic 21700" (editable).
   Outputs: pack nominal/full/empty voltage, capacity Ah and Wh, mass, Wh/kg, max continuous current,
   pack resistance (S·R/P).
2. **Load + runtime**: average power (W), peak power (W), usable depth of discharge (default 80 %),
   converter efficiency (default 0.95). Outputs: runtime (h and min) at average load, average and peak
   current at nominal voltage, voltage sag at peak (I·R_pack) and loaded voltage at empty, I²R heat at
   peak, C-rate at peak; verdict ✅ ok / ⚠️ tight (peak current ≥ 90 % of limit or loaded empty voltage
   below empty cutoff) / ❌ over limit.
3. **Suggest S×P** (extra): user enters target voltage window (min/max nominal V) and target runtime (min);
   list up to 5 lightest S×P combos (S 1–24, P 1–10) that satisfy voltage window, runtime and peak
   current limit, sorted by mass.
4. Small extra: canvas drawing of the S×P cell grid (circles, series groups labelled).
5. Korean + English toggle (browser default). Mobile friendly.
Out of scope (do NOT build): discharge curves, temperature model, aging, URL share, JSON import/export,
copy button, dark-mode logic beyond CSS prefers-color-scheme.

## Core crate (pure Rust, no web deps, unit tested) — `crates/bp_core`
- Pack math, load/runtime, verdict, S×P search.
- Tests (hand-computable, 4–8): 4S2P 3.6 V 3 Ah → 14.4 V, 6 Ah, 86.4 Wh; mass with overhead;
  pack resistance 4·30/2 = 60 mΩ; runtime 86.4 Wh·0.8·0.95 / 50 W = 1.313 h; peak current and sag;
  verdict thresholds (ok / tight / over); S×P search results all satisfy constraints, sorted by mass,
  empty when impossible.
- `crates/bp_web`: UI. Plain web-sys — keep the bundle small.

## Repo hygiene
- README (Korean first, then English): what it is, the math and assumptions (constant nominal voltage,
  linear internal resistance, no temperature/aging), how to use, local build (`cargo test -p bp_core`,
  `trunk build --release`), MIT license, "estimates only — follow cell datasheets and use a proper BMS".
- LICENSE: MIT, copyright "batt-pack contributors". `.gitignore` for target/ dist/.
- `index.html` with a CSP meta that blocks external requests:
  `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; connect-src 'self'; object-src 'none'; base-uri 'self'`.
  In `Trunk.toml` set `filehash = false` and
  `pattern_script = "<script type=\"module\" src=\"{base}boot.js\"></script>"` with an external
  `web/boot.js` (copied via `<link data-trunk rel="copy-file">`) that imports `./bp_web.js` and calls
  init, so no inline script is needed.
- `Trunk.toml`: `public_url = "/batt-pack/"`.
- `.github/workflows/pages.yml`: on push to main + workflow_dispatch; checkout, rust stable + wasm32
  target, install trunk 0.22.1 binary release, `sh tools/privacy_check.sh`,
  `cargo test -p bp_core --release`, `trunk build --release`, configure-pages@v5,
  upload-pages-artifact@v3 (dist), deploy job with actions/deploy-pages@v4
  (permissions pages: write, id-token: write).
- `tools/privacy_check.sh` already exists; keep it and keep it clean.

## Done when
- `cargo test -p bp_core` green; `trunk build --release` succeeds; `sh tools/privacy_check.sh` clean.
- Commit everything with clear commits; open a PR. Do NOT create new GitHub repos or change repo settings.
