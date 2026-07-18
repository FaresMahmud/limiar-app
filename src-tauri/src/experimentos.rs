use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExperimentoCompletoDto {
    pub id: i64,
    pub nome: String,
    pub descricao: Option<String>,
    pub conjunto_id: i64,
    pub conjunto_nome: String,
    pub responsavel: Option<String>,
    pub criado_em: String,
    pub atualizado_em: String,
    pub timepoints: Vec<TimepointDto>,
    pub grupos: Vec<GrupoCompletoDto>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimepointDto {
    pub id: i64,
    pub experimento_id: i64,
    pub rotulo: String,
    pub ordem: i64,
    pub opcional: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrupoDto {
    pub id: i64,
    pub experimento_id: i64,
    pub nome: String,
    pub cor: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrupoCompletoDto {
    pub id: i64,
    pub experimento_id: i64,
    pub nome: String,
    pub cor: String,
    pub animais: Vec<AnimalDto>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnimalDto {
    pub id: i64,
    pub experimento_id: i64,
    pub grupo_id: i64,
    pub marcacao: String,
    pub peso: Option<f64>,
}

/// Abre uma conexão com o banco de dados SQLite local.
fn obter_conexao(app_handle: &tauri::AppHandle) -> Result<Connection, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Falha ao obter diretório de dados do app: {}", e))?;
    
    std::fs::create_dir_all(&app_dir)
        .map_err(|e| format!("Falha ao criar diretório de dados do app: {}", e))?;
        
    let db_path = app_dir.join("limiar.db");
    
    Connection::open(db_path)
        .map_err(|e| format!("Falha ao conectar ao banco de dados: {}", e))
}

// =============================================================================
// CRUD: EXPERIMENTOS
// =============================================================================

/// Comando Tauri: Cria um novo experimento com seus timepoints associados.
#[tauri::command]
pub fn criar_experimento(
    app_handle: tauri::AppHandle,
    nome: String,
    descricao: Option<String>,
    conjunto_id: i64,
    responsavel: Option<String>,
    timepoints: Vec<String>,
) -> Result<ExperimentoCompletoDto, String> {
    if nome.trim().is_empty() {
        return Err("O nome do experimento não pode ser vazio.".to_string());
    }

    let mut conn = obter_conexao(&app_handle)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Falha ao iniciar transação: {}", e))?;

    // Inserir experimento
    tx.execute(
        "INSERT INTO experimentos (nome, descricao, conjunto_id, responsavel, ativo) VALUES (?1, ?2, ?3, ?4, 1)",
        params![nome, descricao, conjunto_id, responsavel],
    )
    .map_err(|e| format!("Falha ao salvar experimento: {}", e))?;

    let experimento_id = tx.last_insert_rowid();

    // Inserir timepoints
    for (i, rotulo) in timepoints.iter().enumerate() {
        if rotulo.trim().is_empty() {
            return Err("O rótulo do timepoint não pode ser vazio.".to_string());
        }
        tx.execute(
            "INSERT INTO timepoints (experimento_id, rotulo, ordem, opcional) VALUES (?1, ?2, ?3, 0)",
            params![experimento_id, rotulo.trim(), i as i64],
        )
        .map_err(|e| format!("Falha ao salvar timepoint '{}': {}", rotulo, e))?;
    }

    tx.commit()
        .map_err(|e| format!("Falha ao confirmar transação: {}", e))?;

    buscar_experimento_por_id(&conn, experimento_id)
}

/// Comando Tauri: Lista todos os experimentos ativos.
#[tauri::command]
pub fn listar_experimentos(app_handle: tauri::AppHandle) -> Result<Vec<ExperimentoCompletoDto>, String> {
    let conn = obter_conexao(&app_handle)?;
    
    let mut stmt = conn
        .prepare("SELECT id FROM experimentos WHERE ativo = 1 ORDER BY criado_em DESC")
        .map_err(|e| format!("Falha ao preparar consulta: {}", e))?;
        
    let ids_iter = stmt
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Falha ao listar experimentos: {}", e))?;

    let mut result = Vec::new();
    for id_res in ids_iter {
        let id = id_res.map_err(|e| format!("Erro ao obter ID: {}", e))?;
        let exp = buscar_experimento_por_id(&conn, id)?;
        result.push(exp);
    }

    Ok(result)
}

/// Comando Tauri: Busca os detalhes completos de um experimento por ID.
#[tauri::command]
pub fn obter_experimento(app_handle: tauri::AppHandle, id: i64) -> Result<ExperimentoCompletoDto, String> {
    let conn = obter_conexao(&app_handle)?;
    buscar_experimento_por_id(&conn, id)
}

/// Comando Tauri: Edita um experimento e recria seus timepoints.
#[tauri::command]
pub fn editar_experimento(
    app_handle: tauri::AppHandle,
    id: i64,
    nome: String,
    descricao: Option<String>,
    conjunto_id: i64,
    responsavel: Option<String>,
    timepoints: Vec<String>,
) -> Result<ExperimentoCompletoDto, String> {
    if nome.trim().is_empty() {
        return Err("O nome do experimento não pode ser vazio.".to_string());
    }

    let mut conn = obter_conexao(&app_handle)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Falha ao iniciar transação: {}", e))?;

    // Atualizar experimento
    let rows_affected = tx
        .execute(
            "UPDATE experimentos SET nome = ?1, descricao = ?2, conjunto_id = ?3, responsavel = ?4, atualizado_em = CURRENT_TIMESTAMP WHERE id = ?5 AND ativo = 1",
            params![nome, descricao, conjunto_id, responsavel, id],
        )
        .map_err(|e| format!("Falha ao atualizar experimento: {}", e))?;

    if rows_affected == 0 {
        return Err("Experimento não encontrado ou inativo.".to_string());
    }

    // Nota: Como estamos na etapa 3 e ainda não existem sequências de teste dependentes no banco,
    // podemos remover e reinserir todos os timepoints do experimento.
    tx.execute("DELETE FROM timepoints WHERE experimento_id = ?1", params![id])
        .map_err(|e| format!("Falha ao limpar timepoints antigos: {}", e))?;

    // Inserir os novos timepoints
    for (i, rotulo) in timepoints.iter().enumerate() {
        if rotulo.trim().is_empty() {
            return Err("O rótulo do timepoint não pode ser vazio.".to_string());
        }
        tx.execute(
            "INSERT INTO timepoints (experimento_id, rotulo, ordem, opcional) VALUES (?1, ?2, ?3, 0)",
            params![id, rotulo.trim(), i as i64],
        )
        .map_err(|e| format!("Falha ao salvar timepoint '{}': {}", rotulo, e))?;
    }

    tx.commit()
        .map_err(|e| format!("Falha ao confirmar alterações: {}", e))?;

    buscar_experimento_por_id(&conn, id)
}

/// Comando Tauri: Exclusão lógica (soft-delete) do experimento.
///
/// Decisão de segurança: os dados filhos (grupos, animais, timepoints) continuam
/// no banco relacionados ao experimento para integridade referencial histórica,
/// mas ficam invisíveis porque o pai (`ativo = 0`) não será listado.
#[tauri::command]
pub fn excluir_experimento(app_handle: tauri::AppHandle, id: i64) -> Result<(), String> {
    let conn = obter_conexao(&app_handle)?;
    
    let rows_affected = conn
        .execute(
            "UPDATE experimentos SET ativo = 0, atualizado_em = CURRENT_TIMESTAMP WHERE id = ?1 AND ativo = 1",
            params![id],
        )
        .map_err(|e| format!("Falha ao excluir experimento: {}", e))?;

    if rows_affected == 0 {
        return Err("Experimento não encontrado ou já excluído.".to_string());
    }

    Ok(())
}

// =============================================================================
// CRUD: GRUPOS
// =============================================================================

/// Comando Tauri: Cria um novo grupo de tratamento em um experimento.
#[tauri::command]
pub fn criar_grupo(
    app_handle: tauri::AppHandle,
    experimento_id: i64,
    nome: String,
    cor: String,
) -> Result<GrupoDto, String> {
    if nome.trim().is_empty() {
        return Err("O nome do grupo não pode ser vazio.".to_string());
    }
    if cor.trim().is_empty() {
        return Err("A cor do grupo é obrigatória.".to_string());
    }

    let conn = obter_conexao(&app_handle)?;
    
    conn.execute(
        "INSERT INTO grupos (experimento_id, nome, cor, ativo) VALUES (?1, ?2, ?3, 1)",
        params![experimento_id, nome.trim(), cor.trim()],
    )
    .map_err(|e| format!("Falha ao salvar grupo: {}", e))?;

    let grupo_id = conn.last_insert_rowid();

    Ok(GrupoDto {
        id: grupo_id,
        experimento_id,
        nome: nome.trim().to_string(),
        cor: cor.trim().to_string(),
    })
}

/// Comando Tauri: Edita as informações de um grupo.
#[tauri::command]
pub fn editar_grupo(
    app_handle: tauri::AppHandle,
    id: i64,
    nome: String,
    cor: String,
) -> Result<GrupoDto, String> {
    if nome.trim().is_empty() {
        return Err("O nome do grupo não pode ser vazio.".to_string());
    }
    if cor.trim().is_empty() {
        return Err("A cor do grupo é obrigatória.".to_string());
    }

    let conn = obter_conexao(&app_handle)?;
    
    let mut stmt = conn
        .prepare("SELECT experimento_id FROM grupos WHERE id = ?1 AND ativo = 1")
        .map_err(|e| format!("Falha ao preparar busca: {}", e))?;
        
    let experimento_id = stmt
        .query_row(params![id], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Grupo não encontrado ou inativo: {}", e))?;

    conn.execute(
        "UPDATE grupos SET nome = ?1, cor = ?2, atualizado_em = CURRENT_TIMESTAMP WHERE id = ?3 AND ativo = 1",
        params![nome.trim(), cor.trim(), id],
    )
    .map_err(|e| format!("Falha ao atualizar grupo: {}", e))?;

    Ok(GrupoDto {
        id,
        experimento_id,
        nome: nome.trim().to_string(),
        cor: cor.trim().to_string(),
    })
}

/// Comando Tauri: Exclusão lógica (soft-delete) de um grupo.
#[tauri::command]
pub fn excluir_grupo(app_handle: tauri::AppHandle, id: i64) -> Result<(), String> {
    let conn = obter_conexao(&app_handle)?;
    
    let rows_affected = conn
        .execute(
            "UPDATE grupos SET ativo = 0, atualizado_em = CURRENT_TIMESTAMP WHERE id = ?1 AND ativo = 1",
            params![id],
        )
        .map_err(|e| format!("Falha ao excluir grupo: {}", e))?;

    if rows_affected == 0 {
        return Err("Grupo não encontrado ou já excluído.".to_string());
    }

    // Excluir também (soft-delete) os animais deste grupo
    conn.execute(
        "UPDATE animais SET ativo = 0, atualizado_em = CURRENT_TIMESTAMP WHERE grupo_id = ?1 AND ativo = 1",
        params![id],
    )
    .map_err(|e| format!("Falha ao desativar animais pertencentes ao grupo: {}", e))?;

    Ok(())
}

// =============================================================================
// CRUD: ANIMAIS
// =============================================================================

/// Comando Tauri: Adiciona um novo animal a um grupo de tratamento.
#[tauri::command]
pub fn criar_animal(
    app_handle: tauri::AppHandle,
    experimento_id: i64,
    grupo_id: i64,
    marcacao: String,
    peso: Option<f64>,
) -> Result<AnimalDto, String> {
    if marcacao.trim().is_empty() {
        return Err("A marcação do animal não pode ser vazia.".to_string());
    }
    if let Some(p) = peso {
        if p <= 0.0 {
            return Err("O peso do animal deve ser maior que zero.".to_string());
        }
    }

    let conn = obter_conexao(&app_handle)?;
    
    conn.execute(
        "INSERT INTO animais (experimento_id, grupo_id, marcacao, peso, ativo) VALUES (?1, ?2, ?3, ?4, 1)",
        params![experimento_id, grupo_id, marcacao.trim(), peso],
    )
    .map_err(|e| format!("Falha ao salvar animal: {}", e))?;

    let animal_id = conn.last_insert_rowid();

    Ok(AnimalDto {
        id: animal_id,
        experimento_id,
        grupo_id,
        marcacao: marcacao.trim().to_string(),
        peso,
    })
}

/// Comando Tauri: Edita a marcação e o peso de um animal.
#[tauri::command]
pub fn editar_animal(
    app_handle: tauri::AppHandle,
    id: i64,
    grupo_id: i64,
    marcacao: String,
    peso: Option<f64>,
) -> Result<AnimalDto, String> {
    if marcacao.trim().is_empty() {
        return Err("A marcação do animal não pode ser vazia.".to_string());
    }
    if let Some(p) = peso {
        if p <= 0.0 {
            return Err("O peso do animal deve ser maior que zero.".to_string());
        }
    }

    let conn = obter_conexao(&app_handle)?;
    
    let mut stmt = conn
        .prepare("SELECT experimento_id FROM animais WHERE id = ?1 AND ativo = 1")
        .map_err(|e| format!("Falha ao preparar busca: {}", e))?;
        
    let experimento_id = stmt
        .query_row(params![id], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Animal não encontrado ou inativo: {}", e))?;

    conn.execute(
        "UPDATE animais SET grupo_id = ?1, marcacao = ?2, peso = ?3, atualizado_em = CURRENT_TIMESTAMP WHERE id = ?4 AND ativo = 1",
        params![grupo_id, marcacao.trim(), peso, id],
    )
    .map_err(|e| format!("Falha ao atualizar animal: {}", e))?;

    Ok(AnimalDto {
        id,
        experimento_id,
        grupo_id,
        marcacao: marcacao.trim().to_string(),
        peso,
    })
}

/// Comando Tauri: Exclusão lógica (soft-delete) de um animal.
#[tauri::command]
pub fn excluir_animal(app_handle: tauri::AppHandle, id: i64) -> Result<(), String> {
    let conn = obter_conexao(&app_handle)?;
    
    let rows_affected = conn
        .execute(
            "UPDATE animais SET ativo = 0, atualizado_em = CURRENT_TIMESTAMP WHERE id = ?1 AND ativo = 1",
            params![id],
        )
        .map_err(|e| format!("Falha ao excluir animal: {}", e))?;

    if rows_affected == 0 {
        return Err("Animal não encontrado ou já excluído.".to_string());
    }

    Ok(())
}

// =============================================================================
// FUNÇÕES AUXILIARES DE CONSULTA
// =============================================================================

fn buscar_experimento_por_id(conn: &Connection, id: i64) -> Result<ExperimentoCompletoDto, String> {
    // Buscar dados base do experimento e o nome do seu conjunto de filamentos
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.nome, e.descricao, e.conjunto_id, c.nome, e.responsavel, e.criado_em, e.atualizado_em \
             FROM experimentos e \
             JOIN conjuntos_filamentos c ON e.conjunto_id = c.id \
             WHERE e.id = ?1 AND e.ativo = 1"
        )
        .map_err(|e| format!("Falha ao preparar consulta do experimento: {}", e))?;

    let row = stmt
        .query_row(params![id], |row| {
            let id: i64 = row.get(0)?;
            let nome: String = row.get(1)?;
            let descricao: Option<String> = row.get(2)?;
            let conjunto_id: i64 = row.get(3)?;
            let conjunto_nome: String = row.get(4)?;
            let responsavel: Option<String> = row.get(5)?;
            let criado_em: String = row.get(6)?;
            let atualizado_em: String = row.get(7)?;
            Ok((id, nome, descricao, conjunto_id, conjunto_nome, responsavel, criado_em, atualizado_em))
        })
        .map_err(|e| format!("Experimento não encontrado: {}", e))?;

    let (id, nome, descricao, conjunto_id, conjunto_nome, responsavel, criado_em, atualizado_em) = row;

    // Buscar timepoints ordenados
    let mut stmt_t = conn
        .prepare("SELECT id, rotulo, ordem, opcional FROM timepoints WHERE experimento_id = ?1 ORDER BY ordem ASC")
        .map_err(|e| format!("Falha ao preparar consulta de timepoints: {}", e))?;
        
    let timepoints_iter = stmt_t
        .query_map(params![id], |row| {
            Ok(TimepointDto {
                id: row.get(0)?,
                experimento_id: id,
                rotulo: row.get(1)?,
                ordem: row.get(2)?,
                opcional: row.get(3)?,
            })
        })
        .map_err(|e| format!("Erro ao buscar timepoints: {}", e))?;

    let mut timepoints = Vec::new();
    for t_res in timepoints_iter {
        timepoints.push(t_res.map_err(|e| format!("Erro ao ler timepoint: {}", e))?);
    }

    // Buscar grupos ativos
    let mut stmt_g = conn
        .prepare("SELECT id, nome, cor FROM grupos WHERE experimento_id = ?1 AND ativo = 1 ORDER BY criado_em ASC")
        .map_err(|e| format!("Falha ao preparar consulta de grupos: {}", e))?;
        
    let grupos_iter = stmt_g
        .query_map(params![id], |row| {
            let g_id: i64 = row.get(0)?;
            let g_nome: String = row.get(1)?;
            let g_cor: String = row.get(2)?;
            Ok((g_id, g_nome, g_cor))
        })
        .map_err(|e| format!("Erro ao buscar grupos: {}", e))?;

    let mut grupos = Vec::new();
    for g_res in grupos_iter {
        let (g_id, g_nome, g_cor) = g_res.map_err(|e| format!("Erro ao ler grupo: {}", e))?;
        
        // Buscar animais ativos para cada grupo
        let mut stmt_a = conn
            .prepare("SELECT id, marcacao, peso FROM animais WHERE grupo_id = ?1 AND ativo = 1 ORDER BY criado_em ASC")
            .map_err(|e| format!("Falha ao preparar consulta de animais: {}", e))?;
            
        let animais_iter = stmt_a
            .query_map(params![g_id], |row| {
                Ok(AnimalDto {
                    id: row.get(0)?,
                    experimento_id: id,
                    grupo_id: g_id,
                    marcacao: row.get(1)?,
                    peso: row.get(2)?,
                })
            })
            .map_err(|e| format!("Erro ao buscar animais: {}", e))?;

        let mut animais = Vec::new();
        for a_res in animais_iter {
            animais.push(a_res.map_err(|e| format!("Erro ao ler animal: {}", e))?);
        }

        grupos.push(GrupoCompletoDto {
            id: g_id,
            experimento_id: id,
            nome: g_nome,
            cor: g_cor,
            animais,
        });
    }

    Ok(ExperimentoCompletoDto {
        id,
        nome,
        descricao,
        conjunto_id,
        conjunto_nome,
        responsavel,
        criado_em,
        atualizado_em,
        timepoints,
        grupos,
    })
}

// =============================================================================
// TESTES UNITÁRIOS (USANDO BANCO EM MEMÓRIA)
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    fn inicializar_banco_mock() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        
        // Executar migrations da Etapa 2 e 3 em ordem
        conn.execute_batch(
            "CREATE TABLE conjuntos_filamentos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                nome TEXT NOT NULL,
                descricao TEXT,
                d REAL NOT NULL,
                ativo INTEGER NOT NULL DEFAULT 1,
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

            CREATE TABLE experimentos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                nome TEXT NOT NULL,
                descricao TEXT,
                conjunto_id INTEGER NOT NULL,
                responsavel TEXT,
                ativo INTEGER NOT NULL DEFAULT 1,
                criado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
                atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(conjunto_id) REFERENCES conjuntos_filamentos(id)
            );

            CREATE TABLE grupos (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                experimento_id INTEGER NOT NULL,
                nome TEXT NOT NULL,
                cor TEXT NOT NULL,
                ativo INTEGER NOT NULL DEFAULT 1,
                criado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
                atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(experimento_id) REFERENCES experimentos(id) ON DELETE CASCADE
            );

            CREATE TABLE animais (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                experimento_id INTEGER NOT NULL,
                grupo_id INTEGER NOT NULL,
                marcacao TEXT NOT NULL,
                peso REAL,
                ativo INTEGER NOT NULL DEFAULT 1,
                criado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
                atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(experimento_id) REFERENCES experimentos(id) ON DELETE CASCADE,
                FOREIGN KEY(grupo_id) REFERENCES grupos(id) ON DELETE CASCADE
            );

            CREATE TABLE timepoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                experimento_id INTEGER NOT NULL,
                rotulo TEXT NOT NULL,
                ordem INTEGER NOT NULL,
                opcional INTEGER NOT NULL DEFAULT 0,
                criado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
                atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(experimento_id) REFERENCES experimentos(id) ON DELETE CASCADE
            );"
        ).unwrap();

        // Inserir conjunto de filamentos mock para satisfazer FK
        conn.execute(
            "INSERT INTO conjuntos_filamentos (nome, d, ativo) VALUES ('Kit Teste', 0.25, 1)",
            [],
        ).unwrap();

        conn
    }

    #[test]
    fn test_fluxo_completo_experimentos() {
        let mut conn = inicializar_banco_mock();

        // 1. Criar Experimento
        let tx = conn.transaction().unwrap();
        tx.execute(
            "INSERT INTO experimentos (nome, conjunto_id, responsavel, ativo) VALUES ('Exp 1', 1, 'Dr. Silva', 1)",
            [],
        ).unwrap();
        let exp_id = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO timepoints (experimento_id, rotulo, ordem) VALUES (?1, 'basal 1', 0)",
            params![exp_id],
        ).unwrap();
        tx.execute(
            "INSERT INTO timepoints (experimento_id, rotulo, ordem) VALUES (?1, '1h', 1)",
            params![exp_id],
        ).unwrap();
        tx.commit().unwrap();

        // 2. Buscar Experimento e validar
        let exp = buscar_experimento_por_id(&conn, exp_id).unwrap();
        assert_eq!(exp.nome, "Exp 1");
        assert_eq!(exp.responsavel.unwrap(), "Dr. Silva");
        assert_eq!(exp.timepoints.len(), 2);
        assert_eq!(exp.timepoints[0].rotulo, "basal 1");
        assert_eq!(exp.timepoints[1].rotulo, "1h");
        assert_eq!(exp.conjunto_nome, "Kit Teste");

        // 3. Criar Grupo
        conn.execute(
            "INSERT INTO grupos (experimento_id, nome, cor, ativo) VALUES (?1, 'Controle', '#ff0000', 1)",
            params![exp_id],
        ).unwrap();
        let grupo_id = conn.last_insert_rowid();

        // 4. Criar Animal
        conn.execute(
            "INSERT INTO animais (experimento_id, grupo_id, marcacao, peso, ativo) VALUES (?1, ?2, '4P', 25.4, 1)",
            params![exp_id, grupo_id],
        ).unwrap();
        let animal_id = conn.last_insert_rowid();

        // 5. Validar carregamento completo
        let exp_c = buscar_experimento_por_id(&conn, exp_id).unwrap();
        assert_eq!(exp_c.grupos.len(), 1);
        assert_eq!(exp_c.grupos[0].nome, "Controle");
        assert_eq!(exp_c.grupos[0].cor, "#ff0000");
        assert_eq!(exp_c.grupos[0].animais.len(), 1);
        assert_eq!(exp_c.grupos[0].animais[0].marcacao, "4P");
        assert_eq!(exp_c.grupos[0].animais[0].peso.unwrap(), 25.4);

        // 6. Excluir animal (soft-delete)
        conn.execute(
            "UPDATE animais SET ativo = 0 WHERE id = ?1",
            params![animal_id],
        ).unwrap();
        let exp_pos_del = buscar_experimento_por_id(&conn, exp_id).unwrap();
        assert_eq!(exp_pos_del.grupos[0].animais.len(), 0); // animal excluído não aparece

        // 7. Excluir grupo (soft-delete)
        conn.execute(
            "UPDATE grupos SET ativo = 0 WHERE id = ?1",
            params![grupo_id],
        ).unwrap();
        let exp_pos_del_g = buscar_experimento_por_id(&conn, exp_id).unwrap();
        assert_eq!(exp_pos_del_g.grupos.len(), 0); // grupo excluído não aparece

        // 8. Excluir experimento (soft-delete)
        conn.execute(
            "UPDATE experimentos SET ativo = 0 WHERE id = ?1",
            params![exp_id],
        ).unwrap();
        assert!(buscar_experimento_por_id(&conn, exp_id).is_err()); // experimento inativo dá erro ao buscar
    }
}
