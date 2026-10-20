# Especificação Normativa — Ticket #52
## Integração de Distribuição na Banca e Auto-Atualização OpenDesktop XDG

**Norma de Referência:** GOST v1.5.0 (Perfil Lumen — Seções 7 e 8)  
**Status:** Aprovado para Implementação  
**Data:** 2026-10-20  

---

### 1. Escopo e Objetivos

Esta especificação define a arquitetura técnica, interfaces de linha de comando, modelo de configuração e esteira de distribuição soberana para o **La Craia (`lacraia`)**, em estrita conformidade com as diretivas do Perfil Lumen da norma GOST:

1. **Protocolo OpenDesktop (XDG Base Directory):**
   - Configuração de usuário persistida em `$XDG_CONFIG_HOME/lacraia/config.toml` (fallback canônico: `~/.config/lacraia/config.toml`).
   - Estado de verificação de atualização e cache em `$XDG_STATE_HOME/lacraia/update_check.json` (fallback canônico: `~/.local/state/lacraia/update_check.json`).
2. **Modelo de Configuração (`config.toml`):**
   - `channel`: Canal padrão ativo (`"stable"` ou `"nightly"`).
   - `pinned`: Booleano que impede qualquer atualização se verdadeiro.
   - `pin`: String de versão fixada (quando travada).
   - `auto_check`: Booleano que ativa ou desativa sondagem silenciosa de versão no startup.
   - `check_interval_hours`: Inteiro definindo a frequência mínima em horas entre consultas remotas de rede (padrão: 24 horas).
   - `auto_update`: Booleano (padrão: `false`). Se habilitado e sem versão pinada, o binário efetua a substituição atômica transparente em segundo plano para o próximo arranque.
3. **Comportamento em Linha de Comando e Alertas em `stderr`:**
   - Emissões de avisos de atualização periódica ou notificação de substituição automática feitas estritamente via `stderr` para não poluir fluxos de automação e pipes (`stdout`).
   - Bloqueio sumário de upgrades em caso de pinning ativo (`pinned = true` ou `pin != ""`), exceto mediante fornecimento da flag de salvaguarda explícita `--force`.
4. **Subcomandos Canônicos de CLI:**
   - `lacraia update [--channel <c>] [--pin <v>] [--force]`
   - `lacraia check-update [--channel <c>]`
   - `lacraia config show`
   - `lacraia config get <key>`
   - `lacraia config pin [<version>]`
   - `lacraia config unpin`
   - `lacraia config set-channel <stable|nightly>`
   - `lacraia config set-auto-check <true|false>`
   - `lacraia config set-auto-update <true|false>`
5. **Distribuição Soberana na Banca (`banca.2lp.in`):**
   - Binários compilados no runner soberano e publicados via Cloudflare R2 sob o prefixo `https://banca.2lp.in/downloads/lacraia/`.
   - Política de retenção: 5 versões estáveis mais recentes (`stable-latest`) e 10 nightlies mais recentes (`nightly-latest`).
   - Manifesto atômico `index.json`, além dos arquivos pontuais `version` e `nightly-version`.
   - Veto absoluto a binários pesados nas Releases do Codeberg.

---

### 2. Formato dos Arquivos

#### 2.1 `$XDG_CONFIG_HOME/lacraia/config.toml`

```toml
[update]
channel = "stable"
pinned = false
pin = ""
auto_check = true
check_interval_hours = 24
auto_update = false
```

#### 2.2 `$XDG_STATE_HOME/lacraia/update_check.json`

```json
{
  "last_check_epoch": 1729410000,
  "latest_version": "0.2.0",
  "channel": "stable"
}
```

---

### 3. Precedência de Resolução

1. **Flag CLI:** `--channel <stable|nightly>`
2. **Variável de Ambiente:** `LACRAIA_CHANNEL`
3. **Arquivo de Configuração:** Campo `channel` em `config.toml`
4. **Fallback Padrão:** `"stable"`

---

### 4. Salvaguardas de Pinning

Se `pinned == true` ou se `pin` não for vazio:
- `lacraia update` é bloqueado com erro informativo, a menos que `--force` seja fornecido.
- A sondagem automática de versão no arranque silencia alertas e descarta auto-update em background.
- `lacraia config unpin` restaura a flexibilidade removendo o pin e configurando `pinned = false`.
