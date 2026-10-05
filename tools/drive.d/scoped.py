"""Out-of-release features: driven by their refusal (gates/features/README.md "The drive").

A feature with no procedure in tools/drive.d is asked about through tools.inspect. When the
catalogue entry's `scope` is not the release this tree builds (the workspace version in
Cargo.toml, 4.0.x -> v40), the feature is measured by two paths:
  catalogued        tools.inspect lists the action with that scope
  refused_by_scope  the action, sent with a minimal well-formed body, is refused `unavailable` at
                    /action with exactly that scope's `because` text
The `because` text of each scope is read from the one place the refusal is built from,
crates/hee4-contracts/src/catalogue.rs (`impl Scope`: `wire_name` and `because`), never written
here; the scope is read from tools.inspect, never written here. A served answer, another code or
another `because` is a FAIL by path name. A feature in the release's own scope, or one whose
scope tools.inspect does not give, is not out of release: it stays UNMEASURED, no procedure.

FEATURES is empty: this module serves no feature by name; tools/drive calls drive() from its
unserved branch.
"""
import json
import os
import re
import uuid

from drive_d import is_result

FEATURES = []

CATALOGUE = os.path.join("crates", "hee4-contracts", "src", "catalogue.rs")
PLACEHOLDER = "drive-scope"


def _arms(src, fn):
    """{variant: string literal} from the match arms of `pub const fn <fn>` in `src`."""
    m = re.search(rf"pub const fn {fn}\(self\)[^{{]*\{{\s*match self \{{(.*?)\n        \}}", src, re.S)
    return dict(re.findall(r'Self::(\w+)\s*=>\s*\{?\s*"([^"]*)"', m.group(1))) if m else {}


def scope_because(repo):
    """{wire scope: because text} from `impl Scope` in catalogue.rs; {} when unreadable."""
    try:
        with open(os.path.join(repo, CATALOGUE)) as f:
            src = f.read()
    except OSError:
        return {}
    impl = src.split("impl Scope {", 1)
    if len(impl) < 2:
        return {}
    body = impl[1].split("\n}\n", 1)[0]
    wire, because = _arms(body, "wire_name"), _arms(body, "because")
    return {w: because[v] for v, w in wire.items() if v in because}


def release_scope(repo):
    """The wire scope of the release this tree builds: `v<major><minor>` of the workspace version."""
    try:
        with open(os.path.join(repo, "Cargo.toml")) as f:
            m = re.search(r'^version\s*=\s*"(\d+)\.(\d+)\.', f.read(), re.M)
    except OSError:
        return None
    return f"v{m.group(1)}{m.group(2)}" if m else None


def display(wire):
    """v42 -> v4.2 (the spelling of the feature files); any other wire name is printed as it is."""
    m = re.fullmatch(r"v(\d)(\d+)", wire)
    return f"v{m.group(1)}.{m.group(2)}" if m else wire


def minimal_body(feature_text):
    """{field: placeholder} for each member of the feature file's `Socket: request body {...}`."""
    m = re.search(r"Socket: request `body` `\{([^}]*)\}`", feature_text)
    if not m:
        return {}
    names = [re.split(r"[:\[\s]", p.strip(), maxsplit=1)[0] for p in m.group(1).split(",")]
    return {n: PLACEHOLDER for n in names if n}


def drive(F, name, repo, feature_text):
    """(scope, why): runs the two paths and returns the display scope, or (None, why) when the
    feature is not out of release (no path run; the caller prints UNMEASURED, no procedure)."""
    r = F.req("tools.inspect", {"action": name, "version": 1})
    body = (r.get("body") or {}) if is_result(r) else {}
    scope = body.get("scope")
    if not is_result(r):
        return None, f"tools.inspect {name} answered {json.dumps(r)[:120]}"
    if not isinstance(scope, str):
        return None, f"tools.inspect {name} carries no scope member"
    release = release_scope(repo)
    if release is None:
        return None, "no workspace version in Cargo.toml"
    if scope == release:
        return None, f"scope {display(scope)} is this release's"
    because = scope_because(repo).get(scope)
    F.check("catalogued", body.get("action") == name, f"tools.inspect action={body.get('action')} scope={scope}")
    if because is None:
        F.check("refused_by_scope", False, f"scope {scope} has no because text in {CATALOGUE}")
        return display(scope), ""
    reply = F.req(name, minimal_body(feature_text), key=f"drive-scope-{uuid.uuid4().hex[:12]}")
    F.refuse("refused_by_scope", reply, "unavailable", "/action", extra={"because": because})
    return display(scope), ""
