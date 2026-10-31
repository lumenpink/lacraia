# ☭ ГОСТ / GOSPLAN — Especificação Canônica Consolidada

> **Aviso de Integridade Gosplan:**
> Este documento foi montado automaticamente pelo CCCP (`cccp profile assemble`).
> A precedência é estrita de baixo para cima: cada camada subsequente sobrepõe as definições anteriores.

### 🌳 Árvore de Herança e Precedência Resolvida:
1. Camada 1: `https://gost.2lp.in/SPEC.md`
2. Camada 2: `https://kollontai.2lp.in/profile.md`
3. Camada 3: `https://lilya.2lp.in/profile.md`

---

<!-- ===================================================================== -->
<!-- CAMADA 1: https://gost.2lp.in/SPEC.md -->
<!-- ===================================================================== -->

# GOST — Padrão Técnico para Repositórios Abertos
**Versão da Norma:** `2.0.0`  
**Status:** Oficial / Padrão Canônico  
**Referência:** ГОСТ (Государственный Стандарт / General Open Source Standard)

---

## 1. Escopo e Propósito

Esta especificação define os requisitos estruturais, metodológicos, de governança e de documentação para criação, manutenção e ciclo de vida de repositórios de software aberto.

O objetivo do GOST é eliminar atritos no surgimento de novos projetos, garantir rastreabilidade histórica completa, assegurar a máxima liberdade do código e impor rigor de engenharia orientado à documentação e testes.

As palavras-chave **DEVE** (*MUST*), **NÃO DEVE** (*MUST NOT*), **DEVERIA** (*SHOULD*), **NÃO DEVERIA** (*SHOULD NOT*) e **PODE** (*MAY*) neste documento devem ser interpretadas conforme descrito na [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119).

---

## 2. Filosofia e Licenciamento

Todo projeto em conformidade com o GOST **DEVE** declarar sua licença de forma explícita na raiz do repositório através de um arquivo `LICENSE`. A especificação técnica detalhada, perfis e matriz normativa encontram-se em [specs/legal/licenses.md](./specs/legal/licenses.md).

### 2.1 Princípio da Menor Permissividade como Padrão Raiz
* **Regra da Raiz:** O arquivo `LICENSE` na raiz **DEVE** ser a licença padrão e geral de todo o repositório.
* Em repositórios com múltiplos tipos de artefatos (como monorepos ou aplicações híbridas), o arquivo `LICENSE` **DEVE** conter a licença menos permissiva (mais restritiva) adotada (ex: `GPL-3.0`).
* Quaisquer licenças mais permissivas (ex: `Apache-2.0` para sub-bibliotecas ou `CC-BY-SA-4.0` para mídias) funcionam como exceções modulares declaradas.

### 2.2 Tipologia por Perfil de Repositório
1. **Aplicações Completas / Monorepos Híbridos:** Adotam `GPL-3.0` (ou `AGPL-3.0` para serviços web) na raiz (`LICENSE`).
2. **Utilitários, Ferramentas Autônomas e Microapps:** Adotam `Apache-2.0` na raiz (`LICENSE`) diretamente, sem necessidade de arquivos satélites supérfluos.
3. **Bibliotecas, Protocolos e SDKs:** Adotam `Apache-2.0` na raiz (`LICENSE`) para máxima interoperabilidade.
4. **Especificações e Normas:** Adotam `CC-BY-SA-4.0` para os textos normativos na raiz (`LICENSE`), com esquemas e templates em `CC0-1.0` e marcas protegidas via satélite.

### 2.3 Arquivos Satélites Padronizados
Quando houver artefatos que exijam tratamentos jurídicos específicos, os seguintes arquivos canônicos **DEVEM** ser utilizados:
* `LICENSE-MEDIA`: Creative Commons Atribuição-CompartilhaIgual 4.0 Internacional (`CC-BY-SA-4.0`) para mídias, áudios, vídeos, fontes e ilustrações.
* `LICENSE-LOGO`: Creative Commons Atribuição-SemDerivações 4.0 Internacional (`CC-BY-ND-4.0`) para marcas, insígnias, ícones oficiais e logotipos.
* `LICENSE-APACHE`: Apache License 2.0 (`Apache-2.0`) para módulos, sub-crates ou bibliotecas desacopladas contidas em monorepos de aplicações.

### 2.4 Mapeamento Obrigatório no `README.md`
A presença de qualquer arquivo satélite (`LICENSE-MEDIA`, `LICENSE-LOGO` ou `LICENSE-APACHE`) **OBRIGA** a inclusão de uma seção explícita de mapeamento de diretórios e arquivos cobertos por cada licença no `README.md`.

### 2.5 Princípio Anti-Boilerplate
Cada repositório implementa exclusivamente as licenças condizentes com seu perfil real. É vedado copiar blocos genéricos de licenciamento e arquivos satélites vazios para projetos simples.

---

## 3. Anatomia Mínima da Raiz e Metadados

Todo repositório GOST **DEVE** conter em sua raiz a seguinte estrutura mínima independente da stack ou linguagem utilizada:

```text
.
├── GOSTVER                     # Declaração canônica de versão da norma e perfil
├── .gost/
│   └── metadata.json           # Manifesto SSOT de metadados do projeto (Regra GOST-26)
├── LICENSE                     # Termo legal de licenciamento padrão (Seção 2)
├── LICENSE-MEDIA               # (Opcional) Licença CC-BY-SA para mídias e assets
├── LICENSE-LOGO                # (Opcional) Licença CC-BY-ND para marcas e logos
├── LICENSE-APACHE              # (Opcional) Licença Apache-2.0 para bibliotecas internas
├── VERSION                     # Versão semântica em texto plano (Seção 4)
├── CHANGELOG.md                # Registro cronológico de mudanças (Seção 4)
├── README.md                   # Apresentação executiva e portal enxuto (Seção 9)
├── .gitignore                  # Higiene do repositório e arquivos ignorados
├── .editorconfig               # Configuração canônica de formatação para editores
├── site/                       # Portal estático de documentação pública (Seção 9)
│   ├── index.html              # Ponto de entrada da documentação
│   └── _headers                # Cabeçalhos HTTP de segurança (ou wrangler.jsonc)
└── .forgejo/workflows/ci.yml   # Esteira hermética de CI/CD em arquivo único (Seção 10)
```

### 3.1 Proibição de Poluição na Raiz
É terminantemente proibida a presença de arquivos avulsos de linters (`.prettierrc`, `.prettierrc.json`, `.eslintrc`, `.eslintrc.js`) ou arquivos de orquestração legados (`compose.yml`, `docker-compose.yml`, `docker-compose.yaml`). Arquivos de configuração devem residir em subdiretórios apropriados ou aderir estritamente aos nomes canônicos (`compose.yaml`).

### 3.2 Manifesto Canônico de Metadados (`.gost/metadata.json` — Regra `GOST-26`)
O arquivo `.gost/metadata.json` é a autoridade central (SSOT) para identidade, vitrine e catálogos:
* `slug`: Identificador único minúsculo em kebab-case ($\le 24$ caracteres, `^[a-z0-9-]+$`).
* `title`: Nome próprio legível do aplicativo ou biblioteca ($\le 25$ caracteres).
* `site_type`: Tipo de vitrine pública (`"doc"` para documentação/CLI ou `"app"` para aplicação web interativa).
* `tagline`: Subtítulo resumido em uma frase clara ($\le 60$ caracteres), respeitando `len(title + " — " + tagline) <= 65`.
* `summary`: Resumo executivo objetivo ($50$ a $140$ caracteres), consumido em meta tags Open Graph e feeds.
* `abstract`: Resumo expandido ($150$ a $500$ caracteres) utilizado no lead do `README.md` e hero do site.
* `tags`: Lista contendo entre 3 e 8 tags em kebab-case ($\le 20$ caracteres cada).
* `icon`: Ícone representativo ou caractere visual.
* `theme`: Objeto com `bg_color` e `accent_color` em formato hexadecimal.
* **Filtro Anti-Jargão:** As descrições em prosa **NÃO DEVEM** conter termos técnicos vazios ou autorreferenciais (`"soberano"`, `"soberana"`, `"hexagonal"`, `"ssot"`, `"rust puro"`, `"em rust"`, `"em php"`, `"wasm"`).

---

## 4. Versionamento Semântico e Histórico

### 4.1 Arquivo `VERSION` e Conformidade SemVer 2.0.0
* O repositório **DEVE** manter um arquivo `VERSION` na raiz contendo exclusivamente a versão atual do projeto em formato de texto puro UTF-8, finalizado por quebra de linha.
* O formato de versionamento **DEVE** aderir estritamente ao [Semantic Versioning 2.0.0](https://semver.org/) (`MAJOR.MINOR.PATCH[-PRERELEASE][+BUILD]`).
* **Exibição Dual Padronizada:**
  - *Versão Short:* `v{MAJOR}.{MINOR}.{PATCH}` (truncada antes do `-` ou `+`) para cabeçalhos, banners e títulos.
  - *Versão Long:* String integral idêntica ao arquivo `VERSION` para rodapés e CLI `--version`.
* **Sincronia Multi-Arquivo Mandatória:** A versão base declarada em `VERSION` deve coincidir rigorosamente com a versão declarada no topo do `CHANGELOG.md`, no arquivo `site/index.html` e nos manifestos de pacotes correspondentes (`Cargo.toml`, `package.json`, `composer.json`, `DESCRIPTION`).

### 4.2 Arquivo `CHANGELOG.md`
* O repositório **DEVE** manter um histórico de modificações legível por humanos e baseado no padrão [Keep a Changelog](https://keepachangelog.com/).
* As seções de cada versão **DEVEM** categorizar alterações em: `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security` ou subdivisões semânticas convencionais.

---

## 5. Ciclo DD-TDD, Branches e o Protocolo de 9 Fases

O fluxo de trabalho em repositórios GOST **DEVE** seguir o ciclo rigoroso de **Documentation-Driven & Test-Driven Development (DD-TDD)** em 9 fases sequenciais e indivisíveis, governado pela regra de rastreabilidade estrita de tickets e isolamento em branches.

### 5.1 Cláusulas Pétreas do Ciclo

1. **Inviolabilidade das 9 Fases e Inferência Automática:** Todas as 9 etapas **DEVEM** ser seguidas em ordem sequencial estrita. O desenvolvedor não informa a fase manualmente; o Maestro de Governança deduz a fase (`ph3` a `ph9`) a partir do estado do histórico e dos arquivos alterados no commit.
2. **Zero Código sem Ticket e Documentação Prévia:** Nenhuma alteração em código de produção é permitida sem ticket aberto e especificação prévia de contrato/documentação.
3. **Isolamento Estrito da Fase de Documentação (Fase 3):** Na Fase 3, é terminantemente proibido alterar qualquer linha de código executável em `src/`, `lib/`, `bin/` ou `tests/`. Apenas especificações textuais, contratos e esquemas são aceitos.
4. **Rastreabilidade Estrita de Commits e Marcadores de Fase:**
   - Commits intermediários **DEVEM** seguir o padrão: `<tipo>: <resumo> (ref #<id> - ph<N>)` (onde `N` varia de 3 a 8).
   - O commit final de encerramento da Fase 9 **DEVE** seguir o padrão: `<tipo>: <resumo> (closes #<id> - ph9)`.
5. **Laudo Textual de Resolução Obrigatório na Fase 9:** O commit de fechamento da Fase 9 (`closes #<id> - ph9`) **DEVE** conter corpo textual ($\ge 80$ caracteres) com os cabeçalhos formais:
   - `[Resumo]:` Síntese objetiva do escopo tratado.
   - `[Solução]:` Descrição das modificações arquiteturais e técnicas efetuadas.
   - `[Validação]:` Relação dos testes determinísticos e gates executados para comprovação.  
   *No momento do merge, o Maestro extrai automaticamente esse laudo e o registra como comentário de fechamento no issue tracker.*
6. **Fast-Tracks Declarados:**
   - `docs`: Isenção formal das Fases 4 a 7 (exige estritamente as Fases 1, 2, 3, 8 e 9).
   - `chore`: Isenção formal das Fases 4 e 7 para manutenções de infraestrutura (exige estritamente as Fases 1, 2, 3, 5, 8 e 9).
7. **Proibição Absoluta de Commit Direto na `main`:** É estritamente proibido commitar diretamente na branch base `main`. Todo trabalho reside em branch de ticket isolada.

### 5.2 Estratégia de Branches e Governança Local-First
* **Branches de Trabalho:** Devem seguir o padrão `^(feat|fix|docs|refactor|test|chore)/([0-9]+)-([a-z0-9-]+)$`.
* **Governança Local-First:** As branches de trabalho existem exclusivamente no ambiente local. O envio de branches de trabalho para a forja remota é proibido, exceto se autorizado via flag explícita (`--multidev`).
* **Branches Persistentes Autorizadas:** Além da `main` (e `pages`), são autorizadas no remote `origin`:
  - Branches de manutenção de versão maior: `v<MAJOR>` ou `release/v<MAJOR>.x`.
  - Branches de épicos multi-ticket: `epic/<slug>`.
* **Protocolo de Gênese (Ticket #1):** Novos repositórios são inicializados pelo Maestro (`init`), abrindo a Issue #1 (`chore`), gerando a anatomia básica, emitindo o Laudo de Gênese no commit inaugural da `main` e disparando a publicação inicial.

### 5.3 As 9 Fases Canônicas do Ciclo DD-TDD

| Fase | Nome | Obrigatoriedade | Padrão do Commit |
| :---: | :--- | :---: | :--- |
| **1** | **Ticket / Issue** | ✅ **OBRIGATÓRIO** | *(Abertura no tracker — sem commit)* |
| **2** | **New Branch** | ✅ **OBRIGATÓRIO** | *(Criação da branch — sem commit)* |
| **3** | **Doc / Spec Contract** | ✅ **OBRIGATÓRIO** | `docs: specify <comportamento> (ref #<id> - ph3)` |
| **4** | **Red: Failing Test** | 🔴 **OBRIGATÓRIO** *(exceto fast-track)* | `test: add failing test for <requisito> (ref #<id> - ph4)` |
| **5** | **Green: Implement** | ✅ **OBRIGATÓRIO** | `feat: implement <funcionalidade> (ref #<id> - ph5)` ou `fix: resolve <bug> (ref #<id> - ph5)` |
| **6** | **Refactor to Excellence** | 🟡 **CONDICIONAL** | `refactor: optimize and polish <escopo> (ref #<id> - ph6)` |
| **7** | **Mutation Test & Fortify** | 🟡 **CONDICIONAL** *(ordem estrita)* | `test: eliminate surviving mutants in <escopo> (ref #<id> - ph7)` *(testes primeiro)*;<br>`refactor: eliminate dead code in <escopo> (ref #<id> - ph7)` *(código depois)* |
| **8** | **Fast Local Hygiene** | ✅ **OBRIGATÓRIO** | `fix: lint and formatting hygiene (ref #<id> - ph8)` *(se houver ajustes)* |
| **9** | **Finish Docs & Close** | ✅ **OBRIGATÓRIO** | `<tipo>: finalize <escopo> (closes #<id> - ph9)` *(com Laudo no corpo)* |

---

## 6. O Maestro de Governança

### 6.1 Política de Ponto Único de Entrada (*Single Entrypoint*)
O gerenciamento do ciclo de vida, git, tickets e conformidade deve ser operado exclusivamente através do Maestro de Governança CLI. É terminantemente proibido o uso de comandos manuais do Git (`git commit`, `git merge`, `git push`, `git pull`, `git tag`) ou ferramentas ad-hoc de terceiros (`tea`, `gh`, `glab`).

### 6.2 Contrato Universal de Subcomandos do Maestro
Qualquer implementação do Maestro de Governança compatível com a GOST deve expor a seguinte interface determinística:
* `init`: Inicializa repositório, estrutura e protocolo de gênese.
* `ticket`: Consulta, abertura, triagem e fechamento de tickets.
* `divan`: Assistente interativo de triagem e planejamento.
* `start <id>`: Cria e faz checkout da branch de trabalho padronizada.
* `status`: Audita a conformidade atual do repositório, branch e hooks.
* `commit`: Valida e cria commits com inserção determinística de tags de ticket e fase (`ph3` a `ph8`).
* `mutate`: Executa testes de mutação restritos ao diff local.
* `hygiene`: Executa os formatadores determinísticos e linters rápidos (Fase 8).
* `close`: Registra o laudo e gera o commit canônico de encerramento (`closes #<id> - ph9`).
* `merge`: Realiza o merge validando o laudo e a tag de fechamento.
* `push`: Executa pré-checagens e realiza o envio para o remote autorizado.
* `release`: Conduz o processo de tag semântica e auditoria pré-release.
* `gost sync/check`: Sincroniza e audita a conformidade das regras técnicas.

### 6.3 Protocolo de Comandos para Agentes de IA
Agentes autônomos de IA devem seguir o vocabulário de comandos operacionais determinísticos:
* `DCPD`: Despacho completo com planejamento e documentação prévia.
* `DCP`: Despacho padrão de implementação e correção.
* `PLAN`: Elaboração e detalhamento de plano de ação sem escrita de código.
* `TDD`: Execução estrita do ciclo Red-Green-Refactor.
* `AUDIT`: Auditoria estática de conformidade e integridade.
* `FIX`: Correção de bugs pontuais com teste de regressão comprovado.
* `RELEASE`: Preparação e condução de versão estável.

---

## 7. Estratégia de Testes e Cobertura

### 7.1 Taxonomia de Testes
1. **Testes Unitários:** Isolamento absoluto, sem dependência de I/O externo, execução em milissegundos.
2. **Testes Funcionais:** Validação de regras de negócio e transformações de dados.
3. **Testes de Integração:** Validação entre subsistemas internos e adaptadores concretos.
4. **Testes Ponta a Ponta (E2E):** Simulação de interação real de usuários em navegadores headless via Playwright quando aplicável.

### 7.2 Delegação de Cobertura para a Esteira
* **Piso Mínimo Inviolável:** A cobertura global de testes automatizados **DEVE** ser de no mínimo **`90%`**, com meta de **`100%`** no domínio.
* **Aferição na Esteira Remota:** A verificação estrita de cobertura com quebra de build é critério bloqueante delegado soberanamente à esteira remota hermética (`ci.yml`), liberando o fluxo de desenvolvimento local de overhead excessivo.

### 7.3 Mutação Restrita ao Diff
Na Fase 7 local, as ferramentas de teste de mutação devem operar restritas ao diff de alterações (`--in-diff` ou `--git-diff-lines`), deixando baterias globais de mutação para esteiras de auditoria periódica.

---

## 8. Análise Estática e Tolerância Zero a Avisos (*Zero Warnings*)

### 8.1 Tolerância Zero a Warnings
Linters e analisadores estáticos devem operar com severidade máxima (ex: `-D warnings`). Avisos são tratados como erros fatais e bloqueiam a integração. É proibida a permanência de tokens de depuração (`dbg!`, `debugger;`, `var_dump`, `dd()`, `print_r`) em código final.

### 8.2 Formatação Determinística e `.editorconfig`
Todo projeto deve adotar um formatador oficial determinístico verificado com flags `--check`. O arquivo `.editorconfig` é obrigatório na raiz e padroniza UTF-8, LF, newline final e remoção de espaços em branco.

---

## 9. Vitrine Pública, SSOT e Banners

### 9.1 Topologia de Vitrine (`site_type`)
* **`site_type: doc`:** O diretório `./site/` serve documentação técnica, tutoriais e especificações.
* **`site_type: app`:** O diretório `./site/` hospeda a aplicação gráfica/interativa, reservando o subdiretório `./site/doc/` para manuais e documentação de apoio.

### 9.2 SSOT em Markdown e README Enxuto
* **SSOT:** Documentações, manuais e guias residem estritamente em Markdown. É proibido duplicar textos extensos manualmente em HTML.
* **README Enxuto:** O arquivo `README.md` na raiz deve ter um teto rígido de **50 linhas**, servindo como portal rápido para a vitrine oficial.

### 9.3 Banner Open Graph Minimalista
* **Resolução:** `1200 × 630 px` com Safe Zone centralizada 4:3 (`720–840px` $\times$ `500px`).
* **Conteúdo Visual:** Apenas o ícone/logo sobre as cores do tema e o texto `{title} — {tagline}` ($\le 65$ caracteres). Veto a badges de linguagem ou links impressos diretamente na imagem.
* **Sincronia de Meta Tags:** `<meta property="og:title">` deve equivaler a `{title} — {tagline}`, e `<meta property="og:description">` ao `{summary}` ($\le 140$ caracteres).

### 9.4 Rodapé Canônico em Duas Linhas
Aplicações e documentações estáticas devem exibir rodapé em duas linhas:
* **Linha 1 — Afirmação Identitária:** Definida e parametrizada conforme o perfil adotado.
* **Linha 2 — Metadados de Soberania e Licença:** `{Nome do App} v{Versão} • [{Licença}](LICENSE)`.

### 9.5 Cabeçalhos HTTP de Segurança
A vitrine estática deve declarar cabeçalhos `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY` e `Referrer-Policy: strict-origin-when-cross-origin` em `site/_headers` ou `wrangler.jsonc`.

---

## 10. Esteira Hermética de CI/CD

### 10.1 Arquivo Único e Hermeticidade
O pipeline de automação reside estritamente em `.forgejo/workflows/ci.yml`. É proibida a existência da pasta `.github/workflows/`. São proibidos comandos de download ou instalação em runtime (`apt-get`, `apk add`, `pip install`, `npm i -g`, `cargo install`, `wget`, `curl`). Todas as ferramentas necessárias devem estar pré-instaladas na imagem base local do runner.

### 10.2 Filas Canônicas e Dependência Estrita
* Filas padronizadas: `test` (Quality Gate), `build`/`release` e `deploy`.
* A fila `test` é obrigatória; todas as filas posteriores devem declarar expressamente `needs: [test]`.
* Builds de release e empacotamento ocorrem exclusivamente na esteira; é proibido compilar releases pesadas na máquina local.

### 10.3 Zero Binários no Histórico Git (Regra `GOST-24`)
É expressamente proibido versionar artefatos compilados, arquivos ELF, pacotes ou tarballs (`.exe`, `.dll`, `.so`, `.tar.gz`, `.zip`, `.deb`, `.rpm`, `.apk`) no repositório Git.

---

## 11. Internacionalização (Intl)

### 11.1 Paridade Matemática de Chaves nas 10 Línguas
Projetos internacionalizados devem manter paridade estrita de chaves nos 10 idiomas canônicos: `en`, `pt`, `es`, `fr`, `de`, `it`, `ru`, `zh`, `ja` e `ar`.

### 11.2 Suporte RTL e Propriedades Lógicas
O idioma árabe (`ar`) requer ativação do atributo `dir="rtl"`. Interfaces devem empregar propriedades lógicas em CSS (`margin-inline`, `padding-inline`), banindo propriedades físicas horizontais estáticas.

---

## 12. Arquitetura de Domínio, DDD e SOLID (Regra `GOST-27`)

### 12.1 Pureza de Domínio (Zero I/O no Core)
O diretório e módulos de domínio/core (`src/domain/`, `lib/core.sh`, `internal/domain/`, etc.) **NÃO DEVEM** conter operações ou importações diretas de I/O (rede, filesystem, banco de dados, variáveis de ambiente ou chamadas a processos de sistema).

### 12.2 Inversão de Dependência e In-Memory Fakes
* O Core comunica-se com o mundo externo exclusivamente através de abstrações (Portas).
* Mocks dinâmicos pesados são rejeitados em favor de adaptadores em memória leves (*In-Memory Fakes*) baseados em tipos nativos da linguagem.
* Fakes e adaptadores reais compartilham a mesma suíte de testes de contrato (LSP).

### 12.3 Validação na Borda e Primitivos Confiáveis
A entrada de dados é validada e sanitizada nas bordas do sistema (*Parse, Don't Validate*). O Core opera com tipos confiáveis e enums nativos da linguagem para modelar estados finitos.

---

## 13. Aderência a Padrões de Sistema e Operação (XDG, 12-Factor & OCI)

### 13.1 Diretórios do Sistema (XDG Base Directory)
Aplicações devem respeitar `$XDG_CONFIG_HOME`, `$XDG_DATA_HOME`, `$XDG_STATE_HOME`, `$XDG_CACHE_HOME` e `$XDG_RUNTIME_DIR`. É terminantemente proibido criar arquivos e pastas ocultas soltas na home do usuário (`~/.app`). Devem oferecer suporte a `NO_COLOR`.

### 13.2 Contêineres OCI e Compose
* Manifestos de orquestração devem adotar o nome canônico `compose.yaml`.
* Dockerfiles devem implementar build multi-stage, execução sob usuário não-root, healthchecks e banimento de tags `:latest`. É proibido o uso de Watchtower.

---

## 14. Perfis de Implementação e Política de Waivers

### 14.1 Herança em Cascata de Perfis
Perfis especializados (`profile.md`) especializam e complementam a norma central em ordem hierárquica de precedência através da diretiva `extends`.

### 14.2 Mecanismo de Waivers Auditados
Exceções temporárias a regras técnicas devem ser formalizadas com auditoria rastreável:
* **Arquivo `custom_rules.json`:** Declara regras ignoradas exigindo `rule_id`, `reason` ($\ge 20$ caracteres) e ID do ticket associado.
* **Diretiva Inline:** `gost-ignore: <RULE_ID> (ticket #<id> - <motivo>)`.
* Agentes autônomos de IA são terminantemente proibidos de criar ou editar diretivas de waiver.

---

## 15. Governança de Fronteira e Quarentena de Branches (`inbox/*`)

### 15.1 Topologia de Remotes Git
O remote `origin` na forja primária é o único destino autorizado para operações de push. Espelhos remotos públicos operam exclusivamente como mirrors de consulta e devem ser configurados com `pushurl = no_push`.

### 15.2 Quarentena de Contribuições Externas
Contribuições externas e Pull Requests vindos de espelhos são recebidos sob isolamento no namespace `refs/heads/inbox/*`, sujeitos a esteiras não-confiáveis offline antes de qualquer integração.

---

<!-- ===================================================================== -->
<!-- CAMADA 2: https://kollontai.2lp.in/profile.md -->
<!-- ===================================================================== -->

# Perfil de Implementação GOST: Kollontai

```yaml
---
id: kollontai
name: "Perfil de Implementação Kollontai"
extends: "https://gost.2lp.in/SPEC.md"
target_gost: "2.0.0"
status: "Ativo / Produção"
---
```

> **Sobre este Perfil:**  
> O perfil **Kollontai** (em homenagem à revolucionária soviética, feminista e diplomata Alexandra Kollontai) define o ferramental concreto de engenharia, a implementação oficial do Maestro de Governança (**`cccp`**), a matriz de versões de compiladores, o protocolo de auto-atualização de binários e a arquitetura de distribuição para projetos aderentes ao padrão **GOST**.  
> Ele é agnóstico a domínios pessoais e identidades individuais, servindo como perfil base para coletivos, laboratórios ou perfis autorais derivados (como o perfil `lumen`).

---

## 1. O Maestro Concreto: `cccp` e `.cccprc`

No perfil Kollontai, a autoridade de governança definida na Seção 6 da norma GOST é materializada pelo binário **`cccp`** (integrado ao sistema de triagem **`divan`**).

### 1.1 Arquivo de Configuração `.cccprc`
Todo repositório sob o perfil Kollontai **DEVE** declarar na raiz (ou herdar via configuração global em `$XDG_CONFIG_HOME/cccp/config.env`) os parâmetros operacionais do Maestro:

```ini
# Idioma primário para triagem de tickets (divan), Laudo da Fase 9 e fallback de i18n
agent_var_lang = portuguese

# Diretivas de Forja Primária (Sobrescritas pelo perfil filho ou pelo repositório)
primary_forge_type = forgejo
primary_owner = <owner>
primary_repo = <slug>

# Espelhos Passivos e Ingestão de Fronteira (Separados por vírgula: codeberg,github)
mirrors = codeberg,github
mirror_owner = <mirror_owner>
```

### 1.2 Mapeamento de Comandos do Maestro (`cccp`)
É estritamente proibido invocar comandos brutos de mutação do Git ou CLIs de terceiros (`tea`, `gh`, `glab`). O ciclo de vida completo opera via `cccp`:

| Subcomando `cccp` | Fase / Escopo | Ação Determinística Executada pelo Maestro |
| :--- | :---: | :--- |
| `cccp init --slug <s> ...` | **Dia Zero** | Cria repo local (`main`), provisiona forja primária e espelhos (`pushurl = no_push`), abre o Ticket `#1` (`chore`), gera anatomia GOST + `.gost/metadata.json`, comita `(closes #1 - ph9)` com Laudo de Gênese, fecha o Ticket `#1` e faz o primeiro push. |
| `cccp ticket create` | **Fase 1** | Cria a issue na forja primária (com tipo, contexto e critérios de aceite) e sincroniza o cache local em `.git/cccp/tickets/<id>.json`. |
| `cccp divan` | **Fase 1** | Sessão assistida de triagem, quebra de épicos e refinamento de tickets. |
| `cccp start <id> [--base <b>]` | **Fase 2** | Lê os metadados do ticket `#<id>`, atualiza a branch base (`main`, `vN` ou `epic/*`) e faz checkout automático para `<tipo>/<id>-<slug>`. |
| `cccp status` | **Fases 1–9** | Inspeciona o histórico da branch ativa e exibe o ticket atual, a fase alcançada (`ph3`..`ph9`) e o próximo passo obrigatório. |
| `cccp commit "<resumo>"` | **Fases 3–8** | Deduz o `<id>` e a fase atual (`ph3`..`ph8`), valida as travas da fase (bloqueia código em `ph3`; exige falha de teste em `ph4`; exige sucesso em `ph5`; impõe ordem `test` $\to$ `refactor` em `ph7`) e formata `<tipo>: <resumo> (ref #<id> - ph<N>)`. |
| `cccp mutate` | **Fase 7** | Executa a ferramenta de teste de mutação da linguagem ativa restrita ao diff da branch (`git diff <base>..HEAD`). |
| `cccp hygiene` / `cccp fix` | **Fase 8** | Executa instantaneamente o linter estático (`-D warnings`) e o verificador/aplicador de formatação determinística das linguagens declaradas no `GOSTVER`. |
| `cccp close "<resumo>"` | **Fase 9** | Exige o corpo estruturado com o **Laudo Textual de Resolução** (`[Resumo]:`, `[Solução]:`, `[Validação]:`), valida a atualização de `CHANGELOG.md`/`site/` e grava o commit final `(closes #<id> - ph9)`. |
| `cccp merge` | **Pós-Fase 9** | Verifica o cumprimento das fases e do Fast Hygiene, integra a branch na base (`main`, `vN` ou `epic/*`), posta o Laudo do commit `ph9` como comentário oficial no ticket encerrando-o na forja e remove a branch local. |
| `cccp epic start|finish <slug>` | **Épicos** | Cria ou finaliza branches persistentes de integração (`epic/<slug>`) para reescritas multi-ticket. |
| `cccp push [--multidev]` | **Publicação** | Bloqueia push de branches efêmeras de ticket (salvo `--multidev`) e envia branches persistentes (`main`, `vN`, `epic/*`) para o `origin`. |
| `cccp release <patch|minor|major>` | **Release** | Executa auditoria pré-tag em container (`act`), consolida o `CHANGELOG.md`, atualiza o `VERSION`, assina a tag Git e publica. |
| `cccp gost sync | check` | **Governança** | Resolve a cascata de perfis (`extends`), propaga o `.gost/metadata.json` e executa as auditorias determinísticas do `rules.json`. |
| `cccp inbox list | review | accept` | **Fronteira** | Gerencia contribuições externas isoladas no namespace de quarentena `refs/heads/inbox/*`. |

---

## 2. Topologia de Forjas, Espelhos e Fronteira

### 2.1 Soberania da Forja Primária e Espelhamento Passivo
1. **Unidirecionalidade Local (`Zero Dual Push`):** O ambiente local mantém o remote `origin` apontando exclusivamente para a forja primária definida no perfil final. A replicação de código para espelhos externos (Codeberg, GitHub) é responsabilidade exclusiva do *Push Mirror* automático da forja primária (`sync_on_commit: true`).
2. **Sinalização de Espelhos e Paridade de Metadados:**
   - O comando `cccp gost sync` mantém `description` (extraído de `summary`), `website` e `topics` (extraídos de `tags` no `.gost/metadata.json`) idênticos entre a forja primária e os espelhos.
   - Nos espelhos secundários, a descrição recebe automaticamente o prefixo canônico:
     `[MIRROR / BACKUP] {summary} — Repositório canônico: {primary_forge_url}`
3. **Configuração Obrigatória de Remotes para Ingestão de PRs:**
   Para permitir que o controlador de fronteira baixe Pull Requests externos sem risco de push acidental para os espelhos, o `.git/config` **DEVE** ser mantido pelo `cccp` no seguinte formato:
   ```ini
   [remote "origin"]
       url = <primary_ssh_url>
       fetch = +refs/heads/*:refs/remotes/origin/*

   [remote "codeberg"]
       url = https://codeberg.org/<mirror_owner>/<slug>.git
       fetch = +refs/pull/*/head:refs/remotes/codeberg/pr/*
       pushurl = no_push

   [remote "github"]
       url = https://github.com/<mirror_owner>/<slug>.git
       fetch = +refs/pull/*/head:refs/remotes/github/pr/*
       pushurl = no_push
   ```

---

## 3. Matriz de Toolchains Homologadas (*Toolchain Pinning*)

As regras arquiteturais (DDD/Hexagonal), estruturas de diretórios, templates de `Dockerfile` e comandos de cada linguagem residem nos módulos `specs/languages/<lang>.md` da norma GOST. O perfil Kollontai fixa as versões mínimas homologadas para execução local e nas imagens da esteira:

| Stack (`GOST_LANGUAGES`) | Versão Canônica / Runtime | Runner de Testes & Cobertura (≥90%) | Mutação (Fase 7) | Linter (`-D warnings`) & Formatador |
| :--- | :--- | :--- | :--- | :--- |
| **`rust`** | `rustc >= 1.99.0` (`edition = "2024"`, target estático `musl`) | `cargo test` + `cargo-llvm-cov` | `cargo-mutants` (`--in-diff`) | `clippy` + `rustfmt` + `cargo-audit` |
| **`php`** | `PHP >= 8.5.0` (compatibilidade reversa `>= 8.2`, `strict_types=1`) | `pest --coverage --min=90` | `infection` (`--git-diff-lines`) | `phpstan --level=max` (target 8.2) + `pint` + `composer audit` |
| **`shell`** | POSIX `/bin/sh` estrito (ou `Bash >= 5.2` justificado) | `shellspec --kcov` | Revisão de borda (Boundary/Subshells) | `shellcheck` + `shfmt -d -i 2 -ci -bn` |
| **`javascript`** | `Node.js >= 24.16.0 LTS` (ESM nativo) + `TypeScript >= 5.8` (`strict`) | `vitest` (v8 coverage) + `playwright` (E2E) | `stryker` | `biome check` + `npm audit` |
| **`r`** | `R >= 4.4.1` + `renv` (lockfile imutável `renv.lock`) | `testthat` (3e) + `covr` | Revisão de invariantes (`NA`/`NaN`/`Inf`) | `lintr` + `styler` + `roxygen2` |
| **`go`** | `Go >= 1.22+` (cgo desabilitado para binário estático puro) | `go test -race` + `go tool cover` | `gremlins` | `golangci-lint` + `gofmt` + `govulncheck` |
| **`python`** | `Python >= 3.12+` (tipagem estrita mypy, Poetry/uv) | `pytest --cov` | `mutatest` | `ruff check/format` + `mypy --strict` + `pip-audit` |

### 3.1 Framework Canônico de Interface Gráfica: Dioxus 0.8
Para aplicações gráficas multiplataforma em Rust (Web WASM, Desktop via Tao/Wry e Mobile Android/iOS), o perfil Kollontai adota o **Dioxus 0.8** (`dioxus = "0.8"`):
* **Ergonomia Mobile Obrigatória:** Respeito a `env(safe-area-inset-top)` / `env(safe-area-inset-bottom)`, alvos de toque mínimos de `48px` com `touch-action: manipulation` e navegação adaptativa (barra inferior em telas `< 640px` e barra lateral compacta em `640px–1024px`).

---

## 4. Protocolo de Auto-Atualização Soberana (XDG) e Manifesto de Distribuição

Todas as ferramentas de linha de comando (CLI) e aplicações desktop distribuídas sob o perfil Kollontai **DEVEM** implementar o protocolo de auto-atualização conectado ao catálogo de distribuição soberano (a **Banca**) e interoperável com a **Jornaleira**.

### 4.1 Caminhos XDG e Formato de Configuração (`config.env`)
Para manter conformidade com a regra anti-bloat da norma GOST (Seção 13.2), a configuração e o cache de estado residem estritamente em caminhos XDG:
1. **Configuração do Usuário:** `${XDG_CONFIG_HOME:-~/.config}/<slug>/config.env`  
   Formato plano canônico `CHAVE="VALOR"` (compatível com POSIX Shell, Rust, Go e C):
   ```sh
   UPDATE_CHANNEL="stable"           # Canal rastreado: "stable" ou "nightly"
   UPDATE_PINNED="false"             # Se "true", bloqueia atualizações e silencia alertas
   UPDATE_PIN_VERSION=""             # Versão fixada quando UPDATE_PINNED="true"
   UPDATE_AUTO_CHECK="true"          # Verificação periódica não-bloqueante em modo autônomo
   UPDATE_CHECK_INTERVAL_HOURS="12"  # Frequência padrão de consulta (12 horas)
   UPDATE_AUTO_UPDATE="false"        # Substituição atômica sem confirmação (opt-in estrito)
   UPDATE_MANAGED_BY_JORNALEIRA=""   # Override: "" (auto-detectar), "true" ou "false"
   ```
   *Precedência de Resolução:* Flags CLI (`--channel`, `--pin`) > Variáveis de ambiente (`<SLUG_UPPER>_UPDATE_*` / `UPDATE_*`) > `${XDG_CONFIG_HOME}/<slug>/config.env` > Padrões normativos Kollontai.

2. **Estado e Cache de Runtime:** `${XDG_STATE_HOME:-~/.local/state}/<slug>/update_check.json`  
   Registro estruturado compartilhado entre o aplicativo e a Jornaleira:
   ```json
   {
     "last_check_timestamp": 1729411200,
     "current_version": "0.1.0",
     "latest_remote_version": "0.1.2",
     "channel": "stable",
     "update_available": true,
     "last_check_status": "success",
     "managed_by": "jornaleira"
   }
   ```

### 4.2 A Jornaleira como Gestora Primária e Comportamento em Runtime
1. **Hegemonia da Jornaleira:** A Jornaleira atua como a gestora primária do ecossistema.  
   * **Detecção:** Se `UPDATE_MANAGED_BY_JORNALEIRA="true"` ou se `command -v jornaleira >/dev/null 2>&1` retornar sucesso (sem override explícito `"false"`):
     * O **auto-check nativo dos aplicativos no startup permanece sumariamente desativado**.
     * Nenhuma requisição de rede ou consulta DNS é disparada durante a invocação normal de comandos do aplicativo.
     * A consulta remota periódica e a substituição de binários passam a ser orquestradas exclusivamente pelos timers e rotinas centrais da Jornaleira.
2. **Modo Autônomo (Sem Jornaleira):**  
   * Se a Jornaleira não estiver presente (ou `UPDATE_MANAGED_BY_JORNALEIRA="false"`), o utilitário assume a responsabilidade de verificar atualizações de forma não-bloqueante no startup caso o intervalo de 12 horas tenha expirado, com timeout estrito de $\le 1\text{s}$ descartando falhas silenciosamente.
3. **Alerta Não-Intrusivo em `stderr`:**  
   Se o arquivo `${XDG_STATE_HOME}/<slug>/update_check.json` indicar `update_available: true`, emite alerta exclusivamente em `stderr` (preservando pipelines em `stdout`):
   * *Quando sob gestão da Jornaleira:*
     ```text
     💡 [<slug>] Nova versão disponível: vX.Y.Z (atual: vA.B.C). Execute 'jornaleira update <slug>' para atualizar.
     ```
   * *Quando em modo autônomo:*
     ```text
     💡 [<slug>] Nova versão disponível: vX.Y.Z (atual: vA.B.C). Execute '<slug> update' para atualizar.
     ```

### 4.3 Contrato de Subcomandos CLI e Persistência de Canais
Todo binário distribuído **DEVE** responder aos subcomandos:
* `<slug> check-update [--channel <stable|nightly>] [--stable] [--nightly]`  
  Realiza consulta remota síncrona não-bloqueante na Banca, atualiza o arquivo `${XDG_STATE_HOME}/<slug>/update_check.json` e informa o status.
* `<slug> update [--channel <stable|nightly>] [--stable] [--nightly] [--pin <ver>] [--force] [--dry-run]`  
  Efetua a substituição atômica pelo binário mais recente da Banca respeitando o canal solicitado.
* `<slug> config show | set-channel <c> | pin [<ver>] | unpin | set-auto-check <bool> | set-auto-update <bool>`  
  Exibe ou altera de forma persistente as opções em `${XDG_CONFIG_HOME}/<slug>/config.env`.

#### **Garantia de Persistência do Canal Nightly:**
* A preferência declarada em `UPDATE_CHANNEL` (em `config.env`) define a assinatura duradoura de atualizações do usuário.
* O comando pontual `<slug> update --stable` (ou `--channel stable`) instala a compilação estável requisitada naquele momento, **mas NUNCA altera o valor de `UPDATE_CHANNEL`** no arquivo `config.env`.
* A reconfiguração persistente do canal exige comando explícito (`<slug> config set-channel stable` ou flag `--save-channel`).
* Enquanto `UPDATE_CHANNEL="nightly"`, o verificador continuará consultando e alertando sobre novas compilações do canal `nightly` assim que publicadas na Banca, mesmo que o executável local em execução seja momentaneamente uma versão estável.

### 4.4 Topologia do Catálogo de Distribuição (Banca) e Retenção Estrita
A esteira de CI publica os binários estáticos e mantém o armazenamento enxuto sob a seguinte estrutura canônica por aplicativo (`<slug>`):
* `<banca_url>/downloads/<slug>/install.sh`: Instalador POSIX puro (`#!/bin/sh`), verifica hash SHA-256 e instala em `~/.local/bin` (ou `/usr/local/bin` se root).
* `<banca_url>/downloads/<slug>/<channel>-latest` e `<channel>-latest.version`: Ponteiros atômicos para a versão ativa de cada canal (`stable` e `nightly`).
* `<banca_url>/downloads/<slug>/index.json`: Manifesto estruturado contendo metadados extraídos do `.gost/metadata.json` (`slug`, `title`, `tagline`, `summary`, `tags`), canais, URLs imutáveis, tamanho em bytes, data ISO-8601 e hashes `sha256`.
* **Política de Retenção e Expurgo Automático:** A publicação na esteira retém estritamente as **5 versões `stable`** mais recentes e as **10 compilações `nightly`** mais recentes, expurgando artefatos antigos automaticamente.

---

## 5. Esteira de CI/CD Hermética (`.forgejo/workflows/ci.yml`)

Em conformidade com a Seção 10 da norma GOST, todo repositório Kollontai possui um único arquivo `.forgejo/workflows/ci.yml` executando em containers locais pré-provisionados (sem downloads externos em runtime), dividido nas filas canônicas:

```yaml
name: "Gosplan Hermetic Pipeline"

on:
  push:
    branches: [main, "v*", "release/v*", "epic/*"]
  pull_request:

jobs:
  test:
    name: "Quality Gate (Lint, Tests, Coverage >= 90%, Audit)"
    runs-on: local-hermetic
    steps:
      - name: "Checkout Code"
        uses: actions/checkout@v4
      - name: "Execute GOST Compliance & Test Suite"
        run: |
          cccp gost check
          cccp test --coverage-min 90

  release:
    name: "Build Static Artifacts & Publish to Banca"
    runs-on: local-hermetic
    needs: [test]
    if: github.ref == 'refs/heads/main'
    steps:
      - name: "Checkout Code"
        uses: actions/checkout@v4
      - name: "Compile & Dispatch to R2 / Banca"
        run: cccp ci publish-artifacts

  deploy:
    name: "Deploy Documentation Site & Containers"
    runs-on: local-hermetic
    needs: [test]
    if: github.ref == 'refs/heads/main'
    steps:
      - name: "Checkout Code"
        uses: actions/checkout@v4
      - name: "Deploy Static Showcase & Orphan Pages"
        run: cccp ci deploy-site
```

* **Isolamento da Quarentena (`inbox/*`):** Para branches `refs/heads/inbox/*`, apenas o job `test` é executado em modo desprivilegiado (sem acesso a secrets de deploy ou chaves da Banca).
* **Publicação Stateless em Branch `pages`:** O espelhamento estático para forjas secundárias utiliza worktree órfã (`git worktree add --orphan -b pages`) com commit único (`docs(site): deploy snapshot [skip ci]`) e `git push --force`, impedindo inflação do histórico Git.

---

## 6. Vitrine Web, Internacionalização e Gerador de Banners

### 6.1 Roteador Estático Cliente (`site/index.html`) para `site_type: doc`
A página raiz atua como roteador leve para os 10 subdiretórios de idioma (`en`, `pt`, `es`, `fr`, `de`, `it`, `ru`, `zh`, `ja`, `ar`), respeitando `localStorage.getItem('doc_lang')`, `navigator.language` e o fallback primário do `.cccprc`:

```html
<script>
  (function() {
    var supported = ['en', 'pt', 'es', 'fr', 'de', 'it', 'ru', 'zh', 'ja', 'ar'];
    var saved = localStorage.getItem('doc_lang');
    if (saved && supported.indexOf(saved) !== -1) {
      window.location.replace(saved + '/');
      return;
    }
    var nav = (navigator.language || 'pt').toLowerCase().substring(0, 2);
    window.location.replace((supported.indexOf(nav) !== -1 ? nav : 'pt') + '/');
  })();
</script>
```

### 6.2 Geração Automatizada do Banner Minimalista (`cccp site banner`)
O banner `site/assets/og_preview.png` (`1200 × 630 px`, Safe Zone 4:3 de `720px–840px` × `500px`) é gerado deterministicamente a partir do `.gost/metadata.json`:
* Renderiza o fundo com `theme.bg_color` e detalhes de contraste com `theme.accent_color`.
* Posiciona o `icon` oficial e imprime exclusivamente a chamada curta **`{title} — {tagline}`** ($\le 65$ caracteres), sem textos secundários.

### 6.3 Template Estrutural do Rodapé Canônico em Duas Linhas
O perfil Kollontai estabelece o contrato de renderização do rodapé nas 10 línguas oficiais, parametrizando as variáveis de identidade (`{author_affirmation_i18n}`, `{author_name}`, `{author_url}`, `{country_i18n}`) que são preenchidas pelo perfil final:

```html
<footer class="gost-footer">
  <p>{author_affirmation_i18n}</p>
  <p>{title} v{version_full} • <a href="{author_url}" target="_blank" rel="noopener">{author_name}</a> • {country_i18n} • <a href="LICENSE">{license_spdx}</a></p>
</footer>
```

---

## 7. Catálogo Local de Descoberta (`~/src/catalog.json`)

Todo repositório inicializado ou sincronizado pelo `cccp` registra seus metadados extraídos do `.gost/metadata.json` no índice central da estação de trabalho (`~/src/catalog.json`):

```json
{
  "slug": "<slug>",
  "title": "<title>",
  "tagline": "<tagline>",
  "summary": "<summary>",
  "tags": ["<tag1>", "<tag2>", "<tag3>"],
  "license": "<spdx>",
  "site_type": "doc",
  "primary_forge": "<primary_forge_url>",
  "website": "<public_site_url>"
}
```

---

<!-- ===================================================================== -->
<!-- CAMADA 3: https://lilya.2lp.in/profile.md -->
<!-- ===================================================================== -->

# Perfil de Implementação GOST: Lilya

```yaml
---
id: lilya
name: "Perfil de Implementação Lilya"
extends: "https://kollontai.2lp.in/profile.md"
target_gost: "2.0.0"
maintainer: "Lumen Pink <hi@lumen.pink>"
canonical_url: "https://lilya.2lp.in/profile.md"
status: "Ativo / Produção"
---
```

> **Sobre este Perfil:**  
> O perfil **Lilya** (anteriormente denominado *Lumen*) é a especialização autoral, pessoal e de infraestrutura soberana do padrão **[GOST](https://gost.2lp.in)**.  
> Ele estende diretamente o perfil base **[Kollontai](https://kollontai.2lp.in)** (`extends: https://kollontai.2lp.in/profile.md`), herdando a autoridade do Maestro **`cccp`**, a matriz de toolchains homologadas e os protocolos de distribuição, aplicando precedência estrita (*deep override*) para a forja soberana em `gambi.2lp.in`, publicação na borda (`*.2lp.in`), catálogo de binários (`banca.2lp.in`), afirmação identitária transfeminista no rodapé e entrega contínua nos servidores `obizero`, `obione` e `obitwo`.

---

## 1. Identidade, Forja Soberana (`gambi.2lp.in`) e Espelhos

### 1.1 Parâmetros Canônicos do Maestro (`.cccprc`)
No perfil Lilya, o Maestro `cccp` opera com os seguintes parâmetros canônicos de infraestrutura:

```ini
agent_var_lang = portuguese

# Forja Primária Soberana (Forgejo em gambi.2lp.in)
primary_forge_type = forgejo
primary_forge_domain = gambi.2lp.in
primary_owner = lumen
primary_repo = <slug>
primary_ssh_url = git@gambi.2lp.in:lumen/<slug>.git

# Espelhos Passivos e Ingestão de Fronteira
mirrors = codeberg,github
mirror_owner = lumenpink
```

### 1.2 Topologia de Repositórios e Push Mirrors Duplos
1. **Forja Primária Soberana:** Hospedada em `https://gambi.2lp.in/lumen/<slug>` (acesso SSH via porta 22 padrão em `git@gambi.2lp.in:lumen/<slug>.git`).
2. **Push Mirrors Automatizados (`sync_on_commit: true`):** A instância `gambi.2lp.in` replica imediatamente todo push recebido na branch `main` (e tags/branches persistentes) para dois espelhos externos de vitrine e preservação:
   - **Espelho FOSS:** Codeberg (`https://codeberg.org/lumenpink/<slug>`)
   - **Espelho Comunitário:** GitHub (`https://github.com/lumenpink/<slug>`)
3. **Remotes Locais e Ingestão de PRs:**
   - O desenvolvedor e os agentes de IA executam `cccp push` exclusivamente para o `origin` (`git@gambi.2lp.in:lumen/<slug>.git`).
   - Os remotes `codeberg` e `github` são mantidos pelo `cccp` com `pushurl = no_push` e refspec `+refs/pull/*/head:refs/remotes/<remote>/pr/*` para ingestão segura na quarentena `inbox/*`.

### 1.3 Allowlist Estrita de Rede na Esteira (`ci.yml`)
Na auditoria da regra `GOST-10-CI-HERMETIC-SINGLE-FILE`, a esteira `.forgejo/workflows/ci.yml` nos runners do ecossistema Lilya **SÓ PODE** referenciar recursos e espelhos internos sob os domínios:
* **`gambi.2lp.in`** (Actions espelhadas internamente ou relativas sem prefixo de servidor).
* **`crates.2lp.in`** (Registry interno e espelho de pacotes).
* **`banca.2lp.in`** (Despacho de artefatos compilados na fila `release`).
* Chamadas a outros domínios de rede pública quebram o `cccp check --strict` imediatamente.

---

## 2. Rodapé Canônico em Duas Linhas (Afirmação Identitária & Soberania)

O perfil Lilya preenche o template estrutural de rodapé herdado do perfil Kollontai com a afirmação identitária transfeminista sul-americana nas 10 línguas oficiais da norma GOST.

### 2.1 Estrutura HTML Canônica
Todo aplicativo com interface gráfica, TUI/CLI com rodapé formatado, vitrine pública ou documentação estática (`./site`) **DEVE** renderizar as duas linhas rigorosamente nesta estrutura:

```html
<footer class="footer">
  <p>Orgulhosamente produzido por uma <a href="https://travesti.2lp.in/pt/" target="_blank" rel="noopener">travesti latina</a> 🏳️‍⚧️ :)</p>
  <p>{title} v{version_full} • por <a href="https://lumen.pink" target="_blank" rel="noopener">Lumen Pink</a> • Brasil 🇧🇷 • <a href="LICENSE">{license_spdx}</a></p>
</footer>
```

### 2.2 Regras Invioláveis de Renderização do Rodapé
1. **Intraduzibilidade Política do Termo `travesti`:** O substantivo *travesti* permanece rigorosamente sem tradução nos 10 idiomas oficiais, preservando sua singularidade histórica e geopolítica latino-americana. O adjetivo concorda gramaticalmente com o idioma ativo e aponta para `https://travesti.2lp.in/<lang>/`.
2. **Versão Short vs. Versão Long:**
   - Cabeçalhos, títulos de janela e badges no topo da página exibem estritamente a **Versão Short** (`v{MAJOR}.{MINOR}.{PATCH}`, truncada antes do `+`).
   - A Linha 2 do rodapé exibe obrigatoriamente a **Versão Long** (string integral do arquivo `VERSION`, incluindo metadados de build `+YYYYMMDD`, ex: `v0.1.0+20261026`).

### 2.3 Matriz Oficial de Tradução do Rodapé (10 Línguas)

| ISO | Linha 1 (Afirmação Identitária Transfeminista) | Linha 2 (Metadados de Soberania e Licença) |
|:---:|---|---|
| **`pt`** | Orgulhosamente produzido por uma [travesti latina](https://travesti.2lp.in/pt/) 🏳️‍⚧️ :) | {title} v{ver} • por [Lumen Pink](https://lumen.pink) • Brasil 🇧🇷 • [{lic}](LICENSE) |
| **`en`** | Proudly produced by a [Latin American travesti](https://travesti.2lp.in/en/) 🏳️‍⚧️ :) | {title} v{ver} • by [Lumen Pink](https://lumen.pink) • Brazil 🇧🇷 • [{lic}](LICENSE) |
| **`es`** | Orgullosamente producido por una [travesti latinoamericana](https://travesti.2lp.in/es/) 🏳️‍⚧️ :) | {title} v{ver} • por [Lumen Pink](https://lumen.pink) • Brasil 🇧🇷 • [{lic}](LICENSE) |
| **`fr`** | Fièrement produit par une [travesti latino-américaine](https://travesti.2lp.in/fr/) 🏳️‍⚧️ :) | {title} v{ver} • par [Lumen Pink](https://lumen.pink) • Brésil 🇧🇷 • [{lic}](LICENSE) |
| **`de`** | Mit Stolz entwickelt von einer [lateinamerikanischen Travesti](https://travesti.2lp.in/de/) 🏳️‍⚧️ :) | {title} v{ver} • von [Lumen Pink](https://lumen.pink) • Brasilien 🇧🇷 • [{lic}](LICENSE) |
| **`it`** | Orgogliosamente prodotto da una [travesti latinoamericana](https://travesti.2lp.in/it/) 🏳️‍⚧️ :) | {title} v{ver} • da [Lumen Pink](https://lumen.pink) • Brasile 🇧🇷 • [{lic}](LICENSE) |
| **`ru`** | С гордостью создано [латиноамериканской travesti](https://travesti.2lp.in/ru/) 🏳️‍⚧️ :) | {title} v{ver} • от [Lumen Pink](https://lumen.pink) • Бразилия 🇧🇷 • [{lic}](LICENSE) |
| **`zh`** | 由一位[拉美 travesti](https://travesti.2lp.in/zh/) 自豪制作 🏳️‍⚧️ :) | {title} v{ver} • 来自 [Lumen Pink](https://lumen.pink) • 巴西 🇧🇷 • [{lic}](LICENSE) |
| **`ja`** | [ラテンアメリカの travesti](https://travesti.2lp.in/ja/) によって誇りを持って制作されました 🏳️‍⚧️ :) | {title} v{ver} • 制作: [Lumen Pink](https://lumen.pink) • ブラジル 🇧🇷 • [{lic}](LICENSE) |
| **`ar`** | صُنع بكل فخر بواسطة [travesti أمريكية لاتينية](https://travesti.2lp.in/ar/) 🏳️‍⚧️ :) | {title} v{ver} • بواسطة [Lumen Pink](https://lumen.pink) • البرازيل 🇧🇷 • [{lic}](LICENSE) |

---

## 3. Publicação Web na Borda (`*.2lp.in`) e Empacotamento Dioxus

### 3.1 Hospedagem Estática via Cloudflare Workers (`wrangler.jsonc`)
Toda vitrine pública (`./site`) é publicada sob o domínio `<slug>.2lp.in` através de `wrangler.jsonc` na raiz:

```jsonc
{
  "$schema": "node_modules/wrangler/config-schema.json",
  "name": "<slug>",
  "compatibility_date": "2026-01-01",
  "routes": [
    {
      "pattern": "<slug>.2lp.in",
      "custom_domain": true
    }
  ],
  "assets": {
    "directory": "./site"
  }
}
```

* **Redundância Estática:** A fila `deploy` da esteira publica simultaneamente o snapshot em branch órfã stateless `pages` para o Codeberg Pages (`https://lumenpink.codeberg.page/<slug>/`).

### 3.2 Identificador de Bundle Canônico (`Dioxus.toml`)
Para aplicações gráficas e mobile construídas com Dioxus 0.8, o namespace reverso e o publisher são fixados como:

```toml
[application]
name = "<slug>"
default_platform = "web"

[bundle]
identifier = "in.2lp.<slug>"
publisher = "Lumen Pink"
icon = ["site/assets/icon.png"]
```

---

## 4. Distribuição de Binários na Banca (`banca.2lp.in`)

A autoridade central de distribuição de executáveis e pacotes definida na Seção 4 do perfil Kollontai reside em **`https://banca.2lp.in`** (servida via Cloudflare R2):

* **Vitrine Interativa:** `https://banca.2lp.in/downloads/#<slug>` (design brutalista/cyberpunk em Vanilla HTML/CSS/JS, sem frameworks pesados, com cópia rápida de comando `curl` e tabela de hashes SHA-256).
* **Comando Universal de Instalação:**
  ```sh
  curl -sSf https://banca.2lp.in/downloads/<slug>/install.sh | sh
  ```
* **Endpoint de Auto-Update:** Os binários CLI consultam `https://banca.2lp.in/downloads/<slug>/<channel>-latest.version` e `index.json` para o protocolo XDG de atualização (`<slug> check-update` / `<slug> update`).
* **Retenção de Artefatos:** Política estrita de retenção de 5 estáveis e 10 compilações noturnas no Cloudflare R2.

---

## 5. Entrega Contínua de Servidores (CD em `obizero`, `obione` e `obitwo`)

Para serviços conteinerizados autohospedados, o perfil Lilya opera com **Continuous Deployment (CD) Determinístico** disparado diretamente pela fila `deploy` da esteira em `gambi.2lp.in`, com banimento total do Watchtower.

### 5.1 Padrão de Orquestração e Healthcheck Obrigatório
Todos os serviços residem em `~/stacks/<slug>/compose.yaml` (estritamente com extensão `.yaml`, sendo banida a extensão `.yml`) e declaram probe ativa de saúde:

```yaml
services:
  app:
    image: lumenpink/<slug>:${APP_VERSION:-latest}
    user: "1000:1000"
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "curl -fsS http://127.0.0.1:<port>/healthz || exit 1"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 10s
```

### 5.2 Topologia de Deploy por Nó de Infraestrutura

#### 1. Nó Central de Execução (`obizero` — Deploy Local via Socket)
* O `forgejo-runner` executa no host `obizero` com o socket Docker montado.
* A imagem recém-compilada na fila `build` já reside no daemon local (`lumenpink/<slug>:<version>`).
* O deploy ocorre sem tráfego externo redundante:
  ```sh
  docker compose -f /home/lumen/stacks/<slug>/compose.yaml up -d --no-build <slug>
  ```
* **Verificação Síncrona de Saúde e Rollback Automático (15s):**
  A esteira monitora o status do container por até 15 segundos. Caso o estado não atinja `healthy` (ou `running` estável), a esteira captura `docker logs --tail 50 <slug>`, reverte imediatamente para a tag estável anterior, executa `docker image prune -f` e falha o pipeline com código não-zero.

#### 2. Nós de Borda Remotos (`obione` e `obitwo` — Deploy via Mesh VPN Tailscale)
* Para serviços distribuídos nos nós de borda (acessíveis via rede privada WireGuard/Tailscale ou expostos via Cloudflare Tunnel `cloudflared` em `app.<slug>.2lp.in`):
* A esteira publica a imagem e aciona a atualização remotamente via SSH sobre a malha Tailscale:
  ```sh
  ssh -o StrictHostKeyChecking=accept-new lumen@<no-tailscale> \
    "docker compose -f ~/stacks/<slug>/compose.yaml pull && \
     docker compose -f ~/stacks/<slug>/compose.yaml up -d && \
     docker image prune -f"
  ```

---

## 6. Sobreposição de Regras de Conformidade (`lilya_rules.json`)

Quando o `cccp sync` compila as regras para um repositório sob o perfil Lilya, as seguintes parametrizações específicas sobrepõem (*deep merge*) o `rules.json` base da GOST:

```json
{
  "extends": "GOST-2.0.0",
  "profile": "lilya",
  "rules_override": [
    {
      "id": "GOST-09-SITE-SECURITY-HEADERS",
      "require_footer_links": [
        "https://travesti.2lp.in/",
        "https://lumen.pink"
      ]
    },
    {
      "id": "GOST-10-CI-HERMETIC-SINGLE-FILE",
      "allowed_uses_domains": ["gambi.2lp.in"],
      "allowed_run_domains": ["gambi.2lp.in", "crates.2lp.in", "banca.2lp.in"]
    },
    {
      "id": "GOST-25-REMOTES-AND-INBOX-QUARANTINE",
      "required_origin_pattern": "^git@gambi\\.2lp\\.in:lumen/[a-z0-9-]+\\.git$",
      "required_mirrors": {
        "codeberg": "https://codeberg.org/lumenpink/<slug>.git",
        "github": "https://github.com/lumenpink/<slug>.git"
      }
    }
  ]
}
```

---

