"""drive_d: the plugin home of tools/drive (gates/features/README.md "The drive").

One module per family, tools/drive.d/<family>.py, each exporting
  FEATURES = [(feature_name, procedure), ...]
where procedure(F, ctx) drives gates/features/<feature_name>.md through a Feature. The host
(tools/drive) and the check (tools/features-check) derive the served set from these modules
and nothing else. 'drive.d' is not an importable package name: the host loads this file as the
module 'drive_d' with importlib (see load_plugins), so a plugin may `from drive_d import ...`.
"""
import importlib.util
import os
import pathlib
import sqlite3
import sys

MARK = "(rev 2026-10-05 drive)"
TERMINAL = {"accepted", "failed", "cancelled", "abandoned"}
PHASES = TERMINAL | {"admitted", "running", "verifying", "cancellation_requested", "repair_pending"}
NO_LEDGER_REASON = "--ledger not passed"


class PluginFault(Exception):
    """A plugin that cannot be served: the host prints FAIL naming the module and the reason."""

    def __init__(self, module, reason):
        super().__init__(f"plugin {module}: {reason}")
        self.module, self.reason = module, reason


def is_result(r):
    """True when a reply is a result frame."""
    return bool(r) and r.get("kind") == "result"


def ledger_ro(ctx):
    """A read-only sqlite3 connection to ctx["ledger"], or None when --ledger was not passed.

    Opens `file:<path>?mode=ro` (uri=True): a write raises sqlite3.OperationalError. A
    side-effect check without a ledger prints UNMEASURED with NO_LEDGER_REASON, never a 0.
    """
    path = ctx.get("ledger")
    if not path:
        return None
    uri = pathlib.Path(path).resolve().as_uri() + "?mode=ro"
    return sqlite3.connect(uri, uri=True)


def _features_of(stem, mod):
    feats = getattr(mod, "FEATURES", None)
    shape = (isinstance(feats, list)
             and all(isinstance(t, tuple) and len(t) == 2 and isinstance(t[0], str) and callable(t[1]) for t in feats))
    if not shape:
        raise PluginFault(stem, "FEATURES must be a list of (name, procedure) tuples")
    return feats


def load_plugins(plugins_dir, features_dir):
    """name -> procedure from every tools/drive.d/*.py (sorted; __init__ excluded).

    Raises PluginFault(module, reason) when the directory is missing, a module does not import,
    FEATURES is missing or mistyped, a name is exported twice (within or across modules), or a
    name has no <features_dir>/<name>.md.
    """
    try:
        entries = sorted(os.listdir(plugins_dir))
    except OSError as e:
        raise PluginFault("drive.d", f"no plugin directory {plugins_dir}: {e.strerror}") from e
    served = {}
    for fn in entries:
        if not fn.endswith(".py") or fn == "__init__.py":
            continue
        stem = fn[:-3]
        spec = importlib.util.spec_from_file_location(f"drive_d.{stem}", os.path.join(plugins_dir, fn))
        mod = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = mod
        try:
            spec.loader.exec_module(mod)
        except Exception as e:  # a plugin that does not import is a named fault, never a skip
            raise PluginFault(stem, f"import failed: {type(e).__name__}: {e}") from e
        for name, proc in _features_of(stem, mod):
            if name in served:
                raise PluginFault(stem, f"duplicate feature {name}")
            if not os.path.isfile(os.path.join(features_dir, name + ".md")):
                raise PluginFault(stem, f"feature {name} has no file {name}.md in {features_dir}")
            served[name] = proc
    return served
