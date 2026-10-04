import os, subprocess, tempfile, textwrap
TOOLS = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

def run(*cmd, env=None, cwd=None, timeout=60):
    e = dict(os.environ); e.update(env or {})
    r = subprocess.run(cmd, capture_output=True, text=True, env=e, cwd=cwd, timeout=timeout)
    return r.returncode, r.stdout, r.stderr

def make_repo(gate_toml, files=None):
    """A throwaway git repo holding gate.toml; returns (dir, head_sha)."""
    d = tempfile.mkdtemp(prefix="gt-")
    for name, body in {"gate.toml": gate_toml, **(files or {})}.items():
        with open(os.path.join(d, name), "w") as f:
            f.write(body)
    g = lambda *a: subprocess.run(["git", "-C", d, "-c", "user.name=t", "-c", "user.email=t@t", *a], check=True, capture_output=True, text=True)
    g("init", "-q"); g("add", "-A"); g("commit", "-qm", "one")
    with open(os.path.join(d, "x.txt"), "w") as f:
        f.write("two\n")
    g("add", "-A"); g("commit", "-qm", "two")
    return d, g("rev-parse", "HEAD").stdout.strip()
