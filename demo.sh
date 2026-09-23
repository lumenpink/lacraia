#!/bin/sh
# =============================================================================
# La Craia Cockpit & Audio Ecosystem Demo Launcher
# Orchestrates lacraia-core daemon, SQLite seeding, and health verification
# =============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PORT="${PORT:-8080}"
OPEN_BROWSER=0
SEED_DB=1

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
        -h|--help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  -p, --port <number>  Specify starting port for backend (default: 8080)"
            echo "  -o, --open           Open browser with live health dashboard"
            echo "  --seed               Seed demo tracks & similarity graph (default: on)"
            echo "  --no-seed            Do not auto-seed database"
            echo "  -h, --help           Display this help message"
            exit 0
            ;;
        [0-9]*)
            PORT="$1"
            shift
            ;;
        *)
            echo "Unknown argument: $1" >&2
            exit 1
            ;;
    esac
done

# Check and auto-increment port if occupied
while command -v lsof >/dev/null 2>&1 && lsof -i :"$PORT" >/dev/null 2>&1; do
    echo "Notice: Port $PORT is occupied, trying port $((PORT + 1))..."
    PORT=$((PORT + 1))
done

DB_PATH="${LACRAIA_DB_PATH:-$SCRIPT_DIR/lacraia.sqlite}"
CACHE_DIR="${LACRAIA_CACHE_DIR:-$SCRIPT_DIR/cache}"
mkdir -p "$CACHE_DIR"

# Seed SQLite database with demo tracks and similarity graph if requested
if [ "$SEED_DB" -eq 1 ] && command -v python3 >/dev/null 2>&1; then
    python3 - <<PY
import sqlite3, time

db_path = "$DB_PATH"
conn = sqlite3.connect(db_path)
cur = conn.cursor()

# Create tables if not existing
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
    created_at INTEGER NOT NULL
);
""")

# Check existing tracks
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

    # Populate initial lookahead queue
    cur.execute("SELECT COUNT(*) FROM queue;")
    if cur.fetchone()[0] == 0:
        for idx, trk_id in enumerate(["trk_1", "trk_2", "trk_3"]):
            cur.execute("INSERT INTO queue (track_id, position, state, created_at) VALUES (?, ?, 'pending', ?);",
                        (trk_id, idx, now))

    conn.commit()
    print("✓ Demo database seeded with 5 tracks, similarity edges, and initial lookahead queue.")
else:
    print(f"✓ SQLite database contains {count} track(s).")
conn.close()
PY
fi

BACKEND_PID=""

cleanup() {
    echo ""
    echo "Stopping demo services..."
    if [ -n "$BACKEND_PID" ] && kill -0 "$BACKEND_PID" 2>/dev/null; then
        kill "$BACKEND_PID" 2>/dev/null || true
        wait "$BACKEND_PID" 2>/dev/null || true
    fi
    echo "✓ All demo processes stopped."
}

trap cleanup INT TERM EXIT

echo "Starting La Craia Core backend on port $PORT..."
PORT="$PORT" LACRAIA_CACHE_DIR="$CACHE_DIR" LACRAIA_DB_PATH="$DB_PATH" cargo run -p lacraia-core &
BACKEND_PID=$!

# Wait for backend health probe
HEALTH_URL="http://localhost:$PORT/health"
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
    echo "Error: Backend failed to become healthy within 20 seconds." >&2
    exit 1
fi

cat << EOF

    ░█░░░█▀█░░░█▀▀░█▀▄░█▀█░▀█▀░█▀█
    ░█░░░█▀█░░░█░░░█▀▄░█▀█░░█░░█▀█
    ░▀▀▀░▀░▀░░░▀▀▀░▀░▀░▀░▀░▀▀▀░▀░▀

  La Craia Cockpit & Automotive Audio Demo
  Decentralized Acoustic Streaming Suite (v0.2.0)

★ Backend Service:  http://localhost:$PORT
★ Health Probe:     http://localhost:$PORT/health
★ Radio Engine:     http://localhost:$PORT/api/radio?artist=Chico+Buarque&track=Construção
★ WebSocket Hub:    ws://localhost:$PORT/ws
★ Audio Cache:      $CACHE_DIR
★ SQLite Database:  $DB_PATH

[Press Ctrl+C to terminate the demo environment]

EOF

if [ "$OPEN_BROWSER" -eq 1 ] && command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$HEALTH_URL" >/dev/null 2>&1 &
fi

wait "$BACKEND_PID"
