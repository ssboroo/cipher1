# Autonomous NEXUS R&D runner

The hourly workflow requires a model API credential. GitHub Actions cannot call a model or autonomously author code without one.

## One-time setup

1. Create an API key in the OpenAI Platform.
2. Open this repository's Settings, then Secrets and variables, then Actions.
3. Add a repository secret named `OPENAI_API_KEY`.
4. Add a repository variable named `OPENAI_MODEL` with an exact model identifier enabled for your API project. There is intentionally no assumed default model identifier.
5. In Settings → Actions → General, enable **Allow GitHub Actions to create and approve pull requests** if you want automatic PR creation.
6. Open Actions, select Autonomous NEXUS R&D, and run the workflow manually once.

Never put an API key in source code, workflow YAML, issues, or commit messages. API usage is billed separately from a ChatGPT subscription.

## Runtime behavior

- Scheduled hourly at minute 17 UTC; GitHub may delay scheduled starts.
- Uses the OpenAI Responses API to propose one small patch per run.
- The model may modify only existing Rust source files under `crates/*/src/`, Markdown files under `docs/`, and `README.md`. The commit step may additionally stage rustfmt-only changes to existing `crates/*/benches/*.rs` files. It rejects new/deleted/renamed files, manifests, lockfiles, build scripts, and workflow changes.
- Tests run without the OpenAI API key or GitHub write token in their environment; checkout credentials are not persisted.
- Runs formatting, workspace tests, and Clippy before committing.
- Pushes a dedicated `autonomous/nexus-*` branch and attempts to open a pull request. The repository setting **Settings → Actions → General → Workflow permissions → Allow GitHub Actions to create and approve pull requests** must be enabled for automatic PR creation. If disabled, the branch and cross-platform CI still run, but the PR step reports a warning.
- Never pushes model-generated code directly to `main` and never auto-merges.
- If no justified patch is proposed, it exits without a commit.
- If either `OPENAI_API_KEY` or `OPENAI_MODEL` is absent, AI patch generation is skipped and a warning is emitted; validation and commit stages are also skipped.

## Safety and limitations

The agent sees only a bounded selection of repository files. It has no web search or independent prior-art verification. Model output is untrusted and must be reviewed. Passing CI does not prove mathematical novelty, security, or production suitability.

GitHub Actions schedules are best-effort, not real-time guarantees. Protect `main` with required reviews and status checks before enabling this workflow.
