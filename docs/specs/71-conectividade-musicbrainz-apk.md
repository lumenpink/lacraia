# Especificação Técnica - Ticket #71: Restaurar Conectividade com API do MusicBrainz no APK Android

## 1. Contexto e Problema
A versão Android (APK) apresentava falha de conectividade ao realizar consultas à API do MusicBrainz (`https://musicbrainz.org/ws/2/recording`), impedindo o catálogo e a busca de faixas online diretamente pelo aplicativo.
A análise técnica identificou duas causas convergentes:
1. **Formato Incompleto do `User-Agent`:** O MusicBrainz impõe uma política estrita de cabeçalho `User-Agent` (`App/Version ( contact-url-or-email )`). No `MainActivity.kt`, a WebView configurava `userAgentString = "${userAgentString} LacraiaApp/0.2.0"` sem o sufixo de contato entre parênteses, levando à rejeição de tráfego (HTTP 403 / 503 com `rate-limiter: lua`).
2. **Ausência de `Network Security Config` Explícito no Android:** O aplicativo Android não possuía o arquivo `network_security_config.xml` configurando a confiança de certificados TLS do sistema para o domínio `musicbrainz.org` e permitindo tráfego em rede local para homelabs/emulador.
3. **Tratamento de Falhas e Logs Estruturados no Cliente:** Ausência de mensagens informativas claras e log estruturado quando a API externa atinge limite de requisições ou sofre timeout.

## 2. Árvore de Decisões (Contexto para Natasha)
Para que Natasha compreenda por que esta solução foi adotada:
1. **Alternativa Descartada 1 - Realizar proxy reverso forçado de todas as buscas pelo core local:**
   - *Por que foi descartada:* Em instalações onde o app Android opera no modo cliente ou PWA leve sem o daemon completo em segundo plano, a busca do MusicBrainz deve funcionar diretamente pela WebView.
2. **Alternativa Descartada 2 - Utilizar chave de API proprietária:**
   - *Por que foi descartada:* O MusicBrainz é uma base aberta de dados comunitários que não requer chaves pagas, apenas identificação canônica transparente via User-Agent.
3. **Decisão Adotada - Padronização Canônica de User-Agent e Configuração de Rede:**
   - No `MainActivity.kt`: Atualizar o `userAgentString` para incluir a URL canônica de contato em conformidade estrita com o padrão do MusicBrainz:
     `"${settings.userAgentString} LacraiaApp/0.2.0 ( https://lacraia.2lp.in )"`.
   - No `lacraia-core` (`MetaBrainzProvider`): Atualizar a versão para `0.2.0` (`"Lacraia/0.2.0 ( https://lacraia.2lp.in )"`).
   - No `res/xml/network_security_config.xml`: Declarar configuração de segurança com suporte a `musicbrainz.org` e tráfego local (`10.0.2.2`, `localhost`, `2lp.in`).
   - No `AndroidManifest.xml`: Declarar `android:networkSecurityConfig="@xml/network_security_config"`.
   - No `site/app/index.html`: Incluir cabeçalho `Accept: application/json` e log estruturado em caso de indisponibilidade ou rate limit da API do MusicBrainz.

## 3. Critérios de Aceite e Contrato
1. `MainActivity.kt` deve configurar `userAgentString` contendo o sufixo `( https://lacraia.2lp.in )`.
2. O manifesto Android deve referenciar `@xml/network_security_config`, e o arquivo XML correspondente deve existir com configuração para `musicbrainz.org`.
3. `MetaBrainzProvider` em `lacraia-core` deve utilizar User-Agent canônico `Lacraia/0.2.0 ( https://lacraia.2lp.in )`.
4. Testes automatizados em Rust devem assegurar a presença de todas as declarações de conformidade de rede e User-Agent.
