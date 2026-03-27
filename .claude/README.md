# .claude/ Directory — Shadow Project Configuration

This directory configures Claude Code for the Shadow project.
Everything here is checked into version control (except files listed in .gitignore).

## File Map

```
.claude/
├── .gitignore                        # Excludes settings.local.json and agent-memory/
│
├── settings.json                     # Hooks, permissions, environment variables
├── settings.local.json               # GITIGNORED — personal overrides (copy from template)
│
├── MILESTONE.md                      # Current active milestone and exit criteria
│                                       Update this as milestones complete
│
├── agents/                           # Subagents — specialised AI assistants
│   ├── rust-daemon.md                # Rust backend expert (daemon, providers, IPC)
│   ├── frontend.md                   # React/TypeScript UI expert
│   ├── cicd.md                       # GitHub Actions, releases, versioning
│   └── code-reviewer.md              # Read-only code review and audit agent
│
├── commands/                         # Slash commands — /command-name
│   ├── new-feature.md                # /new-feature <description> — scaffold a feature
│   ├── git/
│   │   └── commit.md                 # /git:commit — quality-gated commit workflow
│   ├── rust/
│   │   └── new-provider.md           # /rust:new-provider <Name> — scaffold a provider
│   └── release/
│       └── tag.md                    # /release:tag v1.2.3 — cut a release
│
└── skills/                           # Auto-activating knowledge modules
    ├── rust-patterns/
    │   └── SKILL.md                  # Rust patterns: error handling, async, providers,
    │                                   blake3, sled, debouncer, remote key construction
    ├── tauri-ipc/
    │   └── SKILL.md                  # Tauri 2 IPC: commands, events, typed wrappers,
    │                                   app_handle, hook patterns
    ├── react-patterns/
    │   └── SKILL.md                  # React: Zustand stores, hooks, components,
    │                                   activity feed, provider cards, Tailwind patterns
    └── provider-patterns/
        └── SKILL.md                  # BackupProvider trait, upload strategies,
                                        multipart S3, resumable GCS, NAS copy
```

## How Each Piece Works

### CLAUDE.md (project root)
Loaded automatically at the start of every session. Contains project overview,
directory structure, build commands, and all non-negotiable coding rules.
Think of it as the always-on context.

### settings.json
Controls three things:
- **permissions**: what bash commands Claude can run (allow/deny list)
- **hooks**: automated actions at lifecycle events (PreToolUse, PostToolUse, Stop)
- **env**: environment variables set for every session

Current hooks:
- Blocks direct edits to the `main` branch
- Reminds to run clippy/type-check after editing Rust/TS files
- Prints a reminder at session end

### Agents (subagents)
Specialised AI instances with their own focused system prompts. Use them by
delegating tasks: "Use the rust-daemon agent to implement the S3 provider."

Claude will automatically delegate to the right agent based on the task context.
You can also invoke explicitly: `@rust-daemon implement the blake3 hasher`

### Commands (slash commands)
Triggered manually with `/command-name`. Useful for repeatable workflows.

| Command | Usage |
|---|---|
| `/new-feature` | `/new-feature add retry count to UI` |
| `/git:commit` | `/git:commit` |
| `/rust:new-provider` | `/rust:new-provider Backblaze` |
| `/release:tag` | `/release:tag v1.0.0` |

### Skills
Activate automatically when Claude detects the task matches the skill description.
You don't invoke them manually — Claude loads them when relevant.

- Writing Rust code → `rust-patterns` skill loads
- Writing Tauri commands or IPC → `tauri-ipc` skill loads
- Writing React components or hooks → `react-patterns` skill loads

## Adding New Configuration

**New agent:** Create `.claude/agents/my-agent.md` with YAML frontmatter (name, description, allowed-tools).

**New command:** Create `.claude/commands/my-command.md`. Use `$1`, `$2` for arguments.

**New skill:** Create `.claude/skills/my-skill/SKILL.md` with YAML frontmatter (name, description).

**Modify hooks:** Edit `.claude/settings.json` — hooks section.
