# Modelo de dados (rascunho inicial)

> ⚠️ **Rascunho.** Este documento é um ponto de partida e **deve evoluir** conforme
> a implementação avança. Nada aqui é definitivo — nomes de campos, tipos e
> relações serão refinados nas etapas 2–5 do [ROADMAP.md](ROADMAP.md). O schema
> real do SQLite será criado por **migrations** (ver [ARQUITETURA.md](ARQUITETURA.md) §3).

Convenções deste rascunho:
- Nomes de entidades em pt-BR (domínio científico brasileiro).
- Toda tabela tem `id` (inteiro, PK) e, quando útil, `criado_em`/`atualizado_em`.
- "O/X": `O` = sem retirada, `X` = retirou a pata (ver [DOMINIO.md](DOMINIO.md)).

---

## 1. Visão geral das entidades e relações

```
ConjuntoDeFilamentos 1 ──── N Filamento
        │ (usado por)
        ▼
   Experimento 1 ──── N Grupo 1 ──── N Animal
        │                                  │
        │ 1                                │ (testado em cada)
        ▼ N                                │
    Timepoint  ────────────────────────────┘
        │ 1
        ▼ N
  SequenciaDeTeste 1 ──── 1 Limiar
        (uma série O/X por animal por timepoint → um limiar)
```

Leitura: um **Experimento** usa **um ConjuntoDeFilamentos**, tem vários **Grupos**,
cada Grupo tem vários **Animais**; o Experimento define vários **Timepoints**; para
cada (Animal × Timepoint) existe uma **SequenciaDeTeste** que produz um **Limiar**.

---

## 2. Entidades

### 2.1 ConjuntoDeFilamentos
Representa o **kit de filamentos de von Frey** de um laboratório. Existe porque
cada laboratório pode ter um kit diferente, o que muda o cálculo de `d`.

| Campo        | Tipo    | Notas |
|--------------|---------|-------|
| id           | INTEGER | PK |
| nome         | TEXT    | ex.: "Kit padrão camundongo" |
| descricao    | TEXT    | opcional |
| d            | REAL    | valor d calculado a partir dos filamentos |
| ativo        | INTEGER | 1 = ativo, 0 = excluído (soft-delete) |
| criado_em    | TEXT    | ISO 8601 (CURRENT_TIMESTAMP) |
| atualizado_em| TEXT    | ISO 8601 (CURRENT_TIMESTAMP) |

### 2.2 Filamento
Um filamento individual do kit, com sua força calibrada.

| Campo         | Tipo    | Notas |
|---------------|---------|-------|
| id            | INTEGER | PK |
| conjunto_id   | INTEGER | FK → ConjuntoDeFilamentos |
| forca_g       | REAL    | força de dobra em **gramas** |
| ordem         | INTEGER | posição na escala crescente (0-indexada) |

> `d` do conjunto é calculado no Rust a partir dos filamentos (média das diferenças de `log10(forca_g)`) e armazenado na tabela `conjuntos_filamentos` para garantir performance de consulta e rastreabilidade científica. Ver [DOMINIO.md](DOMINIO.md) §3.

### 2.3 Experimento
Um estudo/ensaio completo.

| Campo               | Tipo    | Notas |
|---------------------|---------|-------|
| id                  | INTEGER | PK |
| nome                | TEXT    | |
| descricao           | TEXT    | opcional |
| conjunto_id         | INTEGER | FK → conjuntos_filamentos (kit usado) |
| responsavel         | TEXT    | pesquisador (opcional) |
| ativo               | INTEGER | 1 = ativo, 0 = excluído (soft-delete) |
| criado_em           | TEXT    | ISO 8601 (CURRENT_TIMESTAMP) |
| atualizado_em       | TEXT    | ISO 8601 (CURRENT_TIMESTAMP) |

### 2.4 Grupo
Grupo de tratamento dentro de um experimento (ex.: controle, tratado). Tem uma
**cor** (parte da identificação visual do animal).

| Campo         | Tipo    | Notas |
|---------------|---------|-------|
| id            | INTEGER | PK |
| experimento_id| INTEGER | FK → Experimento |
| nome          | TEXT    | ex.: "Controle", "Morfina 5mg/kg" |
| cor           | TEXT    | cor do grupo (hex ou nome) — identidade visual |
| ativo         | INTEGER | 1 = ativo, 0 = excluído (soft-delete) |
| criado_em     | TEXT    | ISO 8601 (CURRENT_TIMESTAMP) |
| atualizado_em | TEXT    | ISO 8601 (CURRENT_TIMESTAMP) |

### 2.5 Animal
Um roedor. Identificado por marcação física (texto livre) + cor do grupo.

| Campo         | Tipo    | Notas |
|---------------|---------|-------|
| id            | INTEGER | PK |
| experimento_id| INTEGER | FK → Experimento |
| grupo_id      | INTEGER | FK → Grupo |
| marcacao      | TEXT    | ex.: "4P", "2L" — riscos na cauda (rótulo visual) |
| peso          | REAL    | peso do animal em gramas (opcional) |
| ativo         | INTEGER | 1 = ativo, 0 = excluído (soft-delete) |
| criado_em     | TEXT    | ISO 8601 (CURRENT_TIMESTAMP) |
| atualizado_em | TEXT    | ISO 8601 (CURRENT_TIMESTAMP) |

### 2.6 Timepoint
Momento da curva temporal do experimento (basal, indução, 1h, 2h...).

| Campo         | Tipo    | Notas |
|---------------|---------|-------|
| id            | INTEGER | PK |
| experimento_id| INTEGER | FK → Experimento |
| rotulo        | TEXT    | ex.: "basal 1", "tratamento", "2h", "24h" |
| ordem         | INTEGER | posição na curva temporal (0-indexada) |
| opcional      | INTEGER | 0 = obrigatório, 1 = opcional |
| criado_em     | TEXT    | ISO 8601 (CURRENT_TIMESTAMP) |
| atualizado_em | TEXT    | ISO 8601 (CURRENT_TIMESTAMP) |

### 2.7 SequenciaDeTeste
Uma série up-and-down de um **animal** num **timepoint**: a sequência O/X e os
filamentos aplicados. Nominalmente N=6 aplicações.

| Campo            | Tipo    | Notas |
|------------------|---------|-------|
| id               | INTEGER | PK |
| animal_id        | INTEGER | FK → Animal |
| timepoint_id     | INTEGER | FK → Timepoint |
| sequencia_ox     | TEXT    | ex.: "OXXOX" (ver convenção O/X) |
| filamentos_ids   | TEXT    | lista ordenada dos Filamento.id aplicados (JSON) |
| ultimo_filamento_g | REAL  | força do último filamento (entra na fórmula) |
| criado_em        | TEXT    | ISO 8601 (carimbo do momento do teste) |

### 2.8 Limiar
O resultado calculado para uma SequenciaDeTeste (PWT). Guardado para histórico e
comparação — e recalculável a partir da sequência + filamentos.

| Campo             | Tipo    | Notas |
|-------------------|---------|-------|
| id                | INTEGER | PK |
| sequencia_id      | INTEGER | FK → SequenciaDeTeste (1:1) |
| valor_g           | REAL    | limiar em gramas (resultado da fórmula) |
| k_dixon_usado     | REAL    | `k` da Tabela 7 aplicado (rastreabilidade) |
| d_usado           | REAL    | `d` aplicado (rastreabilidade) |
| calculado_em      | TEXT    | ISO 8601 |

> Guardar `k_dixon_usado` e `d_usado` dá **rastreabilidade científica**: permite
> auditar como cada limiar foi obtido, mesmo que a tabela/kit mudem depois.

---

## 3. Questões em aberto (a resolver na implementação)

- Um Animal pertence a exatamente um Experimento, ou pode reaparecer em vários?
  (rascunho assume 1 experimento).
- `sequencia_ox` e `filamentos_ids`: normalizar em tabela filha
  (uma linha por aplicação) vs. guardar como string/JSON? (rascunho usa string/JSON
  pela simplicidade; revisar se precisar consultar aplicação a aplicação).
- Índices únicos: provavelmente `(animal_id, timepoint_id)` deve ser único em
  SequenciaDeTeste.

---

## 4. Decisões tomadas na Etapa 2

- **Estratégia de Exclusão (Soft-Delete)**: Para `ConjuntoDeFilamentos`, foi adotado soft-delete via coluna `ativo` (1 = ativo, 0 = inativo). Isso previne a exclusão física e garante que experimentos e limiares já computados no passado mantenham sua rastreabilidade e integridade referencial ao seu conjunto de origem.
- **Relacionamento da Tabela de Filamentos**: A tabela `filamentos` armazena os valores em gramas com ordem (escala crescente 0-indexada) associados via FK `conjunto_id` à tabela principal. A edição de um conjunto limpa e reinserte os filamentos em uma transação ACID local.
- **Armazenamento de `d`**: O valor `d` é calculado no Rust durante a gravação/atualização do conjunto e persistido na coluna `d` de `conjuntos_filamentos`. Isso agiliza as leituras e previne descalibrações retroativas nos históricos de limiares se as definições da escala mudarem futuramente.

---

## 5. Decisões tomadas na Etapa 3

- **Modelo Temporal (Timepoints)**: Os timepoints são armazenados em uma tabela independente vinculada por FK a cada experimento. Isso suporta a criação de curvas temporais totalmente dinâmicas para cada estudo, respeitando a ordem cronológica desejada pelos pesquisadores.
- **Estratégia de Exclusão (Soft-Delete) em Cascata**:
  - Para Experimentos, Grupos e Animais foi adotada a exclusão lógica via coluna `ativo` (1 = ativo, 0 = inativo) para prevenir a exclusão acidental de dados nociceptivos históricos por engano do usuário.
  - Para a consistência do banco, na deleção física ou limpeza (expurgação), foi configurada a restrição `ON DELETE CASCADE` no SQLite. Se um experimento for removido fisicamente do banco de dados, todas as suas tabelas filhas (grupos, animais e timepoints) são excluídas em cascata para não deixar órfãos óbvios.
  - Se um grupo de tratamento for marcado como inativo (`ativo = 0`), todos os seus animais correspondentes também recebem a alteração de `ativo = 0` recursivamente no comando de exclusão do grupo.

