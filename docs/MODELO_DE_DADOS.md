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
| criado_em    | TEXT    | ISO 8601 |

### 2.2 Filamento
Um filamento individual do kit, com sua força calibrada.

| Campo         | Tipo    | Notas |
|---------------|---------|-------|
| id            | INTEGER | PK |
| conjunto_id   | INTEGER | FK → ConjuntoDeFilamentos |
| forca_g       | REAL    | força de dobra em **gramas** |
| rotulo        | TEXT    | opcional (ex.: número/etiqueta do filamento) |
| ordem         | INTEGER | posição na escala crescente |

> `d` do conjunto é **derivado** destes filamentos (média das diferenças de
> `log10(forca_g)`), **não armazenado como constante fixa**. Ver [DOMINIO.md](DOMINIO.md) §3.

### 2.3 Experimento
Um estudo/ensaio completo.

| Campo               | Tipo    | Notas |
|---------------------|---------|-------|
| id                  | INTEGER | PK |
| nome                | TEXT    | |
| descricao           | TEXT    | opcional |
| conjunto_id         | INTEGER | FK → ConjuntoDeFilamentos (kit usado) |
| responsavel         | TEXT    | pesquisador |
| criado_em           | TEXT    | ISO 8601 |

### 2.4 Grupo
Grupo de tratamento dentro de um experimento (ex.: controle, tratado). Tem uma
**cor** (parte da identificação visual do animal).

| Campo         | Tipo    | Notas |
|---------------|---------|-------|
| id            | INTEGER | PK |
| experimento_id| INTEGER | FK → Experimento |
| nome          | TEXT    | ex.: "Controle", "Morfina 5mg/kg" |
| cor           | TEXT    | cor do grupo (hex ou nome) — identidade visual |

### 2.5 Animal
Um roedor. Identificado por marcação física (texto livre) + cor do grupo.

| Campo         | Tipo    | Notas |
|---------------|---------|-------|
| id            | INTEGER | PK |
| experimento_id| INTEGER | FK → Experimento |
| grupo_id      | INTEGER | FK → Grupo (pode ser definido após randomização) |
| marcacao      | TEXT    | ex.: "4P", "2L" — riscos na cauda (só rótulo, sem lógica) |
| especie       | TEXT    | opcional (rato/camundongo) |
| observacoes   | TEXT    | opcional |

> A **randomização/balanceamento** por limiar basal atribui `grupo_id` de forma
> a minimizar o desvio entre grupos (ver [DOMINIO.md](DOMINIO.md) §5.2). É um
> processo, não um campo.

### 2.6 Timepoint
Momento da curva temporal do experimento (basal, indução, 1h, 2h...).

| Campo         | Tipo    | Notas |
|---------------|---------|-------|
| id            | INTEGER | PK |
| experimento_id| INTEGER | FK → Experimento |
| rotulo        | TEXT    | ex.: "basal 1", "tratamento", "2h", "24h" |
| ordem         | INTEGER | posição na curva temporal |
| opcional      | INTEGER | 0/1 — pontos como 8h/24h podem não ser executados |

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
- Estratégia de soft-delete vs. hard-delete para dados de laboratório.
