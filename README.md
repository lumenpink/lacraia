# La Craia

[![CCCP Compliant](https://img.shields.io/badge/CCCP-v2.3.0-blue.svg)](https://cccp.2lp.in)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Docs](https://img.shields.io/badge/docs-lacraia.2lp.in-green.svg)](https://lacraia.2lp.in)

> **Decentralized Automotive Audio & Heads-Up Display Ecosystem**

La Craia is a focused suite integrating acoustic similarity radio streaming, local audio caching, and synchronized lyrics projected line-by-line via a floating heads-up display (HUD) over navigation apps (Waze).

---

## Workspace Structure

The project is structured as a monorepo across Rust crates and a native Android micro-application:

```
lacraia/
├── packages/
│   ├── lacraia-shared/    # Shared models, LRC parser, fuzzy validator, WS protocol (Rust)
│   ├── lacraia-core/      # Backend service: Axum, SQLite, yt-dlp, Last.fm, WS hub (Rust)
│   ├── lacraia-app/       # Automotive player & cockpit UI in Dioxus (Linux Native & Web/PWA)
│   └── lacraia-android/   # Native Android workspace: :app (Player) & :simbiotyka (HUD Overlay)
├── docs/                  # Design system, identity manual and technical specifications
├── site/                  # Static documentation site deployed to Cloudflare Workers
├── serve.sh               # Local preview HTTP server for site/
└── wrangler.jsonc         # Cloudflare Workers configuration for lacraia.2lp.in
```

---

## Development & Conventional Commits

This repository enforces the **CCCP (Conventional Commits Compliance Program)**:

```bash
# Check repository and version status
cccp status

# Lint commit history
cccp lint

# Local preview of the documentation
./serve.sh --port 8080
```

### Building & Testing

```bash
# Run tests across all Rust crates
cargo test --workspace

# Lint Rust crates
cargo clippy --workspace
```

### Running Services

```bash
# Start backend daemon (default port: 8080)
cargo run -p lacraia-core

# Start backend on a custom port and directories
PORT=8081 LACRAIA_CACHE_DIR=/var/cache/la-craia LACRAIA_DB_PATH=./lacraia.sqlite cargo run -p lacraia-core
```

### Automated Cockpit Demo

```bash
# Launch backend daemon, seed SQLite database, and probe health endpoints
./demo.sh

# Specify custom starting port and automatically open browser
./demo.sh --port 8090 --open
```

---

## Continuous Integration & Deployment

Documentation and static assets are automatically published to Cloudflare Workers at [lacraia.2lp.in](https://lacraia.2lp.in) upon push to `main` via Forgejo Actions (`.forgejo/workflows/ci.yml`).

---

## License

MIT © [Lumen Pink](https://codeberg.org/lumenpink)
