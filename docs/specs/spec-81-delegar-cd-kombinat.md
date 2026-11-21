# Especificação do Ticket #81: Delegar Entregas e Reset de Container do La Craia ao Kombinat

## 1. Contexto e Motivação
O workflow `.forgejo/workflows/ci.yml` executava uploads diretos para R2, rotação manual na Banca, trigger direto na Pratilera e SSH direto em `obizero`/`obione`. Com o Kombinat, a CI publica artefatos via `cccp package upload` nos Generic Packages e mantém apenas build/push da imagem Docker para a Gambi.
