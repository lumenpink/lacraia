# GOST Language Specification: Rust
**Identificador da Stack:** `rust`  
**Padrão Base:** GOST `2.0.0`

---

## 1. Visão Geral e Filosofia
Esta especificação define os requisitos de engenharia, ferramentas e padrões para desenvolvimento em **Rust** (edição 2024 canônica, compilador `rustc >= 1.99.0`) no ecossistema GOST.

---

## 2. Ferramental Mandatório de Testes & Qualidade

| Requisito GOST | Ferramenta Padrão | Comando de Validação no CI |
|---|---|---|
| **Test Runner** | **Cargo Test** | `cargo test --all-targets --all-features` |
| **Cobertura (≥ 90%)** | **cargo-llvm-cov** / **tarpaulin** | `cargo llvm-cov --fail-under-lines 90` |
| **Testes de Mutação** | **cargo-mutants** | `cargo mutants --in-diff --no-shuffle` |
| **Linter (Zero Warnings)** | **Clippy** | `cargo clippy --all-targets --all-features -- -D warnings` |
| **Formatação Determinística** | **rustfmt** | `cargo fmt -- --check` |
| **Auditoria de Segurança** | **cargo-audit** | `cargo audit` |

---

## 3. Gestão de Dependências (Twelve-Factor)
1. **Lockfile Obrigatório:** O arquivo `Cargo.lock` **DEVE** estar commitado na raiz.
2. **Dependências Mínimas:** Evitar crates pesadas quando a biblioteca padrão for suficiente. Feature flags ativadas apenas sob necessidade estrita.
3. **Compilação Hermética:** Builds de produção usam `--locked` para garantir que nenhuma dependência transitória flutue:
   ```bash
   cargo build --release --locked
   ```

---

## 4. Arquitetura de Domínio & SOLID Pragmático (Regra GOST-27)
1. **Pureza de Domínio (Zero I/O no Core):** O diretório `src/domain/` é estritamente puro. É terminantemente proibido importar módulos de sistema ou I/O dentro do Core: `std::fs`, `std::net`, `std::process`, `std::env`, `tokio::fs`, `tokio::net`, `reqwest`, `rusqlite`, `sqlx` ou `clap`.
2. **Estrutura de Pastas Canônica:**
   ```text
   src/
   ├── domain/             # Modelos e regras de negócio puras (sem I/O)
   │   ├── mod.rs
   │   ├── entity.rs       # Invariantes e structs de dados
   │   ├── status.rs       # Enums nativos para máquinas de estado
   │   └── ports.rs        # Traits de I/O (Output Ports)
   ├── application/        # Use cases e orquestração
   │   ├── mod.rs
   │   └── use_cases.rs
   ├── adapters/           # Adaptadores de I/O
   │   ├── mod.rs
   │   ├── in_memory.rs    # Fake em memória com HashMap (testes herméticos)
   │   ├── sqlite.rs       # Persistência concreta
   │   └── cli.rs          # Parser de linha de comando (clap)
   └── main.rs             # Composition Root (bootstrap e injeção)
   ```
3. **In-Memory Fakes:** Testes unitários instanciam structs que implementam os traits de domínio armazenando dados em `HashMap` ou `RwLock<Vec<T>>` na memória, rodando em microssegundos sem dependência de SQLite ou sockets. Fakes e adaptadores reais compartilham os mesmos testes de contrato (LSP).
4. **Parse, Don't Validate:** Validação de entradas externas na borda (parsers `clap` ou DTOs `serde`). O Core opera com primitivos confiáveis ou structs transparentes.
5. **Estados com Enums Nativos:**
   ```rust
   pub enum TaskState {
       Pending,
       Running { started_at: u64 },
       Finished { exit_code: i32 },
       Failed { error: String },
   }
   ```

---

## 5. Padrões de Sistema e Containers OCI
1. **Conformidade XDG:**
   Resolução de diretórios respeita variáveis de ambiente com fallback para `$HOME`:
   ```rust
   pub fn config_dir(app: &str) -> std::path::PathBuf {
       std::env::var("XDG_CONFIG_HOME")
           .map(std::path::PathBuf::from)
           .unwrap_or_else(|_| {
               let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
               std::path::PathBuf::from(home).join(".config")
           })
           .join(app)
   }
   ```
2. **Dockerfile Canônico (Multi-stage + Scratch Estático musl com CA-Certs):**
   ```dockerfile
   FROM rust:1.99-alpine AS builder
   RUN apk add --no-cache musl-dev ca-certificates
   WORKDIR /build
   COPY Cargo.toml Cargo.lock ./
   COPY src ./src
   RUN cargo build --release --target x86_64-unknown-linux-musl --locked

   FROM scratch
   USER 1000:1000
   COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
   COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/app /usr/local/bin/app
   ENTRYPOINT ["/usr/local/bin/app"]
   ```
3. **Streams:** Saída pura de dados via `println!()`, logs e mensagens diagnósticas via `eprintln!()` ou crate `tracing`/`env_logger` configurada para `stderr`.

---

## 6. Otimização de Binário de Release & Linking Estático

### 1. Perfil de Release Canônico (`[profile.release]`)
Para assegurar a máxima redução de tamanho binário, eliminação de símbolos mortos e prevenção de overhead de desbobinamento de pilha (*stack unwinding*), projetos Rust no ecossistema GOST devem declarar o perfil canônico em seus manifests `Cargo.toml`:

```toml
[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

### 2. Linking Estático musl (`crt-static`)
Para garantir independência total do carregador dinâmico (`ld-linux`) e máxima portabilidade em ambientes Linux glibc e musl, a compilação contra alvos `x86_64-unknown-linux-musl` e `aarch64-unknown-linux-musl` deve habilitar o flag `crt-static`:

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-musl]
rustflags = ["-C", "target-feature=+crt-static"]

[target.aarch64-unknown-linux-musl]
rustflags = ["-C", "target-feature=+crt-static"]
```

