#!/usr/bin/env python3
"""Negative control for hee4db. Works on temp copies only; the real DB and sources are never written.

Each clause plants one fault and requires (a) the expected exit code and (b) the diagnostic token that
only that refusal site prints. The needle never comes from the plant itself: e.g. the stale clause edits
a sha in the DB and requires `stale_source`, a token the planted SQL does not contain.
Quiet cases run the same verbs on the clean copy and require no refusal.

Prints one line per case, then `clauses=N/N quiet=Q/Q real_db_unchanged=yes verdict=PASS|FAIL`.
Exit 0 on PASS, 20 on FAIL, 3 on setup failure.
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import sqlite3
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
HEE4DB = Path(os.environ.get("HEE4DB_BIN", HERE.parent / "hee4db"))
HOME = Path.home()
TURSODB = os.environ.get("HEE4DB_TURSODB", str(HOME / ".local/bin/tursodb"))
REAL_DB = Path(os.environ.get("HEE4DB_REAL_DB", HOME / "hee4-evidence/db/hee4-ops.db"))
REAL = {
    "repo": Path(os.environ.get("HEE4_ROOT", HOME / "herdr-engineering-engine-v4")),
    "evidence": Path(os.environ.get("HEE4_EVIDENCE", HOME / "hee4-evidence")),
    "vault": Path(os.environ.get("HEE4_VAULT", "/mnt/storage-10tb/fedora-obsidian-vaults/herdr-engineering-engine-v4.vault")),
    "claude": HOME / ".claude",          # rev 2026-10-01 registry: read only, copied into the case world
}
WORLD = {  # every file the ingest world reads (copied, never linked; a matched directory is created empty)
    "repo": ["plan/DECISIONS.md", "docs/*.md", "modules/MODULES.toml", "modules/*/*/MODULE.md",
             ".claude/agents/*.md", ".claude/skills/*/SKILL.md", ".claude/skills/*/reference", ".claude/settings.json",
             ".claude/hooks/*.sh", "ops/roster/*/modes.conf",
             "crates/**/*.rs"],   # readiness counts .rs files (the "is there code" fact); since V4-89 the repo has code
    "evidence": ["design/*.md", "learnings/PROCESS-LEARNINGS.md", "verification/V*.md"],
    "vault": ["15 Module Design/*.md", "16 System Maps/*.md", "00 Hub/Module Readiness 2026-10-01.md", "50 Jev/Jev Fit Map.md"],
    "claude": ["skills/*/SKILL.md", "skills/*/reference", "hooks/*.sh", "settings.json"],
}
ENV_OF = {"claude": "HEE4DB_CLAUDE_HOME"}   # other roots are HEE4DB_<ROOT>
HABITAT_DB = HOME / "firstmate/data/habitat-ops.db"

# The schedule fixture (rev 2026-10-05 schedule). The control plants its own precondition instead of depending on
# the host: it never reads the host crontab (this host has none) or the host's systemd user unit directory. Every case world gets a
# synthetic crontab in the shape of hee4-evidence/roster/crontab.txt, a planted agent definition + modes.conf, and a
# timers directory (one daily timer, one roster timer) reached through the HEE4DB_TIMERS_DIR seam. The command paths
# are the same in every world so a case world's bytes equal the frozen baseline's (a differing byte reads as stale).
PLANTED = "hee4-planted"
RUNNER = REAL["repo"] / "ops/roster/run-agent.sh"
DAILY = REAL["repo"] / "ops/db/daily.sh"
CRONTAB_FIXTURE = (
    "# HEE v4 agent roster: the control's synthetic crontab (never the host's), in the shape of hee4-evidence/roster/crontab.txt\n"
    "XDG_RUNTIME_DIR=/run/user/1000\n"
    f"17 */6 * * * /usr/bin/bash {RUNNER} {PLANTED} light\n"
    f"40 2 * * *   /usr/bin/bash {RUNNER} {PLANTED} deep\n"
)
AGENT_FIXTURE = f"---\nname: {PLANTED}\ndescription: planted control agent\nmodel: sonnet\ntools: Read, Bash\n---\nA planted agent definition; the control's fixture, never run.\n"
MODES_FIXTURE = "light|1.00|planted\ndeep|4.00|planted\n"


def timer_unit(on_calendar: str, service: str) -> str:
    return f"[Unit]\nDescription=planted control timer\n\n[Timer]\nOnCalendar={on_calendar}\nPersistent=true\nUnit={service}\n\n[Install]\nWantedBy=timers.target\n"


def service_unit(exec_start: str) -> str:
    return f"[Unit]\nDescription=planted control service\n\n[Service]\nType=oneshot\nExecStart={exec_start}\n"


HOOKS_FIXTURE = {
    "jev-recall-guard.sh": "#!/bin/bash\n# planted control hook: a Jev sender by name (the jev_sender rule), unwired\nexit 0\n",
    "pipe-verdict-guard.sh": "#!/bin/bash\n# planted control hook: not a Jev sender, unwired\nexit 0\n",
}
TIMERS_FIXTURE = {
    f"{PLANTED}-daily.timer": timer_unit("*-*-* 02:30:00", f"{PLANTED}-daily.service"),
    f"{PLANTED}-daily.service": service_unit(f"/usr/bin/bash {DAILY}"),
    f"{PLANTED}-roster.timer": timer_unit("*-*-* 03:10:00", f"{PLANTED}-roster.service"),
    f"{PLANTED}-roster.service": service_unit(f"/usr/bin/bash {RUNNER} {PLANTED} light"),
}


def sha(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest() if p.exists() else "absent"


class World:
    def __init__(self, base: Path, name: str, roots: dict[str, Path] | None = None):
        roots = roots or REAL
        self.root = base / name
        self.root.mkdir()
        self.env = dict(os.environ)
        for k, rel_globs in WORLD.items():
            dst = self.root / k
            for pat in rel_globs:
                for src in sorted(roots[k].glob(pat)):
                    d = dst / src.relative_to(roots[k])
                    if src.is_dir():
                        d.mkdir(parents=True, exist_ok=True)
                        continue
                    d.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(src, d)
            dst.mkdir(parents=True, exist_ok=True)
            self.env[ENV_OF.get(k, f"HEE4DB_{k.upper()}")] = str(dst)
        # the schedule fixture: written into every world (the frozen baseline carries the agent and modes files
        # under repo/, the crontab and timers are re-planted per world; identical bytes everywhere)
        (self.root / "crontab.txt").write_text(CRONTAB_FIXTURE)
        self.env["HEE4DB_CRONTAB_FILE"] = str(self.root / "crontab.txt")
        agent = self.root / "repo/.claude/agents" / f"{PLANTED}.md"
        agent.parent.mkdir(parents=True, exist_ok=True)
        agent.write_text(AGENT_FIXTURE)
        modes = self.root / "repo/ops/roster" / PLANTED / "modes.conf"
        modes.parent.mkdir(parents=True, exist_ok=True)
        modes.write_text(MODES_FIXTURE)
        timers = self.root / "timers"
        timers.mkdir()
        for name, text in TIMERS_FIXTURE.items():
            (timers / name).write_text(text)
        self.env["HEE4DB_TIMERS_DIR"] = str(timers)
        # host preconditions the registry and restart cases need, planted rather than assumed (this host has no
        # ~/.claude/hooks and no HEE4_HANDOVER_* note): two unwired hook scripts (one a Jev sender by name) and a handover
        hooks = self.root / "claude/hooks"
        hooks.mkdir(parents=True, exist_ok=True)
        for name, text in HOOKS_FIXTURE.items():
            (hooks / name).write_text(text)
        # the skill the v4 rule must EXCLUDE (its killer case reads it as absent from `recipe skills`); not installed on
        # this host, so the world carries a fixture copy, never the real skill
        v3_skill = self.root / "claude/skills/hee-v3-corpus/SKILL.md"
        v3_skill.parent.mkdir(parents=True, exist_ok=True)
        v3_skill.write_text("---\nname: hee-v3-corpus\ndescription: planted control fixture, the frozen corpus skill the v4 rule excludes\n---\nPlanted. Never loaded.\n")
        handoffs = self.root / "handoffs"
        handoffs.mkdir()
        (handoffs / "HEE4_HANDOVER_planted.md").write_text("# HEE4 handover (planted control fixture)\n\nverdict=PASS planted\n")
        (handoffs / "HEE4_RESTART.md").write_text("# HEE4 restart (planted control fixture)\n")
        self.env["HEE4DB_HANDOFFS"] = str(handoffs)
        self.db = self.root / "db" / "hee4-ops.db"
        self.db.parent.mkdir()
        self.env["HEE4DB_PATH"] = str(self.db)
        self.env["HEE4DB_HABITAT_DB"] = str(base / "habitat-snap.db")
        self.env.pop("HEE4DB_JEV_STATE", None)

    def copy_db_from(self, snap: Path) -> None:
        shutil.copyfile(snap, self.db)
        side = snap.with_name(snap.stem + "-search.db")
        if side.exists():
            shutil.copyfile(side, self.db.with_name("hee4-ops-search.db"))

    def run(self, *args: str) -> tuple[int, str]:
        p = subprocess.run([sys.executable, str(HEE4DB), *args], env=self.env, capture_output=True, text=True, timeout=300)
        return p.returncode, p.stdout + p.stderr

    def sql(self, stmt: str, fk: bool = False) -> None:
        con = sqlite3.connect(str(self.db))
        con.execute(f"PRAGMA foreign_keys={'ON' if fk else 'OFF'}")
        con.execute(stmt)
        con.commit()
        con.close()

    def count(self, table: str) -> int:
        con = sqlite3.connect(f"file:{self.db}?mode=ro", uri=True)
        n = con.execute(f'SELECT count(*) FROM "{table}"').fetchone()[0]
        con.close()
        return n


def corrupt_table_page(db: Path, table: str) -> None:
    con = sqlite3.connect(str(db))
    con.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    root, = con.execute("SELECT rootpage FROM sqlite_schema WHERE name=?", (table,)).fetchone()
    psz, = con.execute("PRAGMA page_size").fetchone()
    con.close()
    with open(db, "r+b") as fh:
        fh.seek((root - 1) * psz + 8)          # past the b-tree page header: cell pointer array
        fh.write(b"\xff" * 32)
        fh.flush()
        os.fsync(fh.fileno())


def main() -> int:
    if not REAL_DB.exists():
        print(f"setup: real DB {REAL_DB} absent; build it first")
        return 3
    real_before = {p: sha(p) for p in [REAL_DB, REAL_DB.with_name("hee4-ops-search.db"), HABITAT_DB]}
    base = Path(tempfile.mkdtemp(prefix="hee4db-control-"))
    results: list[tuple[str, str, bool, str]] = []
    try:
        snap = base / "baseline.db"
        # snapshot through tursodb --readonly (works beside a live writer; copies no -wal frames by accident)
        r = subprocess.run([TURSODB, "-q", "--readonly", str(REAL_DB), f"VACUUM INTO '{snap}'"],
                           stdin=subprocess.DEVNULL, capture_output=True, text=True)
        hs = subprocess.run([TURSODB, "-q", "--readonly", str(HABITAT_DB), f"VACUUM INTO '{base / 'habitat-snap.db'}'"],
                            stdin=subprocess.DEVNULL, capture_output=True, text=True)
        if r.returncode != 0 or hs.returncode != 0:
            print(f"setup: snapshot failed rc={r.returncode}/{hs.returncode} {r.stderr[:200]} {hs.stderr[:200]}")
            return 3
        # Freeze ONE world: copy the live sources once, re-ingest that copy into the snapshot, and derive
        # every case from it. The live sources are being edited by other agents; a case world copied
        # later than the snapshot would read as stale for reasons the control did not plant.
        bw = World(base, "baseline")
        shutil.copyfile(snap, bw.db)
        mrc, mout = bw.run("migrate")   # a snapshot of an older DB gets the newer migrations first
        if mrc != 0:
            print(f"setup: baseline migrate rc={mrc} {mout[-300:]}")
            return 3
        brc, bout = bw.run("ingest")
        if brc != 0:
            print(f"setup: baseline ingest rc={brc} {bout[-300:]}")
            return 3
        snap = bw.db
        frozen = {k: bw.root / k for k in WORLD}
        n = 0

        def case(name: str, kind: str, setup, args: list[str], want_rc: int, needles: list[str], post=None, cleanup=None) -> None:
            nonlocal n
            n += 1
            w = World(base, f"c{n:02d}", frozen)
            w.copy_db_from(snap)
            try:
                try:
                    pre = setup(w) if setup else None
                except Exception as e:  # a setup failure is a recorded miss, never a crash of the control
                    results.append((kind, name, False, f"setup_error {type(e).__name__}: {str(e)[:200]}"))
                    return
                rc, out = w.run(*args)
                ok = rc == want_rc and all(x in out for x in needles)
                why = f"rc={rc} want={want_rc}"
                missing = [x for x in needles if x not in out]
                if missing:
                    why += f" missing={missing}"
                if ok and post:
                    pok, pwhy = post(w, pre)
                    ok = ok and pok
                    why += " " + pwhy
                results.append((kind, name, ok, why if not ok else why + " needles=" + ",".join(needles)))
            finally:
                if cleanup:   # undoes a plant that would block the temp dir's removal (a chmod 000), pass or fail
                    cleanup(w)

        # ── quiet cases (clean copy) ──
        case("quiet-stale", "quiet", None, ["stale"], 0, ["stale_sources=0", "habitat_stale=0"])
        case("quiet-q", "quiet", None, ["q", "SELECT id, verdict FROM verifications"], 0, ["verdict=PASS"])
        case("quiet-search", "quiet", None, ["search", "verdict", "authority"], 0, ["engine=fts"])
        case("quiet-ingest", "quiet", None, ["ingest"], 0, ["changed=0", "unexplained=0"])
        case("quiet-recipes", "quiet", None, ["recipe", "blocks-phase", "P0"], 0, ["verdict=PASS"])

        def red_checks(out: str) -> list[str]:   # the check lines sit inside the JSON doc, so match the token, not a line start
            return sorted({m.group(0) for m in re.finditer(r"check=\w+ verdict=(?:FAIL|UNMEASURED)", out)})

        def quiet_check(w):
            rc, out = w.run("check")
            bad = red_checks(out)
            return rc in (0, 10) and not bad, f"rc={rc} red_lines={len(bad)}" + (f" red={','.join(bad)}" if bad else "")
        n += 1
        w = World(base, f"c{n:02d}", frozen)
        w.copy_db_from(snap)
        qok, qwhy = quiet_check(w)
        results.append(("quiet", "quiet-check", qok, qwhy))

        # ── fault clauses ──
        case("stale-source-sha", "fault",
             lambda w: w.sql("UPDATE sources SET sha256 = replace(sha256, substr(sha256,1,1), 'x') WHERE rel_path='plan/DECISIONS.md'"),
             ["stale"], 20, ["stale_source source=repo:plan/DECISIONS.md"])
        case("stale-source-in-check", "fault",
             lambda w: (w.root / "repo/docs/ANTIPATTERNS.md").write_text((w.root / "repo/docs/ANTIPATTERNS.md").read_text() + "\n"),
             ["check"], 20, ["check=stale verdict=FAIL", "stale_source=repo:docs/ANTIPATTERNS.md"])
        case("stale-source-absent", "fault",
             lambda w: (w.root / "repo/modules/MODULES.toml").unlink(),
             ["stale"], 20, ["stale_source_absent source=repo:modules/MODULES.toml"])
        case("stale-world-new-file", "fault",
             lambda w: (w.root / "vault/16 System Maps/Planted Map.md").write_text("# Planted Map\n"),
             ["stale"], 20, ["stale_world_new_file source=vault:16 System Maps/Planted Map.md"])
        case("unbounded-rows", "fault", None, ["q", "SELECT module, ref FROM module_refs"], 20,
             ["refused_row_cap", "cap=200"])
        case("unbounded-bytes", "fault", None, ["q", "SELECT body FROM design_notes"], 20, ["refused_byte_cap", "cap=262144"])
        case("unbounded-steps", "fault", None,
             ["q", "SELECT count(*) FROM module_refs a, module_refs b, module_refs c"], 20, ["refused_step_budget"])

        def unchanged(table):
            def post(w, pre):
                after = w.count(table)
                return after == pre, f"{table}_rows={pre}->{after}"
            return post
        case("write-through-q", "fault", lambda w: w.count("decisions"), ["q", "DELETE FROM decisions"], 20,
             ["refused_not_read_only"], unchanged("decisions"))
        case("write-through-q-turso", "fault", lambda w: w.count("decisions"),
             ["q", "--engine", "turso", "DELETE FROM decisions"], 20, ["refused_not_read_only"], unchanged("decisions"))
        case("multi-statement-q", "fault", lambda w: w.count("decisions"),
             ["q", "SELECT 1; DELETE FROM decisions"], 20, ["refused_multi_statement"], unchanged("decisions"))

        def empty_world(w):
            w.db.unlink()
            for s in ("-wal", "-shm"):
                Path(str(w.db) + s).unlink(missing_ok=True)
            rc, out = w.run("migrate")
            assert rc == 0, out
        case("empty-world-q", "fault", empty_world, ["q", "SELECT id FROM decisions"], 30, ["unmeasured_empty_world"])
        case("empty-world-check", "fault", empty_world, ["check"], 30, ["check=world verdict=UNMEASURED", "check=stale verdict=UNMEASURED"])
        case("empty-world-search", "fault", empty_world, ["search", "door"], 30, ["unmeasured_empty_world corpus_rows=0"])

        def malformed(w):
            p = w.root / "repo/docs/ANTIPATTERNS.md"
            p.write_text(p.read_text().replace("| ID | Name | Definition | The tell | Detector / TRIGGER | Record | Source |",
                                               "| Key | Name | Definition | The tell | Detector / TRIGGER | Record | Source |", 1))
            return (w.count("antipatterns"), w.count("ingest_receipts"))

        def malformed_post(w, pre):
            ap, rec = w.count("antipatterns"), w.count("ingest_receipts")
            con = sqlite3.connect(f"file:{w.db}?mode=ro", uri=True)
            last = con.execute("SELECT status FROM ingest_receipts ORDER BY id DESC LIMIT 1").fetchone()
            con.close()
            ok = ap == pre[0] and rec == pre[1] + 1 and last and last[0] == "refused"
            return ok, f"antipatterns={pre[0]}->{ap} receipts={pre[1]}->{rec} last={last}"
        case("malformed-ingest-source", "fault", malformed, ["ingest"], 20,
             ["ingest_malformed source=docs/ANTIPATTERNS.md", "AP rows under a header without an ID column"], malformed_post)
        case("unexplained-world-file", "fault",
             lambda w: (w.root / "repo/docs/NEW_THING.md").write_text("# new\n"), ["ingest"], 20,
             ["ingest_world_unexplained", "repo:docs/NEW_THING.md"])
        case("absent-declared-source", "fault",
             lambda w: (w.root / "evidence/learnings/PROCESS-LEARNINGS.md").unlink(), ["ingest"], 30,
             ["ingest_source_absent", "evidence:learnings/PROCESS-LEARNINGS.md"])
        case("integrity-fault", "fault", lambda w: corrupt_table_page(w.db, "antipatterns"), ["check"], 20,
             ["check=integrity verdict=FAIL", "integrity_not_ok"])
        case("foreign-key-fault", "fault",
             lambda w: w.sql("INSERT INTO module_refs(module, kind, ref, ordinal, source_id) VALUES('no-such-module','ap','AP-01',0,1)"),
             ["check"], 20, ["check=foreign_keys verdict=FAIL", "foreign_key_violation"])
        case("orphan-ref", "fault",
             lambda w: w.sql("INSERT INTO module_refs(module, kind, ref, ordinal, source_id) SELECT name,'ap','AP-99',999,source_id FROM modules LIMIT 1"),
             ["check"], 10, ["orphan_ap=AP-99"])
        case("migration-drift", "fault",
             lambda w: w.sql("UPDATE schema_migrations SET sha256 = replace(sha256, substr(sha256,1,1), 'y')"),
             ["check"], 20, ["check=migrations verdict=FAIL", "migration_drift"])
        case("sidecar-stale", "fault",
             lambda w: w.sql("UPDATE decisions SET title = title || ' (planted)' WHERE id='V4-0'"),
             ["search", "verdict", "authority"], 10, ["sidecar=stale engine=like-fallback"])

        # ── alignment + funnel checks (rev 2026-10-01 alignment): one plant per check, each its own diagnostic ──
        def edit(rel: str, old: str, new: str):
            def f(w):
                p = w.root / rel
                t = p.read_text()
                assert t.count(old) >= 1, f"plant anchor absent in {rel}: {old[:60]!r}"
                p.write_text(t.replace(old, new, 1))
            return f
        # Plants that need an OPEN register row insert their own synthetic one (DC-9x) into the case world's copy:
        # the live register may hold no open rows at all (2026-10-01 ratification closed every one), and a control
        # must not depend on the world it certifies happening to contain the fault's precondition.
        REG = "vault/15 Module Design/00 - Module Design Index.md"

        def add_register_row(w, row: str) -> None:
            p = w.root / REG
            lines = p.read_text().split("\n")
            head = next(i for i, ln in enumerate(lines) if ln.startswith("| Id | Conflict |"))
            last = head + 1
            while last + 1 < len(lines) and lines[last + 1].startswith("| DC-"):
                last += 1
            assert last > head + 1, "register has no DC rows to append after"
            lines.insert(last + 1, row)
            p.write_text("\n".join(lines))

        def open_row_then_ingest(row: str, extra=None):
            def f(w):
                add_register_row(w, row)
                if extra:
                    extra(w)
                rc, out = w.run("ingest")
                assert rc == 0, f"plant ingest rc={rc} {out[-300:]}"
            return f

        case("quiet-highway", "quiet", None, ["highway", "store"], 0, ["verdict=PASS", "module=store"])
        case("quiet-readiness", "quiet", None, ["readiness"], 0, ["ready_to_build=", "deferred="])
        # atlas: a synthetic open P5 row the ATLAS §9 table (all '—') does not list; its text cites no UM, so only atlas fires
        case("alignment-atlas-row", "fault",
             open_row_then_ingest("| DC-97 | **Planted open conflict.** control fixture | control | P5 | store | **PROPOSED**: planted |"),
             ["check"], 20, ["check=alignment_atlas verdict=FAIL", "alignment_atlas_ids phase=P5 atlas= register=DC-97"])

        def um_extra(w):  # the marked UM §7 list names a DC no open row carries
            p = w.root / "evidence/design/ULTRAMAP.md"
            t = p.read_text()
            m = "<!-- hee4db:um-amendments:end -->"
            assert t.count(m) == 1, "UM amendments end marker absent or repeated"
            p.write_text(t.replace(m, "  - DC-99 (P3, PROPOSED): planted.\n" + m, 1))
        # um: a synthetic open row citing UM § that the marked list omits (missing) plus a listed id with no row (extra)
        case("alignment-um-list", "fault",
             open_row_then_ingest("| DC-98 | **Planted UM amendment.** UM §4 control fixture | control | P3 | store | **PROPOSED**: planted |", um_extra),
             ["check"], 20, ["check=alignment_um verdict=FAIL", "alignment_um_missing=DC-98", "alignment_um_extra=DC-99"])
        case("funnel-card-heading", "fault",
             edit("repo/modules/hee4-core/store/MODULE.md", "K1%20hee4-core%23store)", "K1%20hee4-core%23storage)"),
             ["check"], 20, ["check=funnel_links verdict=FAIL", "funnel_heading_absent module=store", "heading='storage'"])
        case("funnel-card-phase", "fault",
             edit("repo/modules/hee4-core/store/MODULE.md", "| P5 outbox reader", "| P4 outbox reader"),
             ["check"], 20, ["check=funnel_phases verdict=FAIL", "funnel_phase_mismatch module=store card=P1,P2,P4,P6 manifest=P1,P2,P5,P6"])
        case("readiness-note-edit", "fault",
             edit("vault/00 Hub/Module Readiness 2026-10-01.md", "**READY TO BUILD** | P1,P2,P3", "**ARCH REVIEW** | P1,P2,P3"),
             ["check"], 20, ["check=readiness_note verdict=FAIL", "readiness_note_differs"])
        # U-harden-05: the note carries no count a new source file changes (V4-100's hand regen drifted on the next .rs)
        def plant_rs(w):
            p = w.root / "repo/crates/planted-control/src/planted.rs"
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text("")
        n += 1
        w = World(base, f"c{n:02d}", frozen)
        w.copy_db_from(snap)
        try:
            plant_rs(w)
            rs_now = sum(1 for _ in (w.root / "repo").rglob("*.rs"))
            rrc, rout = w.run("readiness")
            rc, out = w.run("check")
            bad = red_checks(out)
            saw = re.search(rf"\brs_files={rs_now}\b", rout) is not None   # the generator counted the planted file
            ok = rc in (0, 10) and "check=readiness_note verdict=PASS" in out and not bad and rrc == 0 and saw
            results.append(("quiet", "readiness-note-survives-a-new-rs-file", ok,
                            f"rc={rc} readiness_rc={rrc} rs_files={rs_now} counted={'yes' if saw else 'no'}"
                            + (f" red={','.join(bad)}" if bad else "")))
        except Exception as e:
            results.append(("quiet", "readiness-note-survives-a-new-rs-file", False, f"setup_error {type(e).__name__}: {str(e)[:200]}"))

        def plant_rs_and_row(w):
            plant_rs(w)
            edit("vault/00 Hub/Module Readiness 2026-10-01.md", "**READY TO BUILD** | P1,P2,P3", "**ARCH REVIEW** | P1,P2,P3")(w)
        case("readiness-note-real-change-still-fails", "fault", plant_rs_and_row,
             ["check"], 20, ["check=readiness_note verdict=FAIL", "readiness_note_differs"])

        case("register-unknown-module", "fault",
             lambda w: add_register_row(w, "| DC-96 | **Planted unknown module.** control fixture | control | P2 | roster, storr | **PROPOSED**: planted |"),
             ["ingest"], 20, ["ingest_malformed", "DC-96 names unknown module 'storr'"])
        case("register-closed-row-phase", "fault",
             edit("vault/15 Module Design/00 - Module Design Index.md", "| K0 C-2, Socket map | — | — |", "| K0 C-2, Socket map | P1 | — |"),
             ["ingest"], 20, ["ingest_malformed", "DC-02 status RESOLVED with phase 'P1'"])

        # ── Jev Fit Map (rev 2026-10-01 jev-fit): one plant per check and per ingest refusal, each its own diagnostic ──
        FIT = "vault/50 Jev/Jev Fit Map.md"
        case("quiet-highway-jev", "quiet", None, ["highway", "--jev", "--grade", "B"], 0, ["verdict=PASS", "jev grade=B bytes="])
        case("quiet-highway-jev-hop", "quiet", None, ["highway", "candidates"], 0, ['"jev_fits": {"total": 4', '"all_crates": ["E6", "E7"]'])
        case("jev-grades-selfcheck", "fault", edit(FIT, "intersections=51 A=1 B=12", "intersections=51 A=2 B=12"),
             ["check"], 20, ["check=jev_grades verdict=FAIL", "jev_grades_mismatch key=A selfcheck=2 table=1"])
        case("jev-scope-engine-allowed", "fault", lambda w: w.sql("UPDATE jev_fits SET allowed_now='yes' WHERE id='J2'"),
             ["check"], 20, ["check=jev_scope verdict=FAIL", "jev_scope_engine_data_allowed id=J2 grade=B allowed_now=yes"])
        case("jev-scope-x-reason", "fault", lambda w: w.sql("UPDATE jev_fits SET grade_reason='' WHERE id='J4'"),
             ["check"], 20, ["check=jev_scope verdict=FAIL", "jev_scope_x_no_reason id=J4"])
        case("jev-scope-a-not-now", "fault", lambda w: w.sql("UPDATE jev_fits SET allowed_now='no' WHERE id='A4'"),
             ["check"], 20, ["check=jev_scope verdict=FAIL", "jev_scope_a_not_now id=A4 allowed_now=no"])
        case("jev-gates-b-ungated", "fault", lambda w: w.sql("UPDATE jev_fits SET gates='' WHERE id='Q1'"),
             ["check"], 20, ["check=jev_gates verdict=FAIL", "jev_gates_b_ungated id=Q1"])
        case("jev-gates-absent", "fault", lambda w: w.sql("DELETE FROM held_items WHERE id='H-12'"),
             ["check"], 20, ["check=jev_gates verdict=FAIL", "jev_gates_absent gate=H-12"])
        # held (rev 2026-10-05 watch-contradiction): the status word written in the id cell, not the Item cell
        # parse_held reads. The plant is the H-27 row as it was committed on 2026-10-05 (V4-86), re-ingested so
        # the DB shows the symptom (H-27 open); `check=held` must name the cause.
        ATLAS = "evidence/design/DEPLOYMENT_ATLAS.md"
        H27_ROW = "| H-27 *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F07)* | **CLOSED 2026-10-05 by V4-86**: Luke ratified"

        def misplace_h27_then_ingest(w):
            edit(ATLAS, H27_ROW, "| H-27 **CLOSED 2026-10-05 by V4-86** *(rev 2026-10-01 V7/V8/V9/V10-fix, V7 F07)* | Luke ratified")(w)
            rc, out = w.run("ingest")
            assert rc == 0, f"plant ingest rc={rc} {out[-300:]}"

        def h27_open_in_db(w, _pre):
            rc, out = w.run("q", "SELECT id, status FROM held_items WHERE id='H-27'")
            return "open" in out, f"db_status={'open' if 'open' in out else 'not-open'}"
        case("held-closed-word-misplaced", "fault", misplace_h27_then_ingest,
             ["check"], 20, ["check=held verdict=FAIL", "held_closed_word_misplaced id=H-27"], post=h27_open_in_db)
        # quiet half: the same word written in the Item cell of an open row (H-3) is where parse_held reads it,
        # so `check=held` stays PASS and names no row
        n += 1
        w = World(base, f"c{n:02d}", frozen)
        w.copy_db_from(snap)
        edit(ATLAS, "| H-3 | **`systemctl --user enable`**", "| H-3 | **CLOSED 2026-10-05 by V4-99**: planted. Was: **`systemctl --user enable`**")(w)
        _, hout = w.run("check")   # json out: the check lines sit inside the doc, so match the token, not a line
        hok = "check=held verdict=PASS" in hout and "held_closed_word_misplaced" not in hout
        results.append(("quiet", "quiet-held-closed-in-status-cell", hok,
                        f"held_pass={'check=held verdict=PASS' in hout} misplaced_token={'held_closed_word_misplaced' in hout}"))
        case("jev-fit-unknown-grade", "fault", edit(FIT, "| 0 | **X**: exact (DESIGN JM J4) |", "| 0 | **Z**: exact (DESIGN JM J4) |"),
             ["ingest"], 20, ["ingest_malformed", "J4 grade 'Z' not in"])
        case("jev-fit-unknown-module", "fault", edit(FIT, "| J4 | Is the reply empty? | K6 `candidates` |", "| J4 | Is the reply empty? | K6 `candidatez` |"),
             ["ingest"], 20, ["ingest_malformed", "J4 names unknown module 'candidatez'"])
        case("jev-fit-duplicate-id", "fault", edit(FIT, "| J5 | Route ranking R11 |", "| J4 | Route ranking R11 |"),
             ["ingest"], 20, ["ingest_malformed", "jev fit map duplicate id J4"])

        # ── registry (rev 2026-10-01 registry, V4-70): skills, agents, reflexes; one plant per refusal SITE ──
        def then_ingest(*plants):
            def f(w):
                for pl in plants:
                    pl(w)
                rc, out = w.run("ingest")
                assert rc in (0, 10), f"plant ingest rc={rc} {out[-300:]}"
            return f

        def unlink(rel: str):
            return lambda w: (w.root / rel).unlink()

        def absent_from(args: list[str], needle: str):
            def post(w, _pre):
                _rc, out = w.run(*args)
                return needle not in out, f"absent={needle!r}:{needle not in out}"
            return post

        def present_in(args: list[str], needle: str):
            def post(w, _pre):
                _rc, out = w.run(*args)
                return needle in out, f"present={needle!r}:{needle in out}"
            return post
        case("quiet-recipe-skills", "quiet", None, ["recipe", "skills"], 0,
             ['"name": "hee-v4-corpus", "scope": "user", "v4_reason": "rule:v4"', '"name": "claim-discipline", "scope": "user", "v4_reason": "curated:',
              "rule v4_relevant ="], absent_from(["recipe", "skills"], '"name": "hee-v3-corpus"'))
        case("quiet-recipe-agents", "quiet", None, ["recipe", "agents"], 0,
             [f'"agent": "{PLANTED}", "definition": "project:', '"schedule": "light@', '"spend_24h_usd"'])
        case("quiet-recipe-reflexes", "quiet", None, ["recipe", "reflexes"], 0,
             ['"name": "jev-recall-guard.sh", "jev_sender": 1', '"name": "pipe-verdict-guard.sh", "jev_sender": 0', "jev_senders="])
        case("quiet-recipe-restart", "quiet", None, ["recipe", "restart"], 0,
             ['"hold": {"H-5": "', '"p0_open_holds"', '"skills_to_load"', '"agent_status"', '"restart_pointer"', '"entry_routes"', "bytes="])
        case("jev-sender-by-name", "quiet", then_ingest(lambda w: (w.root / "claude/hooks/jev-planted-guard.sh").write_text("#!/bin/bash\necho hi\n")),
             ["q", "SELECT name, jev_sender, jev_reason, wired FROM reflexes WHERE name = 'jev-planted-guard.sh'"], 0,
             ['"jev_sender": 1, "jev_reason": "name", "wired": 0'])
        case("jev-sender-by-code", "quiet", then_ingest(lambda w: (w.root / "claude/hooks/planted-guard.sh").write_text("#!/bin/bash\npython3 -m jevpack.ask x\n")),
             ["q", "SELECT name, jev_sender, jev_reason FROM reflexes WHERE name = 'planted-guard.sh'"], 0,
             ['"jev_sender": 1, "jev_reason": "code:jevpack"'])
        case("registry-skill-absent", "fault", unlink("claude/skills/hee-v4-corpus/SKILL.md"), ["check"], 20,
             ["check=registry_skills verdict=FAIL", "registry_skill_absent skill=user:hee-v4-corpus"])
        case("registry-skill-unparsed", "fault", edit("claude/skills/claim-discipline/SKILL.md", "---\nname: claim-discipline", "name: claim-discipline"),
             ["check"], 20, ["check=registry_skills verdict=FAIL", "registry_skill_unparsed skill=user:claim-discipline error=frontmatter_absent"])
        case("registry-skill-curated-unknown", "fault", then_ingest(unlink("claude/skills/handoff/SKILL.md")), ["check"], 20,
             ["check=registry_skills verdict=FAIL", "registry_skill_curated_unknown name=handoff"])
        AGENT_MD, MODES = f"repo/.claude/agents/{PLANTED}.md", f"repo/ops/roster/{PLANTED}/modes.conf"
        case("registry-agent-undefined", "fault", then_ingest(unlink(AGENT_MD)), ["check"], 20,
             ["check=registry_agents verdict=FAIL", f"registry_agent_undefined agent={PLANTED}"])
        case("registry-agent-unparsed", "fault",
             then_ingest(edit(AGENT_MD, "\ndescription:", "\nsummary:")), ["check"], 20,
             ["check=registry_agents verdict=FAIL", f"registry_agent_definition_unparsed agent={PLANTED} error=frontmatter_missing description"])
        case("registry-agent-no-modes", "fault", then_ingest(unlink(MODES)), ["check"], 20,
             ["check=registry_agents verdict=FAIL", f"registry_agent_no_modes agent={PLANTED}"])
        case("registry-agent-mode-unknown", "fault", then_ingest(edit("crontab.txt", f"run-agent.sh {PLANTED} deep", f"run-agent.sh {PLANTED} turbo")),
             ["check"], 20, ["check=registry_agents verdict=FAIL", f"registry_agent_mode_unknown agent={PLANTED} mode=turbo"])

        def timers_absent(w):   # the seam points at a path that does not exist: an absent source, never a zero
            w.env["HEE4DB_TIMERS_DIR"] = str(w.root / "no-such-timers")
        # (rev 2026-10-05 schedule) no readable schedule source at all is UNMEASURED, never PASS
        case("registry-schedule-unmeasured", "fault", then_ingest(unlink("crontab.txt"), timers_absent), ["check"], 30,
             ["check=registry_agents verdict=UNMEASURED", "registry_schedule_unmeasured"])
        case("registry-stale-skill", "fault",
             lambda w: (w.root / "claude/skills/hee-v4-corpus/SKILL.md").write_text((w.root / "claude/skills/hee-v4-corpus/SKILL.md").read_text() + "\n"),
             ["stale"], 20, ["stale_registry path=claude:skills/hee-v4-corpus/SKILL.md state=changed"])
        case("registry-modes-malformed", "fault",
             lambda w: (w.root / MODES).write_text("broken line without pipes\n"), ["ingest"], 20,
             ["ingest_malformed", "modes.conf wants `mode|budget_usd|prompt`"])
        case("registry-modes-empty-mode", "fault",
             lambda w: (w.root / MODES).write_text(" |1.00|a mode line with no mode name\n"), ["ingest"], 20,
             ["ingest_malformed", "modes.conf wants `mode|budget_usd|prompt`"])
        case("registry-crontab-malformed", "fault", edit("crontab.txt", f"run-agent.sh {PLANTED} deep", "run-agent.sh"), ["ingest"], 20,
             ["ingest_malformed", "run-agent.sh without <agent> <mode>"])

        # ── systemd user timers as a schedule source (rev 2026-10-05 schedule): one plant per refusal site ──
        def plant_timer(w, stem: str, exec_start: str, timer_text: str | None = None) -> None:
            d = w.root / "timers"
            (d / f"{stem}.timer").write_text(timer_text if timer_text is not None else timer_unit("*-*-* 04:00:00", f"{stem}.service"))
            (d / f"{stem}.service").write_text(service_unit(exec_start))
        case("timers-roster-undefined", "fault",
             then_ingest(lambda w: plant_timer(w, "hee4-ghost-roster", f"/usr/bin/bash {RUNNER} hee4-ghost light")), ["check"], 20,
             ["check=registry_agents verdict=FAIL", "registry_agent_undefined agent=hee4-ghost", "hee4-ghost-roster.timer line="])
        case("timers-malformed", "fault",
             lambda w: plant_timer(w, "hee4-broken", "/usr/bin/true", "[Unit]\nDescription=no timer section\n\n[Install]\nWantedBy=timers.target\n"),
             ["ingest"], 20, ["ingest_malformed", "hee4-broken.timer no [Timer] section"])
        case("timers-unit-escape", "fault",   # Unit= names a sibling .service, never a path out of the declared source
             lambda w: plant_timer(w, "hee4-escape", "/usr/bin/true", timer_unit("*-*-* 04:00:00", "../x.service")),
             ["ingest"], 20, ["ingest_malformed", "hee4-escape.timer Unit=../x.service is not a .service name in the timer directory"])
        case("crontab-absent-timers-ok", "quiet", unlink("crontab.txt"), ["ingest"], 0,
             ["registry_crontab_absent", "registry_unreadable=0"], absent_from(["ingest"], "registry_crontab_unmeasured"))
        case("timers-stale", "fault",
             lambda w: (w.root / "timers" / f"{PLANTED}-daily.timer").write_text((w.root / "timers" / f"{PLANTED}-daily.timer").read_text() + "\n"), ["stale"], 20,
             ["stale_registry path=", "/timers state=changed"])
        case("timers-daily-kind", "quiet", None, ["q", "SELECT kind, agent FROM agent_schedules WHERE kind='daily'"], 0,
             ['"rows": [{"kind": "daily", "agent": null}]'])

        # a timers directory that exists but cannot be listed is unreadable, never `ok entries=0` (AP-29/F138: no zero
        # from a source that was not read); the mode is restored in cleanup so the temp dir can be removed
        def timers_unlistable(w):
            (w.root / "timers").chmod(0)

        def timers_listable(w):
            (w.root / "timers").chmod(0o755)
        case("timers-dir-unreadable", "fault", timers_unlistable, ["ingest"], 10,
             ["registry_timers_unmeasured PermissionError", "registry_unreadable=1"],
             present_in(["q", "SELECT status, entries FROM registry_sources WHERE kind='timers'"], '"status": "unreadable"'), cleanup=timers_listable)
        case("timers-dir-unreadable-check", "fault", then_ingest(unlink("crontab.txt"), timers_unlistable), ["check"], 30,
             ["check=registry_agents verdict=UNMEASURED", "registry_schedule_unmeasured", "timers=unreadable"], cleanup=timers_listable)

        def sources_present_no_roster(w):   # an empty present source is a measured zero, never UNMEASURED
            shutil.rmtree(w.root / "repo/ops/roster" / PLANTED)
            (w.root / "crontab.txt").write_text("".join(ln + "\n" for ln in CRONTAB_FIXTURE.splitlines() if "run-agent.sh" not in ln))
            for name in (f"{PLANTED}-roster.timer", f"{PLANTED}-roster.service"):
                (w.root / "timers" / name).unlink()
            rc, out = w.run("ingest")
            assert rc == 0, f"plant ingest rc={rc} {out[-300:]}"
        n += 1
        w = World(base, f"c{n:02d}", frozen)
        w.copy_db_from(snap)
        try:
            sources_present_no_roster(w)
            rc, out = w.run("check")
            want = ["check=world verdict=PASS", "measured_empty=agent_modes", "check=registry_agents verdict=PASS", "roster_jobs=0", "sources=present"]
            missing = [x for x in want if x not in out]
            ok = rc in (0, 10) and not missing
            results.append(("quiet", "empty-world-sources-present", ok, f"rc={rc} want=0|10" + (f" missing={missing}" if missing else " needles=" + ",".join(want))))
        except Exception as e:
            results.append(("quiet", "empty-world-sources-present", False, f"setup_error {type(e).__name__}: {str(e)[:200]}"))

        # ── roster run logs (CN-20, CN-04): planted in the case world's evidence root ──
        def plant_log(w, agent: str, stamp: str, exit_line: bool) -> Path:
            d = w.root / "evidence" / "roster" / agent
            d.mkdir(parents=True, exist_ok=True)
            f = d / f"run-{stamp}-selfcheck.log"
            f.write_text(f"{agent} mode=selfcheck start={stamp} budget_usd=0.50 measurements=m measure_rc=0\n"
                         "cost_usd=0.1 is_error=False subtype=success\n"
                         f"verdict_line={agent[5:]} verdict=PASS planted\n" + ("exit=0 measure_rc=0\n" if exit_line else ""))
            return f
        case("roster-legacy-only", "quiet", lambda w: plant_log(w, "hee4-curator", "20260930T220000Z", False),
             ["record", "run", "--from-logs"], 0, ["excluded_pre_contract=1", "missing_exit_or_cost=0"])
        case("roster-contract-missing-exit", "fault",
             lambda w: (plant_log(w, "hee4-curator", "20260930T220000Z", False), plant_log(w, "hee4-curator", "20261001T120000Z", False)),
             ["record", "run", "--from-logs"], 10, ["excluded_pre_contract=1", "missing_exit_or_cost=1", "exit_status=missing"])

        def db_record_line(w):
            f = plant_log(w, "hee4-workflow-curator", "20261001T120000Z", True)
            rc1, out1 = w.run("record", "run", "--log", str(f))
            assert rc1 == 0 and "new exit_status=recorded" in out1, out1[-300:]
            with open(f, "a") as fh:
                fh.write("db_record=ok rc=0 run_rc=0 measurement_rc=0\n")
            return f
        n += 1
        w = World(base, f"c{n:02d}", frozen)
        w.copy_db_from(snap)
        try:
            f = db_record_line(w)
            rc, out = w.run("record", "run", "--log", str(f))
            ok = rc == 0 and "unchanged exit_status=recorded" in out
            results.append(("quiet", "roster-db-record-line", ok, f"rc={rc} want=0 needle='unchanged exit_status=recorded'"
                            + ("" if ok else f" out={out[-200:]!r}")))
        except Exception as e:
            results.append(("quiet", "roster-db-record-line", False, f"setup_error {type(e).__name__}: {str(e)[:200]}"))

        # The claim id comes from record claim's own output (a copy of a used DB already holds claims).
        n += 1
        w = World(base, f"c{n:02d}", frozen)
        w.copy_db_from(snap)
        rc0, out0 = w.run("record", "claim", "--who", "agent-a", "--claim", "x=1", "--evidence", "e")
        cid = next((str(json.loads(ln)["id"]) for ln in out0.splitlines() if ln.startswith("{")), "none")
        rc, out = w.run("record", "verify", "--id", cid, "--by", "agent-a", "--status", "verified", "--result", "r")
        rc2, out2 = w.run("record", "verify", "--id", cid, "--by", "agent-b", "--status", "verified", "--result", "r")
        ok = rc0 == 0 and rc == 20 and "refused_self_verify" in out and rc2 == 0
        results.append(("fault", "self-verify", ok, f"claim_id={cid} self_rc={rc} other_rc={rc2}"
                        + ("" if "refused_self_verify" in out else " missing=['refused_self_verify']")))
    finally:
        real_after = {p: sha(p) for p in real_before}
        shutil.rmtree(base, ignore_errors=True)
    unchanged_real = real_before == real_after
    for kind, name, ok, why in results:
        print(f"{'ok  ' if ok else 'MISS'} {kind:5} {name:26} {why}")
    faults = [r for r in results if r[0] == "fault"]
    quiet = [r for r in results if r[0] == "quiet"]
    fk = sum(r[2] for r in faults)
    qk = sum(r[2] for r in quiet)
    passed = fk == len(faults) and qk == len(quiet) and unchanged_real
    print(f"clauses={fk}/{len(faults)} quiet={qk}/{len(quiet)} real_db_unchanged={'yes' if unchanged_real else 'NO'} "
          f"verdict={'PASS' if passed else 'FAIL'}")
    return 0 if passed else 20


# Rule neuters for `--neuter`: each disables ONE rule in a temp copy of hee4db; the control must go red.
NEUTERS = [
    ("row-cap", "if len(rows) > row_cap:", "if False:"),
    ("byte-cap", "if nbytes > byte_cap:", "if False:"),
    ("step-budget", 'return 1 if steps["n"] * 1000 > VM_STEP_BUDGET else 0', "return 0"),
    ("authorizer-deny", "return sqlite3.SQLITE_DENY", "return sqlite3.SQLITE_OK"),
    ("q-empty-world", '        if w == 0:\n            o.line(f"unmeasured_empty_world tables={\',\'.join(sorted(tables)) or \'none\'}',
     '        if False:\n            o.line(f"unmeasured_empty_world tables={\',\'.join(sorted(tables)) or \'none\'}'),
    ("integrity", 'ok = py == ["ok"] and tu == ["ok"]', "ok = True"),
    ("foreign-keys", 'EXIT_OK if not fk else EXIT_REFUSED', "EXIT_OK"),
    ("stale-compare", "if cur != sha:", "if False:"),
    ("stale-new-files", "if (r, rel) not in known and", "if False and"),
    ("ap-header", "if t not in ap_tables:", "if False:"),
    ("world-unexplained", "        if unexplained:\n            raise Refusal(\"ingest_world_unexplained\"",
     "        if False:\n            raise Refusal(\"ingest_world_unexplained\""),
    ("absent-source", "        if absent:\n", "        if False:\n"),
    ("self-verify", "if r[0] == a.by:", "if False:"),
    ("sidecar-fresh", 'return ("fresh" if rc == 0 and have == corpus_signature(db) else "stale"), have', 'return "fresh", have'),
    ("migration-drift", "bad = [p.name for v, _, p in files if applied.get(v) != sha256_bytes(read_source(p))]", "bad = []"),
    ("orphans", "n = sum(len(v) for v in orphans.values())", "n = 0"),
    ("check-world", 'add("world", EXIT_OK if not empty else EXIT_UNMEASURED,', 'add("world", EXIT_OK,'),
    ("multi-statement", 'if "one statement" in str(e):', "if False:"),
    ("legacy-cutoff", '"legacy_no_exit" if r["stamp"] < EXIT_CONTRACT_STAMP else "missing"', '"legacy_no_exit"'),
    ("db-record-digest", "if not ln.startswith(DB_RECORD_PREFIX)", "if True"),
    ("ingest-rollback-receipt", "        record_failed_receipt(a.db, \"ingest\", started, r, len(plan))\n", ""),
    ("atlas-ids", "for p in want if p not in gm or gm[p][\"dc_ids\"] != want[p][\"dc_ids\"]]", "for p in want if p not in gm]"),
    ("um-compare", "ok = not miss and not extra and not dup", "ok = True"),
    ("funnel-heading", "        elif head not in hs:\n", "        elif False:\n"),
    ("funnel-phase", "        elif got[0] != want:\n", "        elif False:\n"),
    ("readiness-compare", "            if have == want_txt:\n", "            if True:\n"),
    ("register-unknown-module", "            if r[\"module\"] not in mods:\n                raise Refusal(\"ingest_malformed\", f\"source={rel} line {r['line']} {r['dc_id']}",
     "            if False:\n                raise Refusal(\"ingest_malformed\", f\"source={rel} line {r['line']} {r['dc_id']}"),
    ("register-closed-phase", "        if (cls == \"RESOLVED\") != (phase is None):\n", "        if False:\n"),
    # rev 2026-10-01 jev-fit
    ("jev-grades-compare", "bad = [f\"jev_grades_mismatch key={k} selfcheck={want[k]} table={have[k]}\" for k in want if want[k] != have[k]]",
     "bad = []"),
    ("jev-scope-engine", "if f[\"engine_data\"] and grant_absent and (f[\"allowed_now\"] != \"no\" or", "if False and (f[\"allowed_now\"] != \"no\" or"),
    ("jev-scope-x-reason", "for f in xs if not f[\"grade_reason\"].strip()]", "for f in xs if False]"),
    ("jev-gates-absent", "bad = [f\"jev_gates_absent gate={g}\" for g in cited if aliases.get(g, g) not in held]", "bad = []"),
    ("held-closed-word-misplaced", 'if status == "open" and any("**closed" in c.lower()', 'if False and any("**closed" in c.lower()'),
    ("jev-scope-a-now", "if f[\"grade\"] == \"A\" and (f[\"allowed_now\"] != \"yes\" or f[\"engine_data\"]):", "if False:"),
    ("jev-gates-b-ungated", "for f in fits if f[\"grade\"] == \"B\" and not f[\"gates\"]]", "for f in fits if False]"),
    ("jev-fit-grade", "            if grade not in JEV_GRADES:\n", "            if False:\n"),
    ("jev-fit-module", "            if r[\"module\"] not in mods:\n                raise Refusal(\"ingest_malformed\", f\"source={rel} line {r['line']} {r['fit_id']}",
     "            if False:\n                raise Refusal(\"ingest_malformed\", f\"source={rel} line {r['line']} {r['fit_id']}"),
    ("jev-fit-duplicate", "    if dup:\n        raise Refusal(\"ingest_malformed\", f\"source={src} jev fit map duplicate id",
     "    if False:\n        raise Refusal(\"ingest_malformed\", f\"source={src} jev fit map duplicate id"),
]


NEUTERS += [  # rev 2026-10-01 registry (V4-70): one per refusal site and per rule
    ("registry-skill-absent", 'bad.append(f"registry_skill_absent skill={scope}:{dname} path={root}:{rp}")', "pass"),
    ("registry-skill-unparsed", 'bad.append(f"registry_skill_unparsed skill={scope}:{dname} error={err}")', "pass"),
    ("registry-skill-curated", 'bad += [f"registry_skill_curated_unknown name={c}" for c in sorted(SKILL_V4_CURATED) if c not in names]', "bad += []"),
    ("registry-agent-undefined", 'bad.append(f"registry_agent_undefined agent={ag} {loc}")', "pass"),
    ("registry-agent-unparsed", 'bad.append(f"registry_agent_definition_unparsed agent={ag} error={defs[ag]}")', "pass"),
    ("registry-agent-no-modes", 'bad.append(f"registry_agent_no_modes agent={ag} {loc}")', "pass"),
    ("registry-agent-mode", 'bad.append(f"registry_agent_mode_unknown agent={ag} mode={md} {loc}")', "pass"),
    # rev 2026-10-05 schedule: the timers source and the two measured-zero rules
    ("registry-schedule-unmeasured", "        if not sources_ok:\n", "        if False:\n"),
    ("timers-roster-kind", '        at = [j for j, x in enumerate(toks) if Path(x).name == "run-agent.sh"]\n', "        at = []\n"),
    ("timers-section", '        if "Timer" not in sections:\n', "        if False:\n"),
    ("timers-stale-changed", '        elif tstatus == "ok" and tsha != row[0]:\n', "        elif False:\n"),
    # a listing that swallows PermissionError (Path.glob's behaviour) reads an unlistable dir as ok entries=0
    ("timers-dir-unreadable", "        names = sorted(os.listdir(d))\n", "        names = sorted(os.listdir(d)) if os.access(d, os.R_OK) else []\n"),
    ("timers-unit-escape", '            if Path(unit).name != unit or not unit.endswith(".service"):', "            if False:"),
    ("crontab-absent-word", '        return "absent", None, f"HEE4DB_CRONTAB_FILE={p} absent"\n', '        return "unreadable", None, f"HEE4DB_CRONTAB_FILE={p} absent"\n'),
    ("world-measured-empty", "            if ok_kinds:\n", "            if True:\n"),
    ("registry-stale-changed", '        elif sha256_bytes(reg_read(fp)) != sha:\n', "        elif False:\n"),
    ("registry-modes-shape", "        if len(parts) != 3 or not parts[0].strip():\n", "        if len(parts) != 3:\n"),
    ("registry-crontab-args", "            if len(toks) < j + 3:\n", "            if False:\n"),
    ("skill-v4-rule", "            if part in SKILL_V4_WORDS or part.startswith(SKILL_V4_PREFIXES):\n", "            if False:\n"),
    ("skill-v4-excluded", "    if name in SKILL_V4_EXCLUDED:\n", "    if False:\n"),
    ("skill-v4-curated", "    if name in SKILL_V4_CURATED:\n", "    if False:\n"),
    ("jev-sender-name", '    by_name = name.startswith("jev-")\n', "    by_name = False\n"),
    ("jev-sender-code", '    hits = sorted({w for w in code_words(code) if w != stem and (w.startswith("jevpack")', '    hits = sorted({w for w in code_words(code) if False and (w.startswith("jevpack")'),
]


def neuter() -> int:
    src = (HERE.parent / "hee4db").read_text()
    killed, rows = 0, []
    with tempfile.TemporaryDirectory(prefix="hee4db-neuter-") as td:
        for name, old, new in NEUTERS:
            if src.count(old) != 1:
                rows.append(f"SITE_NOT_UNIQUE {name} count={src.count(old)}")
                continue
            mut = Path(td) / f"hee4db-{name}"
            mut.write_text(src.replace(old, new, 1))
            env = dict(os.environ, HEE4DB_BIN=str(mut), HEE4DB_SCHEMA_DIR=str(HERE.parent / "schema"))
            p = subprocess.run([sys.executable, __file__], env=env, capture_output=True, text=True, timeout=1800)
            red = p.returncode != 0 and "verdict=FAIL" in p.stdout
            missed = [ln.split()[2] for ln in p.stdout.splitlines() if ln.startswith("MISS")]
            killed += red
            rows.append(f"{'killed  ' if red else 'SURVIVED'} {name:24} rc={p.returncode} red_cases={','.join(missed) or '-'}")
    for r in rows:
        print(r)
    ok = killed == len(NEUTERS)
    print(f"neuters killed={killed}/{len(NEUTERS)} verdict={'PASS' if ok else 'FAIL'}")
    return 0 if ok else 20


if __name__ == "__main__":
    sys.exit(neuter() if "--neuter" in sys.argv else main())
