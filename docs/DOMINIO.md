# Domínio científico — von Frey + método up-and-down de Dixon

> Objetivo deste documento: permitir que qualquer pessoa (humana ou IA) entenda
> **o que o software calcula e por quê**, sem precisar reler o artigo original.
> Referência: Dixon, W.J. (1980). *Efficient Analysis of Experimental
> Observations*. Annual Review of Pharmacology and Toxicology, 20:441-462.

---

## 1. O teste de von Frey (contexto biológico)

Mede-se a **sensibilidade mecânica** (nocicepção / dor) da pata de um roedor
(rato ou camundongo). Usa-se um conjunto de **filamentos de nylon calibrados**
(os "filamentos de von Frey"), cada um com uma **força de dobra** conhecida,
tipicamente expressa em **gramas** (g). Os filamentos formam uma escala
**crescente de força**, com espaçamento aproximadamente **logarítmico**.

Aplica-se um filamento perpendicularmente à superfície plantar da pata:

- Se o animal **NÃO retira a pata** → estímulo fraco demais → sobe para o
  **próximo filamento mais forte**. Registramos **`O`** (do inglês *no response*,
  aqui usado como "sem retirada").
- Se o animal **retira a pata** → estímulo suficiente → desce para o **próximo
  filamento mais fraco**. Registramos **`X`** (resposta / retirada).

Esse vai-e-volta em torno do limiar é o **método "up-and-down"** (sobe-e-desce).

> ⚠️ Convenção de símbolos: neste projeto **`X` = o animal RESPONDEU (retirou a
> pata)** e **`O` = NÃO respondeu**. A tabela de Dixon é indexada por essa
> sequência de O/X. Manter esta convenção consistente em todo o código, banco e UI.

---

## 2. O que é o "limiar" (PWT)

O resultado do teste é o **Limiar de Retirada da Pata** — *Paw Withdrawal
Threshold* (**PWT**) — a força (em g) na qual se estima que o animal passaria a
responder 50% das vezes. É o desfecho que o laboratório registra e compara entre
grupos e ao longo do tempo.

O método de Dixon estima esse limiar com **poucos animais/aplicações** (eficiente),
usando a sequência de respostas em torno do ponto de viragem.

---

## 3. A fórmula do limiar

Ao final de uma sequência (nominalmente **N = 6** testes, conforme a tabela de
Dixon), calcula-se:

```
LIMIAR = 10 ^ ( log10(último_filamento_testado) + k_dixon × d )
```

Onde:

- **`último_filamento_testado`** — a força (g) do último filamento aplicado na
  sequência. O cálculo é feito no espaço **log10** porque a escala de filamentos
  é logarítmica.

- **`k_dixon`** — valor tabelado de **máxima verossimilhança** (Dixon 1980,
  **Tabela 7**). É indexado pela **sequência exata de O/X** do animal naquela
  série. É um número (positivo ou negativo) que ajusta o último filamento para
  cima ou para baixo conforme o padrão de respostas.

- **`d`** — o **passo médio** entre filamentos no espaço log10: a **média das
  diferenças entre os `log10` das forças dos filamentos** do conjunto usado.
  **`d` NÃO é uma constante do código** — depende do kit de filamentos de cada
  laboratório e deve ser **calculado automaticamente a partir do cadastro dos
  filamentos** (ver [MODELO_DE_DADOS.md](MODELO_DE_DADOS.md)).

### Como calcular `d`

Dado o conjunto de filamentos do laboratório com forças `f_1 < f_2 < ... < f_n`
(em g), calcula-se em log10 e tira-se a média das diferenças consecutivas:

```
d = média( log10(f_{i+1}) - log10(f_i) )  para i = 1..n-1
```

Se o espaçamento for perfeitamente uniforme em log (kit ideal), `d` é
simplesmente essa diferença constante. Como kits reais têm pequenas variações,
usamos a **média**. (Este é um ponto a confirmar com o laboratório ao implementar:
alguns protocolos usam um `d` fixo do kit; documentar a decisão final aqui.)

---

## 4. A Tabela 7 de Dixon (CRÍTICA — ainda não implementada)

O `k_dixon` vem de uma **tabela de consulta**: para cada **padrão de respostas
O/X** de uma sequência, há um valor `k` correspondente (derivado por máxima
verossimilhança no artigo de Dixon).

### Por que isso é o coração do projeto

O software antigo do laboratório **trava** exatamente porque a tabela embutida
nele está **incompleta** — só cobre sequências de até ~4 respostas. Sequências
com **4+ respostas iguais** caem fora da tabela e o programa quebra.

### Requisito

Implementar a **Tabela 7 completa**: **N de 2 a 6**, **todas as combinações
possíveis de O/X**. Os valores devem ser **transcritos exatamente do artigo
original** (o usuário fornecerá o PDF). 

> 🚫 **NÃO inventar, interpolar ou "chutar" valores de `k`.** Erros aqui produzem
> limiares cientificamente inválidos silenciosamente. Toda a tabela deve vir do
> artigo, idealmente com um teste automatizado que fixe cada valor conhecido.

### Placeholder (estado atual)

```
// TODO(etapa 1): transcrever Tabela 7 de Dixon (1980), N=2..6, todas as
// combinações O/X, a partir do PDF fornecido pelo usuário.
// Estrutura sugerida: mapa { "OXOXO": k, ... } indexado pela string da sequência.
// Ainda NÃO implementado — ver docs/ROADMAP.md etapa 1.
```

---

## 5. Outros conceitos de negócio

### 5.1 Identificação do animal
Cada animal recebe uma **marcação física** na cauda (riscos) combinada com uma
**cor de grupo**. Exemplos de notação: **"4P"** = 4 riscos *próximos* à base da
cauda; **"2L"** = 2 riscos *longe* da base. É apenas um **identificador**
(texto livre + cor) — **não há lógica matemática associada**. No modelo de dados
é um rótulo, não um campo calculado.

### 5.2 Randomização / balanceamento de grupos
Os animais são distribuídos em **grupos de tratamento de forma balanceada com
base no limiar basal** (resposta inicial), **não sequencialmente**. O objetivo é
**reduzir o desvio padrão entre grupos** (grupos comparáveis no ponto de partida).
Implementação futura: um algoritmo que reparte os animais minimizando a diferença
de médias/variância dos limiares basais entre grupos.

### 5.3 Curva temporal (timepoints)
Um experimento acompanha cada animal ao longo de **timepoints**, por exemplo:

```
basal 1 → indução → basal 2 → tratamento → 0.5h → 1h → 2h → 4h → 6h → (8h/24h)
```

Os últimos pontos (8h, 24h) podem ou não ser executados **dependendo do
resultado** dos anteriores. **Cada timepoint gera uma nova sequência de teste**
(nova série O/X) e, portanto, **um novo limiar** por animal.

### 5.4 A necessidade central: decisão em tempo real
O motivo de existir do software: o **limiar precisa ser calculado na hora**, no
laboratório, logo após a sequência. Isso permite decidir **na bancada** se ainda
há efeito do tratamento e se vale **estender a curva temporal** (ir até 8h/24h) ou
parar. É o principal ganho sobre o fluxo atual (anotar à mão → digitar no Excel à
noite), que só revela os resultados horas depois, tarde demais para ajustar o
experimento.

---

## 6. Glossário rápido

| Termo | Significado |
|-------|-------------|
| **von Frey** | Filamentos de nylon calibrados para medir sensibilidade mecânica. |
| **PWT / Limiar** | Paw Withdrawal Threshold — força (g) estimada de retirada da pata. |
| **up-and-down** | Método de subir/descer filamentos em torno do limiar (Dixon). |
| **O** | Animal NÃO retirou a pata (sem resposta). |
| **X** | Animal retirou a pata (respondeu). |
| **`k` / k_dixon** | Coeficiente tabelado (Dixon Tabela 7) para a sequência O/X. |
| **`d`** | Passo médio entre filamentos em log10 (depende do kit do laboratório). |
| **timepoint** | Momento da curva temporal em que se refaz o teste (basal, 1h, ...). |
| **sequência O/X** | Série de respostas de um animal num timepoint; entrada da tabela. |
