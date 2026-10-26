# ==============================================================================
# 🦂 La Craia Core — Dockerfile Multi-Stage Atômico
# Conformidade com GOST (Seção 13.6) e Perfil Kollontai (Seção 12.3)
# ==============================================================================

# ─────────────────────────────────────────────────────────────────────────────
# 1. Estágio de Compilação (Hermético via gost-ci-rust)
# ─────────────────────────────────────────────────────────────────────────────
FROM gambi.2lp.in/lumen/gost-ci-rust:latest AS builder

WORKDIR /build

# Copiar arquivos de dependências e código do workspace
COPY Cargo.toml Cargo.lock* ./
COPY packages/lacraia-shared ./packages/lacraia-shared
COPY packages/lacraia-core ./packages/lacraia-core
COPY packages/lacraia-app ./packages/lacraia-app

# Compilar binário lacraia-core em modo release
RUN cargo build --release -p lacraia-core

# ─────────────────────────────────────────────────────────────────────────────
# 2. Estágio Final de Execução (Minimal, Non-root, Zero Inchaço)
# ─────────────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    ffmpeg \
    tini \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

# Instalar binário autônomo do yt-dlp (fallback Tier 3 de resolução de mídia)
RUN curl -fsSL https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp \
    || echo "Aviso: yt-dlp standalone ignorado; rustypipe é o resolvedor primário"

# Criar usuário não-privilegiado (execução obrigatória sem root - GOST 13.6.3)
RUN groupadd -g 1000 lacraia && \
    useradd -u 1000 -g lacraia -m -s /bin/sh lacraia

WORKDIR /app

# Criar diretórios de persistência de banco de dados SQLite, cache de áudio e site estático
RUN mkdir -p /app/data /app/cache /app/site && \
    chown -R lacraia:lacraia /app

# Copiar binário compilado e assets do site/console
COPY --from=builder /build/target/release/lacraia-core /usr/local/bin/lacraia-core
COPY --chown=lacraia:lacraia site /app/site

USER 1000:1000

ENV PORT=2011 \
    HOST=0.0.0.0 \
    LACRAIA_DB_PATH=/app/data/lacraia.sqlite \
    LACRAIA_CACHE_DIR=/app/cache \
    LACRAIA_SITE_DIR=/app/site \
    YTDLP_BIN=/usr/local/bin/yt-dlp

EXPOSE 2011

ENTRYPOINT ["/usr/bin/tini", "--", "lacraia-core"]
