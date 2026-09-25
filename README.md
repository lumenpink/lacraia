# La Craia

[![CCCP Compliant](https://img.shields.io/badge/CCCP-v2.4.0-blue.svg)](https://cccp.2lp.in)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPLv3-yellow.svg)](LICENSE)
[![Normative Standard: GOST 1.0](https://img.shields.io/badge/GOST-SPEC--01-ff0055.svg)](SPEC.md)
[![Docs](https://img.shields.io/badge/docs-lacraia.2lp.in-green.svg)](https://lacraia.2lp.in)

> **Autonomous Soundwall, Acoustic Affinity Radio & Symbiotic Lyric Ecosystem**

La Craia is a dissident audio suite integrating acoustic similarity streaming, proactive audio caching, and synchronized lyrics projected line-by-line via the Simbiótyka floating companion over any application or interface. Engineered for universal operation across Linux desktop, home servers, soundwalls, web, mobile, vans, and road trips.

---

## Technical Specification & GOST Conformance

This repository conforms to the **GOST (General Open Source Standard)** and the **Lumen Implementation Profile** (`profiles/lumen.md`). Complete normative architectural contracts, SQLite data models, WebAudio visualizer mechanics, RBAC multi-cabin security, and testing strategies are defined in:

👉 **[SPEC-01: La Craia Technical Specification (SPEC.md)](SPEC.md)**

---

## The Patron: Lacraia (1977–2011)

<p align="center">
  <img src="docs/assets/lacraia-halftone.jpg" alt="Lacraia · Patrona do Paredão" width="360" />
</p>

**La Craia** is named in homage to **Lacraia** (1977–2011), an iconic Brazilian dancer, performer, electronic music DJ, and proud black travesti who emerged from Rio de Janeiro's periphery (favela do Arará) to become a transformative cultural phenomenon in Brazilian funk carioca.

Alongside MC Serginho in the early 2000s, Lacraia electrified national stages and mainstream television with explosive anthems such as *"Vai Lacraia"* and *"Éguinha Pocotó"*. Her radical stage presence—marked by high-speed acrobatic choreography, aerial splits, vibrant neon satin, and uncontainable joy—shattered social and cultural barriers in an era of deep prejudice, claiming visibility and dignity for travestis and LGBTQIA+ communities across Brazil.

Far from accepting sanitization or conforming to commercial expectations, she weaponized performance and humor as pure freedom. In her later years, she sustained this underground energy as a resident electronic music DJ in Rio and São Paulo nightclubs.

### Why La Craia?

Our software ecosystem inherits her spirit:
- **Autonomous & Peripheral:** Operating outside centralized streaming silos and corporate telemetry, built from the street up like a sound wall (*paredão*).
- **Radical Joy & Speed:** Fast, responsive, low-latency audio delivery engineered with Rust and WebAssembly.
- **Unapologetic Sovereignty:** Independent, decentralized, and community-driven.

---

## Workspace Structure

The project is structured as a monorepo across Rust crates and a native Android micro-application:

```
lacraia/
├── SPEC.md                # Normative Technical Specification (GOST SPEC-01)
├── packages/
│   ├── lacraia-shared/    # Shared models, LRC parser, fuzzy validator, WS protocol (Rust)
│   ├── lacraia-core/      # Backend service: Axum, SQLite, yt-dlp, Last.fm, WS hub (Rust)
│   ├── lacraia-app/       # Universal player, soundwall & cabin console in Dioxus (Linux Native & Web/PWA)
│   └── lacraia-android/   # Native Android workspace: :app (Player) & :simbiotyka (Floating Lyric HUD)
├── docs/                  # Design system, identity manual and release roadmaps
├── site/                  # Static documentation site deployed to Cloudflare Workers
├── tests/e2e/             # Automated headless Playwright E2E test suite (7 scenarios)
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

# Run exhaustive Playwright E2E suite (13 comprehensive UI, audio, and autopilot scenarios)
node tests/e2e/run_tests.js
```

### Running Services

```bash
# Start backend daemon (default port: 8080)
cargo run -p lacraia-core

# Start backend on a custom port and directories with master key
PORT=8081 LACRAIA_MASTER_KEY=my_master_key LACRAIA_CACHE_DIR=/var/cache/la-craia LACRAIA_DB_PATH=./lacraia.sqlite cargo run -p lacraia-core
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
