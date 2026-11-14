#!/usr/bin/env python3
"""Generate one constrained patch for the CIPHER-1 NEXUS research repository."""
import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.request

API_URL = "https://api.openai.com/v1/responses"
MODEL = os.environ.get("OPENAI_MODEL", "")
MAX_CONTEXT_CHARS = 80000

def run(*args):
    return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT)

def set_output(name, value):
    output_path = os.environ.get("GITHUB_OUTPUT")
    if output_path:
        with open(output_path, "a", encoding="utf-8") as handle:
            handle.write(f"{name}={value}\\n")


def read_file(path):
    try:
        return run("git", "show", "HEAD:" + path)
    except subprocess.CalledProcessError:
        return ""

def main():
    key = os.environ.get("OPENAI_API_KEY")
    if not key:
        print("::warning::OPENAI_API_KEY is not configured; skipping AI patch generation.")
        return 0
    if not MODEL:
        print("::warning::OPENAI_MODEL is not configured; skipping AI patch generation.")
        return 0
    paths = [
        "README.md",
        "crates/nexus-transform/src/lib.rs",
        "crates/cipher1-core/src/lib.rs",
        "crates/cipher1-core/src/aead.rs",
        "docs/NEXUS_MATHEMATICAL_RESEARCH.md",
        "docs/NEXUS_ALGORITHM_SPEC.md",
        "docs/NEXUS_VALIDATION_PLAN.md",
        "docs/NARF_BASELINE_BENCHMARK.md",
        "docs/NARF_BASELINE_RESULTS.md",
        "docs/ARCHITECTURE.md",
        "docs/THREAT_MODEL.md",
        "docs/C1AE_USAGE.md",
        "docs/PIPELINE_USAGE.md",
        "docs/NEXUS_FUZZING.md",
        "docs/SUPPLY_CHAIN_SECURITY.md",
        "docs/RESEARCH_STATUS.md",
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
        "NARF is a public reversible preprocessing transform, not encryption. C1AE is an integration of the standard XChaCha20-Poly1305 AEAD, not a custom cipher. Preserve its nonce, associated-data, output-limit, and zeroization invariants. Keep BLAKE3 at 1.8.7 or newer to avoid the compromised arrayref dependency; do not weaken the supply-chain gate or lower the MSRV. Never invent or alter cryptographic primitives; do not add dependencies without strong reason. "
        "Only propose edits to existing Rust source files under crates/*/src/ or Markdown files under docs/, plus README.md. Never add, delete, rename, or change file modes. Do not modify Cargo manifests, lockfiles, CI, scripts, or build scripts. "
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
        set_output("changed", "false")
        return 0
    if patch.startswith("```"):
        lines = patch.splitlines()
        if lines and lines[0].startswith("```"): lines = lines[1:]
        if lines and lines[-1].strip() == "```": lines = lines[:-1]
        patch = "\n".join(lines).strip()
    if "diff --git " not in patch:
        print("Model output was not a unified diff; refusing to apply it.", file=sys.stderr)
        return 4
    file_headers = re.findall(r"^diff --git a/(.*?) b/(.*?)$", patch, re.MULTILINE)
    if not file_headers:
        print("Patch has no parseable file headers; refusing to apply it.", file=sys.stderr)
        return 5
    for old_path, new_path in file_headers:
        parts = old_path.split("/")
        allowed = (
            old_path == "README.md"
            or (old_path.startswith("docs/") and old_path.endswith(".md"))
            or (len(parts) >= 4 and parts[0] == "crates" and parts[2] == "src"
                and old_path.endswith(".rs"))
        )
        if old_path != new_path or not allowed or any(part in ("", ".", "..") for part in parts):
            print("Patch path is outside the strict source/documentation allowlist.", file=sys.stderr)
            return 5
        try:
            subprocess.check_call(
                ["git", "cat-file", "-e", "HEAD:" + old_path],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        except subprocess.CalledProcessError:
            print("Patch may only modify existing files.", file=sys.stderr)
            return 5
    if "new file mode" in patch or "deleted file mode" in patch or "rename from " in patch:
        print("Patch may not add, delete, or rename files.", file=sys.stderr)
        return 5
    with open("/tmp/nexus-agent.patch", "w", encoding="utf-8") as handle:
        handle.write(patch + "\n")
    subprocess.check_call(["git", "apply", "--check", "/tmp/nexus-agent.patch"])
    subprocess.check_call(["git", "apply", "/tmp/nexus-agent.patch"])
    print("Applied one candidate patch. CI must validate it before commit.")
    set_output("changed", "true")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
