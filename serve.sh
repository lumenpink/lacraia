#!/bin/sh
# =============================================================================
# La Craia Static Site Preview Server
# Serves the site/ directory locally without publishing
# =============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [ -d "$SCRIPT_DIR/site" ]; then
    SITE_DIR="$SCRIPT_DIR/site"
elif [ -f "$SCRIPT_DIR/index.html" ]; then
    SITE_DIR="$SCRIPT_DIR"
else
    echo "Error: Could not locate site/ directory." >&2
    exit 1
fi

PORT="${PORT:-8080}"
OPEN_BROWSER=0

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
        -h|--help)
            echo "Usage: $0 [options] [port]"
            echo ""
            echo "Options:"
            echo "  -p, --port <number>  Specify port number (default: 8080)"
            echo "  -o, --open           Open default web browser automatically"
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

URL="http://localhost:$PORT"

cat << EOF

    ░█░░░█▀█░░░█▀▀░█▀▄░█▀█░▀█▀░█▀█
    ░█░░░█▀█░░░█░░░█▀▄░█▀█░░█░░█▀█
    ░▀▀▀░▀░▀░░░▀▀▀░▀░▀░▀░▀░▀▀▀░▀░▀

  La Craia Local Documentation Preview Server
  Autonomous Automotive Audio & HUD Suite

★ Local URL:  $URL
★ Serving:    $SITE_DIR

[Press Ctrl+C to terminate the server]

EOF

if [ "$OPEN_BROWSER" -eq 1 ] && command -v xdg-open >/dev/null 2>&1; then
    (sleep 1 && xdg-open "$URL" >/dev/null 2>&1) &
fi

if command -v python3 >/dev/null 2>&1; then
    exec python3 -m http.server "$PORT" --directory "$SITE_DIR"
elif command -v python >/dev/null 2>&1; then
    (cd "$SITE_DIR" && exec python -m SimpleHTTPServer "$PORT")
elif command -v php >/dev/null 2>&1; then
    exec php -S "localhost:$PORT" -t "$SITE_DIR"
elif command -v busybox >/dev/null 2>&1; then
    exec busybox httpd -f -p "$PORT" -h "$SITE_DIR"
elif command -v npx >/dev/null 2>&1; then
    exec npx -y serve "$SITE_DIR" -l "$PORT"
else
    echo "Error: No suitable HTTP server found (python3, php, busybox, or npx required)." >&2
    exit 1
fi
