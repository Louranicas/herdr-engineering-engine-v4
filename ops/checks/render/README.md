# Mermaid render check (`just render`)

Ops tooling under the HOLD (V4-0). It is not engine code.

`render.cjs` renders **every mermaid block in every note of the v4 vault** in headless chromium, using the pinned mermaid build. It exits 0 only when `render_fail=0` and at least one block was rendered.

## What it scans
- **The world:** every `*.md` under the vault. Dot-directories (`.obsidian`, `.trash`) are excluded.
- **This is not a list of directories.** The first harness scanned only `15 Module Design` and `16 System Maps`, so it never saw the blocks in `50 Jev` or `90 Database`.
- **Per-directory counts:** it prints `render_dir "<dir>" notes= notes_with_mermaid= blocks=`, so a shrinking world is visible.
- **Fences it reads:**
  - backtick or tilde fences, three or more long, with the info string exactly `mermaid`;
  - fences inside a blockquote or callout (`> `), with the quote prefix stripped from the body;
  - an unterminated fence, which counts as a render failure.

2026-10-01 result: `notes=38 blocks=113 rendered=113 render_fail=0`. The blocks were spread across 15 Module Design (77), 16 System Maps (32), 50 Jev (3) and 90 Database (1).

## What counts as rendered
A block counts as rendered when all of these hold:
- `mermaid.render` returns an SVG longer than 200 bytes;
- the SVG has no XML `parsererror`;
- it has no "Syntax error in text";
- it has a non-empty bounding box.

A rendered label that shows literal `"` characters is printed as `LITERAL_QUOTES <note>:<line>` and counted in `literal_quotes=N`. It is a warning, not a failure: a quote in a label can be intended.

## The plant control (`just render plant`, or `node render.cjs --plant`)
`--plant` builds a temporary vault and runs it through the same scanner. The vault contains:
- a nested directory;
- a `50 Jev` folder;
- a callout block;
- a note with no mermaid;
- a broken block hidden in `.obsidian/`, which must be skipped.

It requires all of the following:
- `PLANT-good` renders;
- `PLANT-bad-bracket` fails;
- `PLANT-note-participant` fails (a participant named `NOTE`, inside a callout);
- `PLANT-quoted-alias` renders with literal quotes.

When every plant gives its expected result, it prints `render_plant ... control=tripped` and `render verdict=FAIL (as intended: plant control)`, and exits **1**. That run is red, as it must be. Any other result exits **4** (`control=BROKEN`).

2026-10-01 result: `met=4/4 blocks=4 render_fail=2 dot_dirs_skipped=yes control=tripped`, rc=1.

## Exit codes
| Code | Meaning |
|---|---|
| 0 | PASS |
| 1 | `render_fail>0`, or the plant control tripped as intended |
| 2 | harness error |
| 3 | setup refused: pins, modules, vault, or a note or block cap |
| 4 | the plant control is BROKEN |
| 30 | UNMEASURED: no mermaid block was found |

## Pins and where they live
- **The pin:** `package.json` here pins `mermaid` 11.17.2 and `playwright` 1.62.1 exactly. `package-lock.json` pins all 115 packages, plus one optional package that is absent on Linux.
- **Read-back:** before rendering, `render.cjs` reads every locked package's installed `package.json`. Any version mismatch refuses the run (exit 3, `render_setup refused version_mismatch …`). On success it prints `render_pins verdict=PASS locked=115 …`.
- **The modules live outside the repo,** at `~/.cache/hee4-render/node_modules` (override: `HEE4_RENDER_MODULES`). There is a measured reason for this. On 2026-10-01 an in-repo `ops/checks/render/node_modules` (172 MB, 9,523 files) did two things:
  - it slowed `module_funnel.py --control` from 8 s to 38 s, because the control copies the whole repo into tmpfs once per case;
  - it put 166 third-party `.md` files into the funnel's `cite_outside_world` scan.

  Only `package.json` and `package-lock.json` are in the repo.
- **Install, or reinstall after a lock change.** No network is needed: on 2026-10-01 every package was already in the npm cache, and `npm install --offline` and `npm ci --offline` both succeeded.
  ```bash
  mkdir -p ~/.cache/hee4-render
  cp ops/checks/render/package.json ops/checks/render/package-lock.json ~/.cache/hee4-render/
  (cd ~/.cache/hee4-render && npm ci --offline --no-audit --no-fund)
  ```
- **Chromium** comes from the playwright browser cache that was already present. For playwright 1.62.1 (browsers.json revision 1234) that is:
  - `~/.cache/ms-playwright/chromium-1234/chrome-linux64/chrome` (`executable_path`, Chrome for Testing 151.0.7922.34);
  - `~/.cache/ms-playwright/chromium_headless_shell-1234/`.

  The run prints the browser version and path it launched.

## Provenance
This file was moved on 2026-10-01 from the batchC scratch harness (`render.cjs`). The changes were:
- the hard-coded `~/.npm/_npx/<hash>/node_modules/playwright` path was replaced by the pinned install;
- the scan world became the whole vault;
- the pin read-back was added;
- the plant control now asserts each plant's outcome.

Runbook: `runbooks/render.toml`.
