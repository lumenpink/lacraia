# Especificação Técnica — Ticket #77: Padronização do Binário para `lacraia` e Emissão Canônica de `--version`

- **Ticket:** [#77](https://gambi.2lp.in/lumen/lacraia/issues/77)
- **Branch:** `feat/77-renomear-binario-para-lacraia-e-expor-versao-canonica-via-version`
- **Norma:** GOST 1.5.0 (DD-TDD)
- **Status:** Fase 3 (Especificação)
- **Responsável:** Lumen Pink & Antigravity

---

## 1. Contexto e Motivação

O projeto La Craia gera artefatos executáveis para Linux (x86_64 e aarch64), Windows e Android. Atualmente, o binário principal compilado pelo pacote `packages/lacraia-core` utiliza a nomenclatura `lacraia-core`. Essa discrepância gera os seguintes atritos operacionais:
1. **Divergência com a Banca e ecossistema:** Os utilitários do ecossistema seguem a convenção canônica de executáveis com o nome exato da aplicação (`lauda`, `mocambo`, `papeli`, `banca`). O workflow de CI inclusive já publicava o binário na Banca sob o identificador `lacraia`, criando uma assimetria entre o nome interno de build (`target/.../lacraia-core`) e o nome distribuído.
2. **Conformidade de CLI (`--version` / `-V`):** A execução da flag `--version` ou `-V` deve retornar canonicamente o nome e a versão semântica do sistema (`lacraia x.y.z`), sem ruídos de log e com código de saída 0 (*exit status 0*).
3. **Desalinhamento em scripts de suporte:** `Dockerfile`, `dev.sh`, `.forgejo/workflows/ci.yml` e a documentação ainda invocavam referências legadas a `lacraia-core`.

---

## 2. Decisão Arquitetural e Justificativas Técnicas (para a Natasha)

1. **Renomeação do `[[bin]]` em `packages/lacraia-core`:**
   - Em `packages/lacraia-core/Cargo.toml`, altera-se `name = "lacraia-core"` para `name = "lacraia"`.
   - O nome do pacote crate permanece `lacraia-core` para manter a estrutura do workspace modular (`lacraia-shared`, `lacraia-core`, `lacraia-app`).
   - O binário gerado na raiz de `target/{target}/release/` passa a ser `lacraia` (ou `lacraia.exe` no Windows).
   - *Alternativa descartada:* Manter binários duplicados `lacraia` e `lacraia-core` simultaneamente no `Cargo.toml`. Essa alternativa polui o cache de compilação, duplica o tempo de linkagem LTO e introduz ambiguidades na esteira de CI.

2. **Saída Canônica de Versão:**
   - Em `packages/lacraia-core/src/cli.rs`, as flags `-V`, `--version` e o subcomando `version` devem emitir estritamente `lacraia {CURRENT_VERSION}` via `println!`, finalizando imediatamente a execução com sucesso (`Ok(true)` -> exit 0), sem disparar daemons de background ou inicialização do Axum.

3. **Alinhamento da Esteira de CI e Contêineres:**
   - Em `.forgejo/workflows/ci.yml`:
     - Compilação via `cargo zigbuild --release ... -p lacraia-core` gerando artefato `target/.../release/lacraia`.
     - Empacotamento de tarballs e zips consumindo diretamente `lacraia` (e `lacraia.exe`).
   - Em `Dockerfile`:
     - Cópia e ponto de entrada apontando para `/usr/local/bin/lacraia` e `ENTRYPOINT ["/usr/bin/tini", "--", "lacraia"]`.
   - Em `packages/lacraia-core/src/cli.rs`:
     - As rotas de download no `cmd_check_update` e `get_download_url_for_channel` apontam para o artefato `lacraia`.

---

## 3. Matriz de Rastreabilidade DD-TDD

| Requisito | Arquivo Afetado | Teste Automatizado |
| :--- | :--- | :--- |
| Definição do Binário `lacraia` | `packages/lacraia-core/Cargo.toml` | `packages/lacraia-core/tests/cli_test.rs` |
| Exclusão do Binário `lacraia-core` | `packages/lacraia-core/Cargo.toml` | `packages/lacraia-core/tests/cli_test.rs` |
| Emissão de Versão `lacraia x.y.z` | `packages/lacraia-core/src/cli.rs` | `packages/lacraia-core/tests/cli_test.rs` |
| CI e Scripts Saneados | `ci.yml`, `Dockerfile`, `dev.sh` | `packages/lacraia-core/tests/cli_test.rs` |
