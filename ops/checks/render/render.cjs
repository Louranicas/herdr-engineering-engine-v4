#!/usr/bin/env node
// HEE v4 mermaid render check (ops tooling under the HOLD, V4-0; not engine code).
//
// Renders EVERY mermaid block in EVERY note of the v4 vault in headless chromium with the pinned
// mermaid build, and fails as one unit if any block does not render. The world is the vault's whole
// note tree (every *.md outside dot-directories), not a list of directories: the first harness
// scanned only `15 Module Design` and `16 System Maps` and so never saw the blocks in `50 Jev` and
// `90 Database`. The per-directory counts are printed so a shrinking world is visible.
//
// Usage (from anywhere):
//   node ops/checks/render/render.cjs [--vault DIR] [--out DIR]
//   node ops/checks/render/render.cjs --plant        negative control (see below)
//
// Exit: 0 render_fail=0 and at least one block rendered · 1 render_fail>0 (or, under --plant, the
// control tripped as intended) · 2 harness error · 3 setup refused (pins, chromium, vault) ·
// 4 plant control BROKEN (a planted fault did not produce its own diagnostic) · 30 UNMEASURED (no
// mermaid block found: an empty world is not a pass).
//
// Pins: mermaid 11.17.2 and playwright 1.62.1, exact, in package.json + package-lock.json beside
// this file. The modules live OUTSIDE the repo at $HEE4_RENDER_MODULES (default
// ~/.cache/hee4-render/node_modules), installed by `npm ci` from this directory's lock (see
// README.md for why not in-repo). Before rendering, every package the lock names is read back from
// the installed tree and must carry the locked version, or the run refuses (exit 3).
//
// --plant builds a temporary vault (nested directory, a `50 Jev`-style folder, a note with no
// mermaid) holding four blocks with known outcomes, scans it through the SAME scanner, and requires:
//   PLANT-good renders · PLANT-bad-bracket fails · PLANT-note-participant fails ·
//   PLANT-quoted-alias renders with its quotes literal (LITERAL_QUOTES).
// When all four hold it prints `render_plant control=tripped` and exits 1 (the run is red, as it
// must be). Anything else is exit 4.
'use strict';
const fs = require('fs');
const os = require('os');
const path = require('path');

const HERE = __dirname;
const DEFAULT_VAULT = process.env.HEE4_VAULT || '/mnt/storage-10tb/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault';
const MODULES = process.env.HEE4_RENDER_MODULES || path.join(os.homedir(), '.cache/hee4-render/node_modules');
const NOTE_CAP = 4096;          // notes scanned; a larger world refuses rather than truncates
const NOTE_BYTES_CAP = 4 << 20; // bytes per note read
const BLOCK_CAP = 2000;         // mermaid blocks rendered

function setupRefuse(msg) { console.log(`render_setup refused ${msg}`); process.exit(3); }

function args() {
  const a = process.argv.slice(2), o = { vault: DEFAULT_VAULT, out: null, plant: false };
  for (let i = 0; i < a.length; i++) {
    if (a[i] === '--plant') o.plant = true;
    else if (a[i] === '--vault' && a[i + 1]) o.vault = a[++i];
    else if (a[i] === '--out' && a[i + 1]) o.out = a[++i];
    else { console.error(`usage: render.cjs [--vault DIR] [--out DIR] | --plant   (unknown: ${a[i]})`); process.exit(2); }
  }
  return o;
}

// Read back every locked package version from the installed tree.
function checkPins() {
  let lock;
  try { lock = JSON.parse(fs.readFileSync(path.join(HERE, 'package-lock.json'), 'utf8')); }
  catch (e) { setupRefuse(`lock_unreadable ${e.message}`); }
  const want = JSON.parse(fs.readFileSync(path.join(HERE, 'package.json'), 'utf8')).dependencies || {};
  for (const [n, v] of Object.entries(want)) if (!/^\d+\.\d+\.\d+$/.test(v)) setupRefuse(`unpinned ${n}@${v}`);
  let checked = 0, skippedOptional = 0;
  for (const [k, meta] of Object.entries(lock.packages || {})) {
    if (!k.startsWith('node_modules/')) continue;
    const pj = path.join(MODULES, k.slice('node_modules/'.length), 'package.json');
    if (!fs.existsSync(pj)) {
      if (meta.optional) { skippedOptional++; continue; }
      setupRefuse(`missing ${k} modules=${MODULES} (run: cd ~/.cache/hee4-render && npm ci --offline --no-audit --no-fund)`);
    }
    const got = JSON.parse(fs.readFileSync(pj, 'utf8')).version;
    if (got !== meta.version) setupRefuse(`version_mismatch ${k} locked=${meta.version} installed=${got}`);
    checked++;
  }
  if (checked === 0) setupRefuse('pins_unmeasured locked_packages=0');
  for (const [n, v] of Object.entries(want)) {
    const got = JSON.parse(fs.readFileSync(path.join(MODULES, n, 'package.json'), 'utf8')).version;
    if (got !== v) setupRefuse(`version_mismatch ${n} pinned=${v} installed=${got}`);
  }
  console.log(`render_pins verdict=PASS locked=${checked} optional_absent=${skippedOptional} ` +
    Object.entries(want).map(([n, v]) => `${n}=${v}`).join(' ') + ` modules=${MODULES}`);
}

// Every *.md under root, dot-directories excluded (.obsidian, .trash, .git), bounded.
function notes(root) {
  const out = [];
  const walk = (d) => {
    for (const e of fs.readdirSync(d, { withFileTypes: true }).sort((x, y) => (x.name < y.name ? -1 : 1))) {
      if (e.name.startsWith('.')) continue;
      const p = path.join(d, e.name);
      if (e.isDirectory()) walk(p);
      else if (e.isFile() && e.name.endsWith('.md')) {
        out.push(p);
        if (out.length > NOTE_CAP) setupRefuse(`refused_note_cap notes>${NOTE_CAP} cap=${NOTE_CAP}`);
      }
    }
  };
  walk(root);
  return out;
}

// Mermaid fences: ``` or ~~~ (3+), optionally inside a blockquote/callout (`> `), info string
// exactly `mermaid`. Closed by the same fence character repeated at least as many times.
const OPEN = /^(\s*(?:>\s*)*)(`{3,}|~{3,})\s*mermaid\s*$/;
function blocks(root) {
  const out = [], perDir = {};
  const files = notes(root);
  for (const f of files) {
    const rel = path.relative(root, f);
    const top = rel.includes(path.sep) ? rel.split(path.sep)[0] : '(root)';
    perDir[top] = perDir[top] || { notes: 0, with_mermaid: 0, blocks: 0 };
    perDir[top].notes++;
    if (fs.statSync(f).size > NOTE_BYTES_CAP) setupRefuse(`refused_note_bytes ${rel} cap=${NOTE_BYTES_CAP}`);
    const lines = fs.readFileSync(f, 'utf8').split('\n');
    let here = 0;
    for (let i = 0; i < lines.length; i++) {
      const m = OPEN.exec(lines[i]);
      if (!m) continue;
      const quote = m[1], fence = m[2];
      const close = new RegExp('^\\s*(?:>\\s*)*' + (fence[0] === '`' ? '`' : '~') + `{${fence.length},}\\s*$`);
      let j = i + 1; const body = [];
      while (j < lines.length && !close.test(lines[j])) {
        body.push(quote ? lines[j].replace(/^\s*(?:>\s?)*/, '') : lines[j]);
        j++;
      }
      here++;
      out.push({ id: `${rel}:${i + 1}`, src: body.join('\n'), unterminated: j >= lines.length });
      i = j;
    }
    if (here) { perDir[top].with_mermaid++; perDir[top].blocks += here; }
  }
  if (out.length > BLOCK_CAP) setupRefuse(`refused_block_cap blocks>${BLOCK_CAP} cap=${BLOCK_CAP}`);
  return { out, perDir, notes: files.length };
}

const PLANTS = [
  { dir: 'A Nested/deeper', file: 'good.md', id: 'PLANT-good', src: 'flowchart TD\n A["x (y)"] --> B', expect: 'render' },
  { dir: '50 Jev', file: 'bad.md', id: 'PLANT-bad-bracket', src: 'flowchart TD\n A["x" --> B', expect: 'fail' },
  { dir: '50 Jev', file: 'callout.md', id: 'PLANT-note-participant', src: 'sequenceDiagram\n participant NOTE as K3 notify\n A->>NOTE: hi', expect: 'fail', callout: true },
  { dir: '90 Other', file: 'quoted.md', id: 'PLANT-quoted-alias', src: 'sequenceDiagram\n participant RT as "K6 runtime settle"\n A->>RT: hi', expect: 'quotes' },
];
function buildPlantVault() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'hee4-render-plant-'));
  for (const p of PLANTS) {
    fs.mkdirSync(path.join(root, p.dir), { recursive: true });
    const body = p.callout
      ? `# ${p.id}\n\n> [!note] callout\n> \`\`\`mermaid\n${p.src.split('\n').map((l) => '> ' + l).join('\n')}\n> \`\`\`\n`
      : `# ${p.id}\n\ntext\n\n\`\`\`mermaid\n${p.src}\n\`\`\`\n`;
    fs.writeFileSync(path.join(root, p.dir, p.file), body);
  }
  fs.writeFileSync(path.join(root, 'no-mermaid.md'), '# none\n\n```bash\necho hi\n```\n');
  fs.mkdirSync(path.join(root, '.obsidian'));
  fs.writeFileSync(path.join(root, '.obsidian', 'hidden.md'), '```mermaid\nflowchart TD\n A[ --> B\n```\n');
  return root;
}

(async () => {
  const o = args();
  checkPins();
  const vault = o.plant ? buildPlantVault() : o.vault;
  if (!fs.existsSync(vault) || !fs.statSync(vault).isDirectory()) setupRefuse(`vault_absent ${vault}`);
  const { chromium } = require(path.join(MODULES, 'playwright'));
  const exe = chromium.executablePath();
  const mm = path.join(MODULES, 'mermaid', 'dist', 'mermaid.min.js');
  if (!fs.existsSync(mm)) setupRefuse(`mermaid_dist_absent ${mm}`);
  const { out: bs, perDir, notes: nNotes } = blocks(vault);
  for (const [d, c] of Object.entries(perDir).sort()) if (c.blocks) console.log(`render_dir "${d}" notes=${c.notes} notes_with_mermaid=${c.with_mermaid} blocks=${c.blocks}`);
  if (o.out) fs.mkdirSync(o.out, { recursive: true });

  const browser = await chromium.launch();
  console.log(`render_browser chromium=${browser.version()} executable_path=${exe}`);
  const page = await browser.newPage();
  await page.setContent('<!doctype html><html><body><div id="c"></div></body></html>');
  await page.addScriptTag({ path: mm });
  await page.evaluate(() => mermaid.initialize({ startOnLoad: false, securityLevel: 'strict' }));
  let ok = 0, bad = 0, quotes = 0, n = 0;
  const result = {};
  for (const b of bs) {
    n++;
    let r;
    if (b.unterminated) r = { ok: false, err: 'unterminated mermaid fence' };
    else r = await page.evaluate(async ({ src, k }) => {
      try {
        const { svg } = await mermaid.render('m' + k, src);
        const doc = new DOMParser().parseFromString(svg, 'image/svg+xml');
        const perr = doc.getElementsByTagName('parsererror').length;
        doc.querySelectorAll('style').forEach((s) => s.remove());
        const txt = doc.documentElement.textContent || '';
        const errSvg = /Syntax error in text/.test(txt);
        const host = document.getElementById('c'); host.innerHTML = svg;
        const bb = host.querySelector('svg').getBBox(); host.innerHTML = '';
        if (!(bb.width > 0 && bb.height > 0)) return { ok: false, err: `empty bbox ${bb.width}x${bb.height}` };
        return { ok: !perr && !errSvg && svg.length > 200, svg, len: svg.length, perr, errSvg, quotes: /"/.test(txt) };
      } catch (e) {
        document.querySelectorAll('[id^="dm"]').forEach((x) => x.remove());
        return { ok: false, err: String(e.message || e).split('\n').slice(0, 3).join(' | ') };
      }
    }, { src: b.src, k: n });
    result[b.id] = r;
    if (r.ok) {
      ok++;
      if (o.out) fs.writeFileSync(path.join(o.out, `${String(n).padStart(3, '0')}.svg`), r.svg);
      if (r.quotes) { quotes++; console.log(`LITERAL_QUOTES ${b.id}`); }
    } else { bad++; console.log(`RENDER_FAIL ${b.id} :: ${r.err || JSON.stringify({ len: r.len, perr: r.perr, errSvg: r.errSvg })}`); }
  }
  await browser.close();
  console.log(`mermaid_render notes=${nNotes} blocks=${n} rendered=${ok} render_fail=${bad} literal_quotes=${quotes}`);

  if (o.plant) {
    // Map each plant to the block the scanner found in its file; a plant the scanner missed is a miss.
    const byFile = {};
    for (const id of Object.keys(result)) byFile[id.replace(/:\d+$/, '')] = result[id];
    let met = 0;
    for (const p of PLANTS) {
      const r = byFile[path.join(p.dir, p.file)];
      const hit = !!r && ((p.expect === 'render' && r.ok && !r.quotes) || (p.expect === 'fail' && !r.ok) || (p.expect === 'quotes' && r.ok && r.quotes));
      if (hit) met++;
      console.log(`render_plant ${hit ? 'ok  ' : 'MISS'} ${p.id} expect=${p.expect} got=${!r ? 'not_scanned' : r.ok ? (r.quotes ? 'rendered_literal_quotes' : 'rendered') : 'render_fail'}`);
    }
    const hiddenSkipped = !Object.keys(result).some((k) => k.startsWith('.obsidian'));
    const tripped = met === PLANTS.length && hiddenSkipped && n === PLANTS.length && bad === 2;
    fs.rmSync(vault, { recursive: true, force: true });
    console.log(`render_plant expected=${PLANTS.length} met=${met}/${PLANTS.length} blocks=${n} render_fail=${bad} dot_dirs_skipped=${hiddenSkipped ? 'yes' : 'no'} control=${tripped ? 'tripped' : 'BROKEN'}`);
    console.log(`render verdict=FAIL${tripped ? ' (as intended: plant control)' : ' plant_control=BROKEN'}`);
    process.exit(tripped ? 1 : 4);
  }
  if (n === 0) { console.log('render verdict=UNMEASURED blocks=0'); process.exit(30); }
  console.log(`render verdict=${bad ? 'FAIL' : 'PASS'} render_fail=${bad} blocks=${n}`);
  process.exit(bad ? 1 : 0);
})().catch((e) => { console.log(`render verdict=FAIL HARNESS_ERROR ${e.message}`); process.exit(2); });
