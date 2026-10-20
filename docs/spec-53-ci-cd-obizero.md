# Especificação Normativa — Ticket #53
## Continuous Deployment Soberano no Nó Central (obizero) e Descomissionamento do Watchtower

**Norma de Referência:** GOST v1.5.0 (Perfil Lumen — Seção 9)  
**Status:** Aprovado para Implementação  
**Data:** 2026-10-20  

---

### 1. Escopo e Objetivos

Esta especificação define o pipeline de entrega contínua (Continuous Deployment - CD) da aplicação containerizada **La Craia (`lacraia`)** no nó central de execução (`obizero`), substituindo em definitivo o mecanismo legado baseado no Watchtower:

1. **Construção Local Soberana:** A imagem Docker `lumenpink/lacraia:latest` é gerada diretamente no Docker daemon do nó `obizero` através do socket montado no runner do Forgejo.
2. **Descomissionamento Compulsório do Watchtower:**
   - Remoção de qualquer gatilho ou webhook HTTP do Watchtower no `.forgejo/workflows/ci.yml`.
   - Veto absoluto ao rótulo `com.centurylinklabs.watchtower.enable`.
3. **Deploy Local Imediato Sem Download Redundante:**
   - Acionamento imediato da nova imagem no Docker Compose runtime:
     ```bash
     docker compose -f /home/lumen/stacks/lacraia/compose.yaml up -d --no-build lacraia
     ```
4. **Sondagem de Healthcheck Ativo & Rollback Automático:**
   - Verificação em loop de até 15 segundos confirmando que o container `lacraia` permanece em estado `running`/`healthy`.
   - Em caso de falha de inicialização, aciona imediatamente o rollback para a imagem anterior salva e aborta o job com erro.
5. **Purga Automática de Camadas Órfãs:**
   - Execução de `docker image prune -f` após o deploy bem-sucedido para evitar acúmulo de disco.

---

### 2. Etapas do Job no Workflow (`.forgejo/workflows/ci.yml`)

1. Build e tag local da imagem `lumenpink/lacraia:latest`.
2. Backup do ID da imagem anterior (para rollback).
3. Push opcional para o Docker Hub (caso `DOCKER_PASSWORD` esteja configurado).
4. Aplicação do deploy local:
   ```bash
   docker compose -f /home/lumen/stacks/lacraia/compose.yaml up -d --no-build lacraia
   ```
5. Validação de inicialização:
   ```bash
   for i in $(seq 1 15); do
     STATUS=$(docker inspect --format='{{.State.Status}}' lacraia 2>/dev/null || echo "unknown")
     if [ "$STATUS" = "running" ]; then
       echo "✔ Container lacraia saudável e em execução."
       exit 0
     fi
     sleep 1
   done
   echo "✘ Falha na inicialização do container lacraia. Executando rollback..."
   # rollback
   ```
6. Limpeza: `docker image prune -f`.
