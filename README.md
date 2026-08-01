# statsvg-rs

A Rust renderer that produces styled SVG GitHub stats cards and publishes them as static files via GitHub Pages — no servers, no hosting bills, no API rate limits to manage. A scheduled GitHub Action re-renders the cards every 6 hours.

## Live demo

These two SVGs are this repo's own output, re-rendered every 6 hours by the workflow:

**`profile.svg`** — for embedding in any of your repo READMEs:

![Profile card](https://humbertcostas.github.io/statsvg_rs/profile.svg)

**`stats.svg`** — for your profile README (anti-profile: lifetime totals, contributed-to repos, most-starred highlight):

![Lifetime stats](https://humbertcostas.github.io/statsvg_rs/stats.svg)

Embed either with:

```markdown
![Stats](https://humbertcostas.github.io/statsvg_rs/profile.svg)
![Lifetime stats](https://humbertcostas.github.io/statsvg_rs/stats.svg)
```

## Two variants, one workflow

Each render produces both cards. Pick the one that fits the embed context:

### `profile.svg` — for any repo README

Header-heavy, includes avatar, bio, location, pinned repos, contribution grid. Use when the reader doesn't know who you are yet.

### `stats.svg` — for your profile README ("anti-profile")

Skips identity duplicates (no avatar, no bio — the reader is on your profile and sees those already). Shows what GitHub deliberately hides: **lifetime contributions since you joined**, longest streak, all-time aggregate stars/commits/PRs, **top repos you contributed to but don't own**, your most-starred single repo as a highlight tile.

## Setup (one time)

1. **Fork** this repo and clone it.
2. **Edit `.github/workflows/render.yml`** — set:
   ```yaml
   env:
     STATSVG_USER: your-github-login
     STATSVG_THEME: github_dark        # or nord / dracula / light / solarized
     STATSVG_WIDTH: '680'              # any integer 360–1600
     STATSVG_HIGHLIGHT: ''             # optional callout, e.g. "Currently building X"
   ```
3. **(Optional) Private repo data**: generate a [classic PAT](https://github.com/settings/tokens) with `repo` scope, add it as repo secret `STATS_GH_TOKEN`. Without it, only public data is included (using the auto-provided `GITHUB_TOKEN`).
4. **Enable GitHub Pages**: repo Settings → Pages → Source = **GitHub Actions**.
5. **Push to main** — workflow renders and publishes. Your URLs:
   ```
   https://<your-user>.github.io/<repo>/profile.svg
   https://<your-user>.github.io/<repo>/stats.svg
   ```

The workflow re-runs every 6 hours; you can also trigger it manually under **Actions → Render and publish stats SVG → Run workflow**.

## Embed in your READMEs

In any of your repo READMEs:
```markdown
![Stats](https://your-user.github.io/statsvg_rs/profile.svg)
```

In your **profile README** (`your-user/your-user` repo):
```markdown
![Lifetime stats](https://your-user.github.io/statsvg_rs/stats.svg)
```

GitHub's image proxy caches each card. Append `?v=1` to bust if needed.

## CLI flags

```bash
cargo run --release -- render --user <login> [flags] > card.svg
```

| Flag                          | Default                    | Notes                                                       |
|-------------------------------|----------------------------|-------------------------------------------------------------|
| `--user <login>`              | *(required)*               | GitHub login                                                |
| `--variant <profile\|stats>`  | `profile`                  | preset section toggles + last-year vs lifetime stats        |
| `--theme <name>`              | `github_dark`              | `github_dark` · `nord` · `dracula` · `light` · `solarized`  |
| `--width <px>`                | `680`                      | card width, clamped 360–1600                                |
| `--highlight <text>`          | *(none)*                   | optional callout line under the stats row                   |
| `--top-repos-count <1..6>`    | `3`                        | how many pinned repos to show                               |
| `--no-header`                 | off                        | hide profile header                                         |
| `--no-stats`                  | off                        | hide the 6-cell stats row                                   |
| `--no-grid`                   | off                        | hide the contribution heatmap                               |
| `--no-languages`              | off                        | hide the top-languages bars                                 |
| `--no-top-repos`              | off                        | hide pinned repos                                           |
| `--show-contributed-to`       | off (profile) / on (stats) | show top repos you've contributed to but don't own          |
| `--show-most-starred`         | off (profile) / on (stats) | highlight your single most-starred owned repo               |
| `--no-border`                 | off                        | hide the outer rounded border                               |

The same toggles exist as query-string params (`show_header=false`, `variant=stats`, etc.) when you run the HTTP server mode locally.

## Local development

```bash
cp .env.example .env
# Add a GitHub classic PAT to GH_TOKEN

# Render either variant to stdout
cargo run --release -- render --user humbertcostas --variant profile > profile.svg
cargo run --release -- render --user humbertcostas --variant stats --highlight "Building cool things" > stats.svg
open profile.svg    # macOS  (xdg-open on Linux)

# Or run as an HTTP server for iterating on layout
cargo run
curl "http://localhost:3000/api?username=humbertcostas&variant=stats" -o card.svg
```

## What's on the cards (and what isn't)

| Section                                         | `profile.svg` | `stats.svg`   |
|-------------------------------------------------|---------------|---------------|
| Avatar (real photo, base64-embedded)            | ✓             | —             |
| Name, login, bio, location                      | ✓             | —             |
| Followers / following pill                      | ✓             | —             |
| Member-since · years on GitHub                  | ✓             | —             |
| 6 stat cells (last-year totals)                 | ✓             | —             |
| 6 stat cells (**lifetime** since you joined)    | —             | ✓             |
| 18-week contribution grid                       | ✓             | —             |
| Top languages bars                              | ✓             | ✓             |
| Pinned repos                                    | ✓             | —             |
| **Top repos you contributed to** (not your own) | —             | ✓             |
| **Most-starred owned repo** highlight           | —             | ✓             |
| Custom highlight line                           | optional      | optional      |
| Outer border                                    | ✓ (toggle)    | ✓ (toggle)    |
| Updated date in footer                          | ✓             | ✓             |

## Adding a theme

Edit `src/theme.rs`: declare a `pub const MY_THEME: Theme = Theme { ... }` and add `&MY_THEME` to `ALL_THEMES`. The `--theme` flag and `/themes` endpoint pick it up automatically.

## Project layout

```
src/
  main.rs    — CLI entry: `render` subcommand + HTTP server mode
  github.rs  — GraphQL types, fetcher, stat computation, lifetime aggregation, avatar fetch
  svg.rs     — SVG string builder + per-section renderers
  theme.rs   — Theme struct + built-in theme constants
  params.rs  — RenderConfig, Variant enum, query/CLI parsing
  error.rs   — AppError enum + IntoResponse impl
.github/workflows/
  render.yml — cron + push → renders both variants, deploys to GitHub Pages
```

The original implementation spec lives in [`github-stats-rs-claude-code-prompt.md`](./github-stats-rs-claude-code-prompt.md). The repo has diverged from it (variants, lifetime stats, avatar embed, contributed-to, etc.) — the spec is reference for the original baseline only.
