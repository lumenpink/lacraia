# Roadmap v0.2.0 – La Craia

## Próximos tickets (sugestões)

1. **Integração Bluetooth AVRCP**
   - Permitir controle físico de reprodução (play/pause/next/previous) via volante.
   - Expor APIs Android `BluetoothLe` + `MediaSession` para mapear ações.

2. **Pré‑cache de estações na garagem**
   - Detectar quando o carro está conectado a Wi‑Fi da garagem.
   - Baixar e armazenar localmente os arquivos de áudio de estações favoritas para playback offline.

3. **Visualizador de ondas sonoras no Web HUD**
   - Widget em React/Dioxus mostrando espectro em tempo real.
   - Tema “cockpit noturno” com cores de alto contraste.

4. **Suporte a “skip silence”**
   - Detectar trechos silenciosos nas faixas e avançar automaticamente.
   - Configurável via UI no HUD.

5. **Melhorias de performance LCP**
   - Carregar assets críticos via `content‑visibility` e `fetch‑priority`.
   - Medir com Web Vitals e otimizar.

6. **Modo “Eco” de economia de energia**
   - Reduzir volume e desativar atualização de letras quando bateria do dispositivo está baixa.

7. **Integração com serviços de voz (Google Assistant / Alexa)**
   - Permitir comandos de voz para trocar de estação, pausar, etc.

8. **Teste automatizado de HUD UI**
   - Testes de instrumentação Android usando Espresso para validar a sobreposição flutuante.

9. **Documentação de API pública**
   - Gerar OpenAPI spec para endpoints de streaming e controle.

10. **CI/CD pipeline aprimorada**
    - GitHub Actions que rodem testes, lint e deploy automático para Cloudflare Workers.

---

Esses itens podem ser quebrados em tickets individuais (`#11`, `#12`, …) seguindo o fluxo CCCP de commits e documentação. 
