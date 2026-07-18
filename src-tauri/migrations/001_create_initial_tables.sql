-- Migration 1: Criação das tabelas de conjuntos de filamentos
CREATE TABLE conjuntos_filamentos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    nome TEXT NOT NULL,
    descricao TEXT,
    d REAL NOT NULL,
    ativo INTEGER NOT NULL DEFAULT 1, -- 1 = ativo, 0 = excluído (soft-delete)
    criado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
    atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE filamentos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    conjunto_id INTEGER NOT NULL,
    forca_g REAL NOT NULL,
    ordem INTEGER NOT NULL,
    FOREIGN KEY(conjunto_id) REFERENCES conjuntos_filamentos(id) ON DELETE CASCADE
);
