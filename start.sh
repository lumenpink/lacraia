#!/bin/sh
# =============================================================================
# La Craia Unified Automotive Orchestrator (start.sh)
# Spins up the lacraia-core daemon, seeds demo state, and serves the
# standalone Cockpit Web & PWA player at /app.
# =============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PORT="${PORT:-8080}"
OPEN_BROWSER=0
SEED_DB=1
SITE_DIR="${LACRAIA_SITE_DIR:-$SCRIPT_DIR/site}"

while [ $# -gt 0 ]; do
    case "$1" in
        -p|--port)
            PORT="$2"
            shift 2
            ;;
        -o|--open)
            OPEN_BROWSER=1
            shift
            ;;
        --no-seed)
            SEED_DB=0
            shift
            ;;
        --seed)
            SEED_DB=1
            shift
            ;;
        --site-dir)
            SITE_DIR="$2"
            shift 2
            ;;
        -h|--help)
            echo "La Craia Unified Launcher - Cockpit Web & Core Daemon"
            echo ""
            echo "Usage: $0 [options] [port]"
            echo ""
            echo "Options:"
            echo "  -p, --port <number>      Port to bind backend and web app (default: 8080)"
            echo "  -o, --open               Open browser at http://localhost:<port>/app automatically"
            echo "  --seed                   Seed demo tracks & graph into SQLite (default: on)"
            echo "  --no-seed                Skip database seeding"
            echo "  --site-dir <path>        Static site directory to serve (default: ./site)"
            echo "  -h, --help               Show this help message"
            echo ""
            exit 0
            ;;
        [0-9]*)
            PORT="$1"
            shift
            ;;
        *)
            echo "Error: Unknown argument '$1'. Run with --help for options." >&2
            exit 1
            ;;
    esac
done

# Check if port is in use and auto-increment if necessary
while command -v lsof >/dev/null 2>&1 && lsof -i :"$PORT" >/dev/null 2>&1; do
    echo "Notice: Port $PORT is occupied, trying port $((PORT + 1))..."
    PORT=$((PORT + 1))
done

DB_PATH="${LACRAIA_DB_PATH:-$SCRIPT_DIR/lacraia.sqlite}"
CACHE_DIR="${LACRAIA_CACHE_DIR:-$SCRIPT_DIR/cache}"
mkdir -p "$CACHE_DIR"

# Seed SQLite database if requested
if [ "$SEED_DB" -eq 1 ] && command -v python3 >/dev/null 2>&1; then
    python3 - <<PY
import sqlite3, time

db_path = "$DB_PATH"
conn = sqlite3.connect(db_path)
cur = conn.cursor()

cur.execute("""
CREATE TABLE IF NOT EXISTS tracks (
    id TEXT PRIMARY KEY,
    artist TEXT NOT NULL,
    title TEXT NOT NULL,
    duration_sec INTEGER NOT NULL,
    cached_path TEXT,
    genres TEXT,
    last_played_at INTEGER
);
""")
cur.execute("""
CREATE TABLE IF NOT EXISTS similarity_edges (
    source_track_id TEXT NOT NULL,
    target_track_id TEXT NOT NULL,
    weight REAL NOT NULL,
    provider TEXT NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (source_track_id, target_track_id)
);
""")
cur.execute("""
CREATE TABLE IF NOT EXISTS queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    track_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    state TEXT NOT NULL DEFAULT 'pending',
    created_at INTEGER NOT NULL,
    cabin_id TEXT NOT NULL DEFAULT 'default',
    requested_by TEXT
);
""")

cur.execute("SELECT COUNT(*) FROM tracks;")
count = cur.fetchone()[0]

if count == 0:
    demo_tracks = [
        ("trk_1", "Chico Buarque", "Construção", 258, None, "MPB, Samba"),
        ("trk_2", "Elis Regina & Tom Jobim", "Águas de Março", 212, None, "Bossa Nova, MPB"),
        ("trk_3", "João Gilberto", "Chega de Saudade", 121, None, "Bossa Nova"),
        ("trk_4", "Daft Punk", "One More Time", 320, None, "Electronic, House"),
        ("trk_5", "Kraftwerk", "Autobahn", 1360, None, "Electronic, Krautrock")
    ]
    cur.executemany("""
    INSERT OR REPLACE INTO tracks (id, artist, title, duration_sec, cached_path, genres, last_played_at)
    VALUES (?, ?, ?, ?, ?, ?, NULL);
    """, demo_tracks)

    now = int(time.time())
    edges = [
        ("trk_1", "trk_2", 0.94, "demo", now),
        ("trk_2", "trk_3", 0.96, "demo", now),
        ("trk_3", "trk_1", 0.88, "demo", now),
        ("trk_4", "trk_5", 0.89, "demo", now),
        ("trk_5", "trk_4", 0.89, "demo", now)
    ]
    cur.executemany("""
    INSERT OR REPLACE INTO similarity_edges (source_track_id, target_track_id, weight, provider, updated_at)
    VALUES (?, ?, ?, ?, ?);
    """, edges)

    cur.execute("SELECT COUNT(*) FROM queue;")
    if cur.fetchone()[0] == 0:
        for idx, trk_id in enumerate(["trk_1", "trk_2", "trk_3"]):
            cur.execute("INSERT INTO queue (track_id, position, state, created_at) VALUES (?, ?, 'pending', ?);",
                        (trk_id, idx, now))

    conn.commit()
    print("✓ Banco SQLite populado com cantigas de demonstração e fila inicial.")
else:
    print(f"✓ Banco SQLite ativo ({count} faixa(s) cadastradas).")
conn.close()
PY
fi

BACKEND_PID=""

cleanup() {
    echo ""
    echo "Encerrando serviços da La Craia..."
    if [ -n "$BACKEND_PID" ] && kill -0 "$BACKEND_PID" 2>/dev/null; then
        kill "$BACKEND_PID" 2>/dev/null || true
        wait "$BACKEND_PID" 2>/dev/null || true
    fi
    echo "✓ Processos finalizados com segurança."
}

trap cleanup INT TERM EXIT

echo "Iniciando La Craia Core na porta $PORT..."
PORT="$PORT" LACRAIA_CACHE_DIR="$CACHE_DIR" LACRAIA_DB_PATH="$DB_PATH" LACRAIA_SITE_DIR="$SITE_DIR" cargo run -p lacraia-core &
BACKEND_PID=$!

# Probe health endpoint
HEALTH_URL="http://localhost:$PORT/health"
APP_URL="http://localhost:$PORT/app"
PORTAL_URL="http://localhost:$PORT"
WS_URL="ws://localhost:$PORT/ws"

READY=0
for i in $(seq 1 40); do
    if command -v curl >/dev/null 2>&1; then
        STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$HEALTH_URL" || true)
        if [ "$STATUS" = "200" ]; then
            READY=1
            break
        fi
    fi
    sleep 0.5
done

if [ "$READY" -ne 1 ]; then
    echo "Erro: O servidor da La Craia não respondeu no prazo esperado." >&2
    exit 1
fi

cat << EOF

    ░█░░░█▀█░░░█▀▀░█▀▄░█▀█░▀█▀░█▀█
    ░█░░░█▀█░░░█░░░█▀▄░█▀█░░█░░█▀█
    ░▀▀▀░▀░▀░░░▀▀▀░▀░▀░▀░▀░▀▀▀░▀░▀

  La Craia :: Cockpit Automotivo Web & Core Daemon (v0.2.0)
  Suíte Automotiva Dissidente & Streaming por Afinidade

  ★ Cockpit Web / PWA:  $APP_URL
  ★ Portal Documentação: $PORTAL_URL
  ★ Barramento WebSocket: $WS_URL
  ★ Sonda de Saúde:     $HEALTH_URL
  ★ Banco SQLite:       $DB_PATH
  ★ Cache de Áudio:     $CACHE_DIR

[Pressione Ctrl+C para encerrar o ambiente]

EOF

if [ "$OPEN_BROWSER" -eq 1 ]; then
    if command -v xdg-open >/dev/null 2>&1; then
        xdg-open "$APP_URL" >/dev/null 2>&1 &
    elif command -v open >/dev/null 2>&1; then
        open "$APP_URL" >/dev/null 2>&1 &
    fi
fi

wait "$BACKEND_PID"
