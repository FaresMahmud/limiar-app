-- Migration 2: Criação das tabelas de experimentos, grupos, animais e timepoints
CREATE TABLE experimentos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nome TEXT NOT NULL,
    descricao TEXT,
    conjunto_id INTEGER NOT NULL,
    responsavel TEXT,
    ativo INTEGER NOT NULL DEFAULT 1, -- 1 = ativo, 0 = excluído (soft-delete)
    criado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
    atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(conjunto_id) REFERENCES conjuntos_filamentos(id)
);

CREATE TABLE grupos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    experimento_id INTEGER NOT NULL,
    nome TEXT NOT NULL,
    cor TEXT NOT NULL, -- Cor em hexadecimal ou string HTML
    ativo INTEGER NOT NULL DEFAULT 1, -- 1 = ativo, 0 = excluído (soft-delete)
    criado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
    atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(experimento_id) REFERENCES experimentos(id) ON DELETE CASCADE
);

CREATE TABLE animais (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    experimento_id INTEGER NOT NULL,
    grupo_id INTEGER NOT NULL,
    marcacao TEXT NOT NULL, -- ex: "4P", "2L"
    peso REAL, -- peso em gramas (opcional)
    ativo INTEGER NOT NULL DEFAULT 1, -- 1 = ativo, 0 = excluído (soft-delete)
    criado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
    atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(experimento_id) REFERENCES experimentos(id) ON DELETE CASCADE,
    FOREIGN KEY(grupo_id) REFERENCES grupos(id) ON DELETE CASCADE
);

CREATE TABLE timepoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    experimento_id INTEGER NOT NULL,
    rotulo TEXT NOT NULL, -- ex: "basal 1", "indução", "1h"
    ordem INTEGER NOT NULL,
    opcional INTEGER NOT NULL DEFAULT 0, -- 0 = obrigatório, 1 = opcional (8h/24h)
    criado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
    atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(experimento_id) REFERENCES experimentos(id) ON DELETE CASCADE
);
