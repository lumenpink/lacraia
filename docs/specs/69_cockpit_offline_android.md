# Spec: Cockpit Offline Embarcado e Resiliência contra Tela Preta (#69)

## Contexto
O aplicativo Android do La Craia (`in.twolp.lacraia.app`) inicializava com a URL padrão `http://10.0.2.2:2011`, que só é resolvida dentro do emulador Android SDK. Em dispositivos físicos, esse endereço é inalcançável, e na ausência de assets empacotados e de tratamento de erro no WebViewClient, o aplicativo apresentava uma tela preta perpétua.

## Objetivo
1. Embarcar os assets do Cockpit (`site/app`) diretamente no pacote do Android (`assets/index.html` e recursos visuais) via configuração de `sourceSets` no Gradle.
2. Atualizar o `MainActivity.kt` para carregar o Cockpit embarcado (`file:///android_asset/index.html` ou via `WebViewAssetLoader`) na ausência de servidor configurado ou quando a conexão de rede falhar.
3. Adicionar tratamento resiliente no `WebViewClient` (`onReceivedError`) que previna tela preta, redirecionando graciosamente para o Cockpit autônomo local e alertando o usuário.

## Critérios de Aceitação
1. O `build.gradle.kts` do módulo `:app` deve declarar os diretórios `site/app` e `site/assets` em `sourceSets.main.assets.srcDirs`.
2. `MainActivity.kt` deve implementar fallback para o asset local em `onReceivedError` ou quando nenhum endpoint externo for alcançado.
3. Testes unitários/integração em `packages/lacraia-app/tests/server_config_test.rs` devem validar que o fallback local para `android_asset` e resiliência de erro existem no código Kotlin.
