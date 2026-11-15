# Especificação Técnica - Ticket #70: Restringir Adição à Fila Exclusivamente a Faixas do MusicBrainz

## 1. Contexto e Problema
Anteriormente, o sistema permitia a inclusão na fila de reprodução de entradas de texto livre ad-hoc (ex.: IDs sem padronização como `adhoc:123`, strings arbitrárias digitadas sem metadados), o que gerava inconsistências severas no visor do paredão, na indexação contínua de similaridade sonora e no histórico de reprodução.
Para garantir consistência e enriquecimento confiável de metadados, toda adição à fila de reprodução via API pública (`POST /api/queue`) ou interface Cockpit deve ser estritamente vinculada a uma gravação validada do MusicBrainz (identificada por `mb:<id>`, `musicbrainz:<id>` ou UUID do MusicBrainz).

## 2. Árvore de Decisões (Contexto para Natasha)
Para que Natasha entenda por que esta regra foi estabelecida:
1. **Alternativa Descartada 1 - Aceitar qualquer texto e tentar adivinhar em background:**
   - *Por que foi descartada:* Resolução assíncrona tardia leva a descompassos entre o que o passageiro pediu e o que o som toca, além de aceitar lixo ou termos inválidos na fila persistida.
2. **Alternativa Descartada 2 - Validar apenas no frontend em JavaScript:**
   - *Por que foi descartada:* Deixaria a API HTTP e clientes remotos desprotegidos contra payloads mal formatados ou injeções de texto arbitrário.
3. **Decisão Adotada - Validação no Core e no Cockpit:**
   - No Core (`lacraia-core`): Criada a função de validação canônica `is_musicbrainz_validated(track_id: &str) -> bool`.
     - Reconhece identificadores com prefixo `mb:` ou `musicbrainz:`, ou UUIDs canônicos do MusicBrainz.
     - No handler HTTP `handle_queue_append` (`POST /api/queue`), payloads que não possuam identificador válido do MusicBrainz são rejeitados com `HTTP 400 Bad Request` e a mensagem de erro padronizada:
       `MUSICBRAINZ_REQUIRED: Apenas faixas validadas pelo MusicBrainz podem ser adicionadas à fila`.
   - No Cockpit (`site/app/index.html`):
     - Remoção da geração automática de faixas com id `adhoc:`.
     - Caso o usuário tente enfileirar uma faixa não validada pelo catálogo do MusicBrainz, o Cockpit bloqueia a operação e emite um alerta explícito: "⛔ Apenas faixas validadas pelo MusicBrainz podem ser adicionadas à fila!".

## 3. Critérios de Aceite e Contrato
1. `POST /api/queue` com `track_id` não validado pelo MusicBrainz (ex.: texto livre sem prefixo `mb:` ou UUID) deve retornar `400 Bad Request` com código `MUSICBRAINZ_REQUIRED`.
2. `POST /api/queue` com `track_id` contendo prefixo `mb:` (ex.: `mb:5b719463-e578-43bb-a320-c75c8d002f23`) ou UUID válido do MusicBrainz deve ser aceito e enfileirado com sucesso.
3. No Cockpit, a busca não deve mais gerar entradas artificiais `adhoc:` e deve orientar o usuário a pesquisar faixas catalogadas.
4. Testes automatizados de unidade e integração devem validar a rejeição de texto livre e a aceitação de itens do MusicBrainz.
