-- Migration 3: Criação das tabelas de sequências de testes e respostas
CREATE TABLE sequencias_teste (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    animal_id INTEGER NOT NULL,
    timepoint_id INTEGER NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('em_andamento', 'concluida')),
    filamento_inicial REAL NOT NULL,
    limiar REAL, -- Nulo até finalizar
    estimativa_log REAL,
    k_dixon REAL,
    d_usado REAL,
    n_nominal INTEGER,
    criado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
    atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(animal_id) REFERENCES animais(id) ON DELETE CASCADE,
    FOREIGN KEY(timepoint_id) REFERENCES timepoints(id) ON DELETE CASCADE
);

-- Um animal não pode ter duas sequências em_andamento simultâneas para o mesmo timepoint
CREATE UNIQUE INDEX idx_sequencias_em_andamento ON sequencias_teste(animal_id, timepoint_id) WHERE status = 'em_andamento';

CREATE TABLE respostas_sequencia (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sequencia_id INTEGER NOT NULL,
    ordem INTEGER NOT NULL,
    filamento_g REAL NOT NULL,
    resposta TEXT NOT NULL CHECK(resposta IN ('O', 'X')),
    criado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(sequencia_id) REFERENCES sequencias_teste(id) ON DELETE CASCADE,
    UNIQUE(sequencia_id, ordem)
);
