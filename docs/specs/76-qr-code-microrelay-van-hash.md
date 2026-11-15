# Especificação Técnica - Ticket #76: Apontar QR Code do Modo Van para Microrelay com Hash Identificador

## 1. Contexto e Problema
No ambiente Android (APK) e em instalações autônomas locais, a origem do navegador (`window.location.origin`) resolve para `file://` ou `http://localhost:2011`.
Anteriormente, o QR Code de embarque de passageiros montava a URL de compartilhamento usando `baseUrl = window.location.origin`, gerando links inacessíveis quando escaneados pela câmera dos celulares dos passageiros em redes externas ou móveis (4G/5G).
Para que o pareamento ocorra de forma transparente, o QR Code gerado no Modo Van precisa codificar a URL pública do microrelay configurado com o parâmetro de identificação unívoca da van (`?van=<van_hash>`) e o relay WebSocket correspondente (`&relay=<relay_url>`).

## 2. Árvore de Decisões (Contexto para Natasha)
Para que Natasha compreenda a arquitetura adotada e as alternativas descartadas:
1. **Alternativa Descartada 1 - Exigir que o passageiro esteja na mesma sub-rede Wi-Fi e usar o IP local (LAN):**
   - *Por que foi descartada:* Em eventos na rua, manifestações ou viagens em vans, os aparelhos móveis dos passageiros estão utilizando planos de dados móveis 4G/5G com operadoras distintas atrás de CGNAT. Exigir roteador Wi-Fi dedicado cria atrito operacional inadmissível para o público popular.
2. **Alternativa Descartada 2 - Forçar domínio hardcoded estático no APK:**
   - *Por que foi descartada:* Impede que organizadores com servidores próprios de homelab ou relays comunitários customizados utilizem suas próprias URLs de microrelay.
3. **Decisão Adotada - Resolução Dinâmica com Fallback Resiliente:**
   - Implementa a função `resolvePublicPassengerBaseUrl()`:
     - Prioriza o endereço HTTP/HTTPS derivado da URL do microrelay ativa (`localStorage.getItem("lacraia_relay_url")` ou input `#input-relay-url`).
     - Se o app estiver rodando em ambiente nativo/local (`isNativeApp`, `file:`, `localhost`, `10.0.2.2`), redireciona a base para a infraestrutura pública do microrelay (`https://lacraia-relay.2lp.in`).
     - Preserva a identificação da sala através do `state.vanHash` (ou fallback seguro `state.cabinId`), adicionando os parâmetros `?van=<hash>&relay=<relay_url>`.
   - Clientes que escanearem o QR Code ou clicarem no link copiado recebem automaticamente o parâmetro `relay`, conectando-se como `client` na sala correspondente do microrelay.

## 3. Critérios de Aceite e Contrato
1. O QR Code e o elemento `#qr-link-text` no Modo Van não devem apontar para `file://` ou `localhost` em ambientes nativos do APK.
2. A URL gerada deve conter o parâmetro `van=` codificado com o `vanHash` unívoco da sessão/cabine.
3. A URL gerada deve conter ou apontar para a rota do microrelay WebSocket configurado.
4. O cliente web de passageiro que receber o link com `?van=<hash>&relay=<relay>` deve conectar o WebSocket diretamente ao relay na sala informada.
5. Testes automatizados no Rust (`packages/lacraia-app` e/ou `packages/lacraia-core`) devem validar essa formatação e contrato.
