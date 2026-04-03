Create a bash script for setting up a development environment.

Include:
- OS detection (macOS vs Ubuntu/Debian)
- Install dependencies: Git, Node.js (via nvm), Python (via pyenv), Docker, PostgreSQL client
- Configure: create .env from .env.example, setup git hooks, initialize database
- Verify: check all tools installed with versions, run a smoke test
- Idempotent (safe to run multiple times)

Use section IDs: detect-os, install-deps, configure, verify

Use AAP section markers to delineate each major code block.
Wrap each logical section with `# region id` and `# endregion id`.


Output raw code only. No markdown fences, no explanation.