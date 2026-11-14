# Autonomous NEXUS R&D runner

The hourly workflow requires a model API credential. GitHub Actions cannot call a model or autonomously author code without one.

## One-time setup

1. Create an API key in the OpenAI Platform.
2. Open this repository's Settings, then Secrets and variables, then Actions.
3. Add a repository secret named `OPENAI_API_KEY`.
4. Optionally add a repository variable named `OPENAI_MODEL` to override the default model identifier. The selected model must be available to the API project.
5. Open Actions, select Autonomous NEXUS R&D, and run the workflow manually once.

Never put an API key in source code, workflow YAML, issues, or commit messages. API usage is billed separately from a ChatGPT subscription.

## Runtime behavior

- Scheduled hourly at minute 17 UTC; GitHub may delay scheduled starts.
- Uses the OpenAI Responses API to propose one small patch per run.
- Only allows edits under `crates/`, `docs/`, and `README.md`.
- Runs formatting, workspace tests, and Clippy before committing.
- Pushes a dedicated `autonomous/nexus-*` branch and opens a pull request.
- Never pushes model-generated code directly to `main` and never auto-merges.
- If no justified patch is proposed, it exits without a commit.
- If `OPENAI_API_KEY` is absent, the AI job is skipped and a warning is emitted.

## Safety and limitations

The agent sees only a bounded selection of repository files. It has no web search or independent prior-art verification. Model output is untrusted and must be reviewed. Passing CI does not prove mathematical novelty, security, or production suitability.

GitHub Actions schedules are best-effort, not real-time guarantees. Protect `main` with required reviews and status checks before enabling this workflow.
