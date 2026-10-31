# GOST Language Specification: POSIX Shell
**Identificador da Stack:** `shell`  
**Padrão Base:** GOST `2.0.0`

---

## 1. Visão Geral e Filosofia
Esta especificação define os requisitos de engenharia, ferramentas e padrões para desenvolvimento de scripts e utilitários de sistema em **POSIX Shell** e **Bash** no ecossistema GOST.

O padrão universal e prioritário é **POSIX Shell estrito (`/bin/sh`)** visando máxima portabilidade e execução determinística em contêineres e distribuições minimalistas (Alpine, Debian, BusyBox). Quando recursos avançados de interpretador forem indispensáveis (como arrays associativos ou manipulação robusta de descritores), adota-se **Bash 5.2+** como baseline canônico do estado da arte, com shebang explícita `#!/usr/bin/env bash`.

---

## 2. Ferramental Mandatório de Testes & Qualidade

| Requisito GOST | Ferramenta Padrão | Comando de Validação no CI |
|---|---|---|
| **Test Runner** | **ShellSpec** (BDD Framework) | `shellspec` |
| **Cobertura (≥ 90%)** | **kcov** integrado | `shellspec --kcov` |
| **Linter (Zero Warnings)** | **ShellCheck** | `shellcheck -s sh *.sh lib/*.sh` (ou `-s bash`) |
| **Formatação Determinística** | **shfmt** | `shfmt -d -i 2 -ci -bn .` |

---

## 3. Diretrizes de Codificação & Robustez
1. **Portabilidade POSIX:** Scripts de utilidade geral devem priorizar compatibilidade com `/bin/sh` estrito, evitando *bashismos* desnecessários a menos que explicitamente exigido e indicado na shebang (`#!/usr/bin/env bash`).
2. **Defensiva Estrita:**
   Todo script executável **DEVE** declarar tratamento defensivo de erros:
   ```sh
   set -eu
   # Em bash:
   set -euo pipefail
   ```
3. **Citações Obrigatórias:** Todas as expansões de variáveis devem ser envolvidas em aspas duplas (`"$VAR"`), prevenindo *word splitting* e *globbing* acidental.
4. **Funções Puras e Modulares:** Lógica reutilizável isolada em funções com variáveis locais (`local var="val"` em bash, ou convenções de namespace em sh).

---

## 4. Pureza Arquitetural & Core em `lib/core.sh` (Regra GOST-27)
1. **Core de Funções Puras:** A lógica central de cálculo, manipulação de texto e regras de negócio deve residir em `lib/core.sh`.
2. **Isolamento de I/O:** Em conformidade com a Regra `GOST-27`, funções contidas em `lib/core.sh` não devem invocar comandos de rede, versionamento ou bancos externos (`curl`, `wget`, `git`, `sqlite3`, `ssh`). Operações de I/O são realizadas na borda (`bin/` ou adaptadores de entrada) e passadas como argumentos ou via stdin para as funções do core.

---

## 5. Integração com Sistema e XDG
1. **Resolução de Diretórios XDG com Fallbacks:**
   ```sh
   APP_NAME="meu-util"
   CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/$APP_NAME"
   DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/$APP_NAME"
   STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/$APP_NAME"
   CACHE_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/$APP_NAME"
   RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$APP_NAME"
   ```
2. **Sanitização de Loaders de Configuração:**
   Para prevenir execução acidental de código arbitrário ao carregar arquivos `KEY=VAL`:
   ```sh
   if [ -f "$CONFIG_FILE" ]; then
       while IFS='=' read -r key val || [ -n "$key" ]; do
           # Ignora comentários e linhas vazias
           case "$key" in
               \#*|"") continue ;;
           esac
           # Valida formato estrito de identificador
           if echo "$key" | grep -Eq '^[A-Za-z_][A-Za-z0-9_]*$'; then
               # Remove aspas envolventes se presentes
               clean_val="$(echo "$val" | sed -e 's/^"//' -e 's/"$//' -e "s/^'//" -e "s/'$//")"
               export "$key=$clean_val"
           fi
       done < "$CONFIG_FILE"
   fi
   ```

---

## 6. Streams Unix, NO_COLOR & Exit Codes
1. **Segregação de Saída:**
   ```sh
   # Saída de dados (pipeable):
   printf '%s\n' "$data"
   
   # Logs e mensagens para o usuário (stderr):
   printf '[INFO] Processando %s...\n' "$item" >&2
   ```
2. **Respeito a NO_COLOR:**
   ```sh
   if [ -z "${NO_COLOR:-}" ] && [ -t 1 ]; then
       RED='\033[0;31m'
       NC='\033[0m'
   else
       RED=''
       NC=''
   fi
   ```
3. **Exit Codes Semânticos:**
   - `0`: Sucesso
   - `1`: Erro geral de execução
   - `2`: Erro de sintaxe nos argumentos
   - `130`: Interrompido por SIGINT (Ctrl+C)
