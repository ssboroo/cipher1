#!/usr/bin/env python3
"""Generate one constrained patch for the CIPHER-1 NEXUS research repository."""
import json
import os
import subprocess
import sys
import urllib.error
import urllib.request

API_URL = "https://api.openai.com/v1/responses"
MODEL = os.environ.get("OPENAI_MODEL", "gpt-6-astra")
MAX_CONTEXT_CHARS = 45000

def run(*args):
    return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT)

def read_file(path):
    try:
        return run("git", "show", "HEAD:" + path)
    except subprocess.CalledProcessError:
        return ""

def main():
    key = os.environ.get("OPENAI_API_KEY")
    if not key:
        print("OPENAI_API_KEY is required", file=sys.stderr)
        return 2
    paths = [
        "README.md",
        "crates/nexus-transform/src/lib.rs",
        "crates/nexus-transform/Cargo.toml",
        "docs/NEXUS_MATHEMATICAL_RESEARCH.md",
        "docs/NEXUS_ALGORITHM_SPEC.md",
        "docs/NEXUS_VALIDATION_PLAN.md",
        "docs/NARF_BASELINE_BENCHMARK.md",
    ]
    sections = []
    for path in paths:
        content = read_file(path)
        if content:
            sections.append("===== " + path + " =====\n" + content)
    context = "\n\n".join(sections)[:MAX_CONTEXT_CHARS]
    instructions = (
        "You are the cautious research engineer for CIPHER-1 NEXUS. Produce at most one small, reviewable, technically justified improvement. "
        "All repository artifacts must be English. Do not claim world-first novelty, cryptographic security, quantum resistance, production readiness, or superiority without evidence. "
        "NARF is a public reversible preprocessing transform, not encryption. Do not invent cryptographic primitives or add dependencies without strong reason. "
        "Do not modify CI workflow files, scripts/autonomous_rnd.py, Cargo.lock, or files outside crates/, docs/, and README.md. "
        "Prefer a correctness fix, meaningful test, precise mathematical clarification, or reproducible benchmark improvement. "
        "Return ONLY a unified diff with diff --git headers, or exactly NO_CHANGE. Never fabricate test results."
    )
    user_input = "Repository: " + os.environ.get("GITHUB_REPOSITORY", "ssboroo/cipher1") + "\nCommit: " + os.environ.get("GITHUB_SHA", "unknown") + "\n\nCurrent files:\n" + context + "\n\nIdentify one concrete improvement and return a minimal patch or NO_CHANGE."
    payload = {"model": MODEL, "instructions": instructions, "input": user_input, "max_output_tokens": 6000}
    request = urllib.request.Request(API_URL, data=json.dumps(payload).encode(), headers={
        "Authorization": "Bearer " + key, "Content-Type": "application/json"}, method="POST")
    try:
        with urllib.request.urlopen(request, timeout=180) as response:
            result = json.load(response)
    except urllib.error.HTTPError as exc:
        print("OpenAI API request failed (" + str(exc.code) + "): " + exc.read().decode("utf-8", "replace"), file=sys.stderr)
        return 3
    output = []
    for item in result.get("output", []):
        if item.get("type") == "message":
            for part in item.get("content", []):
                if part.get("type") == "output_text":
                    output.append(part.get("text", ""))
    patch = "\n".join(output).strip()
    if not patch or patch == "NO_CHANGE":
        print("No justified patch proposed.")
        return 0
    if patch.startswith("```"):
        lines = patch.splitlines()
        if lines and lines[0].startswith("```"): lines = lines[1:]
        if lines and lines[-1].strip() == "```": lines = lines[:-1]
        patch = "\n".join(lines).strip()
    if "diff --git " not in patch:
        print("Model output was not a unified diff; refusing to apply it.", file=sys.stderr)
        return 4
    if any(x in patch for x in ("../", "/etc/", ".github/workflows/", "scripts/autonomous_rnd.py")):
        print("Patch contains a forbidden path; refusing to apply it.", file=sys.stderr)
        return 5
    with open("/tmp/nexus-agent.patch", "w", encoding="utf-8") as handle:
        handle.write(patch + "\n")
    subprocess.check_call(["git", "apply", "--check", "/tmp/nexus-agent.patch"])
    subprocess.check_call(["git", "apply", "/tmp/nexus-agent.patch"])
    print("Applied one candidate patch. CI must validate it before commit.")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
