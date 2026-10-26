# Especificação Técnica: Deploy Soberano no Obizero via SSH

**Ticket Associado:** #59  
**Branch:** `feat/59-cd-obizero-ssh`  
**Norma:** GOST 1.5.0

## 1. Objetivo
Executar o Continuous Deployment soberano da Lacraia no nó `obizero` através de sessão SSH remota a partir do runner, eliminando qualquer dependência de bind-mount local do diretório de stacks (`/home/lumen/stacks/lacraia`) dentro do container do runner.

## 2. Requisitos Técnicos
1. No arquivo `.forgejo/workflows/ci.yml`, substituir os passos diretos de deploy e healthcheck locais por um passo único via SSH (`Sovereign CD on obizero via SSH`).
2. Utilizar as opções padrão de SSH com fallback para `172.17.0.1` (gateway Docker do host) e usuário `lumen`:
   - Chave: `~/.ssh/id_obitwo` ou `~/.ssh/id_ed25519` (injetada de forma hermética pelo runner) ou segredo `OBIZERO_SSH_KEY`.
   - Host: `${OBIZERO_SSH_HOST:-172.17.0.1}`.
   - Usuário: `${OBIZERO_SSH_USER:-lumen}`.
3. Executar remotamente no host:
   - Navegação até `~/stacks/lacraia`.
   - Captura do `OLD_IMAGE_ID` para protocolo de rollback.
   - Aplicação da atualização via `docker compose -f compose.yaml up -d --no-build lacraia`.
   - Sondagem ativa de estabilidade (`docker inspect --format='{{.State.Status}}' lacraia`) durante 15s.
   - Rollback automático para `OLD_IMAGE_ID` caso o container não estabilize.
   - Purga de camadas órfãs (`docker image prune -f`).
4. Atualizar o script de auditoria `tests/audit_cd_workflow.sh` para validar a presença da invocação SSH e o ciclo completo de CD soberano.

## 3. Critérios de Aceitação
- Execução verde do script `tests/audit_cd_workflow.sh`.
- Testes e auditorias do repositório (`cargo test`) aprovados.
