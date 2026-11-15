# Especificação Técnica - Ticket #73: Exibir Campos do Modo Van Apenas Quando Ativado

## 1. Contexto e Problema
O Cockpit opera primordialmente como um console individual para reprodução contínua no paredão de som. O Modo Van é um recurso colaborativo secundário para compartilhamento em grupo, permitindo a passageiros ingressarem em cabines e requisitarem faixas via QR Code ou microrelay WebSocket 5G.
Anteriormente, campos administrativos específicos do Modo Van (chave mestra da instalação e URL do microrelay 5G) permaneciam sempre visíveis na janela de configurações, poluindo a tela e expondo informações de administração e rede sem necessidade quando o usuário opera o som de forma individual.

## 2. Árvore de Decisões (Contexto para Natasha)
Para que Natasha entenda por que esta estrutura foi adotada:
1. **Alternativa Descartada 1 - Criar uma modal de configurações separada apenas para a Van:**
   - *Por que foi descartada:* Criaria proliferação desnecessária de modais no Cockpit e duplicaria botões de ação e listeners.
2. **Alternativa Descartada 2 - Desabilitar (`disabled`) os campos de van em vez de ocultar:**
   - *Por que foi descartada:* Manter os inputs na tela desabilitados continua ocupando espaço vertical precioso em telas móveis e mantém o ruído visual para o operador monousuário.
3. **Decisão Adotada - Agrupamento em container condicional `#van-mode-settings-box`:**
   - Agrupa os campos específicos do Modo Van (chave mestra de administração `#input-master-key` e endereço do retransmissor móvel `#input-relay-url`) dentro do container `<div id="van-mode-settings-box">`.
   - Por padrão no HTML, o container inicia com `display: none;`.
   - A função `applyVanMode(enabled)` em `site/app/index.html` passa a gerenciar a visibilidade do container:
     - Se `enabled == true`: `vanSettingsBox.style.display = "flex"`.
     - Se `enabled == false`: `vanSettingsBox.style.display = "none"`.
   - O evento `change` do `#check-van-mode` aciona `applyVanMode`, garantindo reatividade imediata no DOM.

## 3. Critérios de Aceite e Contrato
1. O elemento `#van-mode-settings-box` deve agrupar `#input-master-key` e `#input-relay-url` sob a seção do Modo Van.
2. O estado padrão do container no HTML deve ser oculto (`display: none;`).
3. Ao ativar o checkbox `#check-van-mode`, a função `applyVanMode` deve tornar `#van-mode-settings-box` visível (`display: flex`).
4. Ao desativar `#check-van-mode`, `#van-mode-settings-box` deve ser recolhido para `display: none`.
5. Testes automatizados devem validar estaticamente e funcionalmente a presença do container e a lógica de exibição condicional.
