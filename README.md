# Gitr

The Git sister of [Diffr](https://github.com/crussella0129/Diffr) — sync all your GitHub forks with their newest versions at the push of a button.

Gitr is a Rust CLI tool for managing git repos across multiple hosting services (GitHub, GitLab, Gitea, Bitbucket, Azure DevOps). It discovers your repos via API and filesystem scan, tracks fork relationships, and keeps everything in sync with configurable merge strategies.

## Quick Start

```bash
# Build from source
cargo build --release -p gitr-cli

# Initialize config and database
gitr config init

# Register your GitHub account (stores token in OS keychain)
gitr host add gh --provider github --user <your-username> --token <your-pat>

# Discover all your repos (including forks)
gitr scan --host gh

# See what's out of date
gitr status

# Sync a single fork
gitr sync <repo-name> --dry-run   # preview first
gitr sync <repo-name>             # fast-forward + push

# Sync all forks at once
gitr sync all
```

## Usage

### Configuration

```bash
gitr config init    # creates ~/.gitr/ with config.toml + gitr.db
gitr config show    # print current config
```

Config lives at `~/.gitr/config.toml`:

```toml
default_merge_strategy = "ff"
sync_concurrency = 8
scan_paths = []
max_scan_depth = 4
```

### Host Management

```bash
gitr host add <label> --provider github --user <username>   # prompts for token
gitr host add gh --provider github --user alice --token ghp_...
gitr host list              # table of registered hosts
gitr host info <label>      # details + repo counts
gitr host verify <label>    # test credentials + show rate limit
gitr host remove <label>    # delete host + credentials
```

Tokens are stored in the OS keychain (Windows Credential Manager / macOS Keychain / Linux Secret Service) and never touch disk in plaintext.

### Scanning & Discovery

```bash
gitr scan --host gh              # discover repos via GitHub API
gitr scan --path ~/projects      # also scan local filesystem
gitr scan                        # scan all hosts + configured paths
```

Scan reconciles local repos with remote APIs:
- **Matched** — found both locally and on the host
- **Local-only** — on disk but not on any registered host
- **Remote-only** — on the host but not cloned locally

Discovered repos are automatically tracked in the local database.

### Repo Management

```bash
gitr repo list                   # all tracked repos
gitr repo list --forks           # only forks
gitr repo list --host gh         # filter by host
gitr repo info <name>            # full details + branch status
```

### Syncing

```bash
gitr sync <repo>                 # sync a single fork (fast-forward by default)
gitr sync <repo> --strategy merge      # merge instead of ff
gitr sync <repo> --strategy rebase     # rebase instead of ff
gitr sync <repo> --dry-run             # show behind count without changing anything
gitr sync all                    # sync all tracked forks in parallel
gitr sync all --dry-run          # preview all
```

**Fork sync flow:**
1. Clone if not already local
2. Add `upstream` remote pointing to parent repo
3. `git fetch upstream --prune`
4. Checkout default branch
5. Apply strategy (fast-forward / merge / rebase)
6. `git push origin`
7. Record result in database

Parallel sync uses a configurable concurrency limit (default 8).

### Status Dashboard

```bash
gitr status                      # all hosts
gitr status --host gh            # single host
```

```
HOST / REPO              BRANCH   BEHIND  AHEAD  STRATEGY  LAST SYNC  STATUS
gh (github)
  linux                  main        3       0   ff        12:00      behind
  rust                   main        0       0   ff        12:00      synced
  react                  main        0       2   ff        11:45      ahead

Summary: 298 synced | 35 behind | 2 ahead | 0 errors
```

### Sync History

```bash
gitr history                     # recent sync operations
gitr history <repo>              # history for a specific repo
gitr history --limit 50          # show more records
```

## Architecture

7-crate Rust workspace:

```
gitr-core      Shared models, config, error types
gitr-auth      OS keychain credential management (keyring crate)
gitr-db        SQLite (WAL mode) schema, migrations, CRUD
gitr-host      HostProvider trait + GitHub implementation
gitr-discover  Filesystem scanner + API discovery + reconciliation
gitr-sync      Git CLI wrappers, fork sync executor, parallel engine
gitr-cli       Clap CLI with all commands
```

Git operations shell out to `git` via `std::process::Command` (not libgit2) for maximum compatibility with SSH keys, credential helpers, and GPG signing.

## Roadmap

### Phase 2: Multi-Host & Mirroring
- GitLab, Gitea, Bitbucket, Azure DevOps provider implementations
- Cross-host sync links (`gitr link add source target`)
- Any-to-any mirroring with directed sync graph
- Cycle detection and topological execution order

### Phase 3: TUI & Daemon
- Ratatui interactive dashboard (`gitr status --watch`)
- Scheduled auto-sync via cron expressions
- systemd / launchd service integration
- Desktop notifications on sync failures

### Phase 4: Collections & Advanced
- User-defined repo collections (`gitr collection create`, `gitr collection add`)
- Branch filtering (include/exclude patterns)
- Transform rules (branch renaming, path filtering)
- Webhook-triggered sync

## License

See [LICENSE](LICENSE) for details.
