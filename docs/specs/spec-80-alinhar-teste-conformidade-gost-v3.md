# Especificação do Ticket #80: Alinhar Teste de Conformidade com Padrão GOST v3

## 1. Contexto e Motivação
Após a purga de artefatos obsoletos do GOST v2 em `lacraia`, o manifesto `GOSTVER` foi definitivamente expurgado em favor de `.gost/GOST.md` e `.gost/rules.toml`.
Entretanto, o teste `packages/lacraia-shared/tests/spec_gost_conformance_test.rs` ainda tentava abrir e inspecionar o arquivo `GOSTVER`, resultando em pânico de I/O (`Os { code: 2, kind: NotFound }`) na esteira hermética de CI.

## 2. Requisitos da Correção
1. Atualizar `packages/lacraia-shared/tests/spec_gost_conformance_test.rs`:
   - Validar que `GOSTVER` foi purgado com sucesso (`assert!(!root.join("GOSTVER").exists())`).
   - Validar que `.gost/GOST.md` e `.gost/rules.toml` existem e contêm os contratos canônicos da metanorma GOST v3 e regras declarativas.
   - Preservar as validações de `SPEC.md` (9 seções essenciais, ausência de termos imperialistas, autoria e licença).
2. Atualizar a regra `KOLLONTAI-018` em `.gost/rules.toml` com o bypass determinístico para esteiras de CI herméticas.
3. Testes do workspace `cargo test -p lacraia-shared --test spec_gost_conformance_test` devem passar com 100% de sucesso.

## 3. Árvore de Decisões e Registro Arquitetural (para a Natasha)
1. **Validação de Conformidade sem `GOSTVER`:**
   - *Decisão:* A integridade normativa agora é aferida pelo livro compilado `.gost/GOST.md` e o catálogo `.gost/rules.toml`, enquanto o teste de conformidade garante que regressões não reintroduzam arquivos legados.
