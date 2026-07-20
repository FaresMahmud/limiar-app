use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConjuntoFilamentosDto {
    pub id: i64,
    pub nome: String,
    pub descricao: Option<String>,
    pub d: f64,
    pub criado_em: String,
    pub atualizado_em: String,
    pub valores: Vec<f64>,
}

/// Calcula o valor 'd' a partir de uma lista de valores de filamentos (em gramas).
/// 
/// Algoritmo:
/// 1. Ordena a lista de valores em ordem crescente.
/// 2. Valida se há pelo menos 2 valores.
/// 3. Valida se todos são maiores que zero.
/// 4. Valida se não há duplicatas.
/// 5. Calcula log10 de cada um, tira a diferença de logs consecutivos e calcula a média dessas diferenças.
pub fn calcular_d(valores: &[f64]) -> Result<f64, String> {
    if valores.len() < 2 {
        return Err("O conjunto de filamentos deve conter pelo menos 2 valores para calcular d.".to_string());
    }

    // Clonar e ordenar os valores de forma segura (sem panic em NaNs, embora validemos isso)
    let mut ordenados = valores.to_vec();
    ordenados.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Validar valores
    for &val in &ordenados {
        if val.is_nan() || val.is_infinite() {
            return Err("Os valores dos filamentos devem ser números válidos.".to_string());
        }
        if val <= 0.0 {
            return Err("Todos os valores dos filamentos devem ser maiores que zero.".to_string());
        }
    }

    // Verificar se há duplicados (valores iguais)
    for i in 0..ordenados.len() - 1 {
        // Usamos uma pequena tolerância para floats
        if (ordenados[i+1] - ordenados[i]).abs() < 1e-9 {
            return Err("O conjunto não pode conter valores duplicados.".to_string());
        }
    }

    // Calcular as diferenças consecutivas no espaço log10
    let mut soma_diferencas = 0.0;
    for i in 0..ordenados.len() - 1 {
        let log_atual = ordenados[i].log10();
        let log_proximo = ordenados[i+1].log10();
        soma_diferencas += log_proximo - log_atual;
    }

    let d = soma_diferencas / (ordenados.len() - 1) as f64;

    if d <= 0.0 || d.is_nan() || d.is_infinite() {
        return Err("O valor d calculado é inválido (deve ser maior que zero).".to_string());
    }

    Ok(d)
}

/// Abre uma conexão com o banco de dados SQLite local no diretório de dados do app.
fn obter_conexao(app_handle: &tauri::AppHandle) -> Result<Connection, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Falha ao obter diretório de dados do app: {}", e))?;
    
    // Garantir que o diretório exista
    std::fs::create_dir_all(&app_dir)
        .map_err(|e| format!("Falha ao criar diretório de dados do app: {}", e))?;
        
    let db_path = app_dir.join("limiar.db");
    
    Connection::open(db_path)
        .map_err(|e| format!("Falha ao conectar ao banco de dados: {}", e))
}

/// Comando Tauri: Cria um novo conjunto de filamentos e seus valores relacionados.
#[tauri::command]
pub fn criar_conjunto(
    app_handle: tauri::AppHandle,
    nome: String,
    descricao: Option<String>,
    valores: Vec<f64>,
) -> Result<ConjuntoFilamentosDto, String> {
    if nome.trim().is_empty() {
        return Err("O nome do conjunto não pode ser vazio.".to_string());
    }

    // Calcular d
    let d = calcular_d(&valores)?;

    let mut conn = obter_conexao(&app_handle)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Falha ao iniciar transação: {}", e))?;

    // Inserir conjunto
    tx.execute(
        "INSERT INTO conjuntos_filamentos (nome, descricao, d, ativo) VALUES (?1, ?2, ?3, 1)",
        params![nome, descricao, d],
    )
    .map_err(|e| format!("Falha ao salvar conjunto de filamentos: {}", e))?;

    let conjunto_id = tx.last_insert_rowid();

    // Ordenar os filamentos para salvar na ordem correta
    let mut valores_ordenados = valores.clone();
    valores_ordenados.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Inserir cada filamento
    for (i, &val) in valores_ordenados.iter().enumerate() {
        tx.execute(
            "INSERT INTO filamentos (conjunto_id, forca_g, ordem) VALUES (?1, ?2, ?3)",
            params![conjunto_id, val, i as i64],
        )
        .map_err(|e| format!("Falha ao salvar filamento {}: {}", val, e))?;
    }

    tx.commit()
        .map_err(|e| format!("Falha ao confirmar transação: {}", e))?;

    // Buscar o conjunto recém criado com seus timestamps
    let conjunto = buscar_conjunto_por_id(&conn, conjunto_id)?;
    Ok(conjunto)
}

/// Comando Tauri: Lista todos os conjuntos de filamentos ativos (não deletados).
#[tauri::command]
pub fn listar_conjuntos(app_handle: tauri::AppHandle) -> Result<Vec<ConjuntoFilamentosDto>, String> {
    let conn = obter_conexao(&app_handle)?;
    
    let mut stmt = conn
        .prepare("SELECT id, nome, descricao, d, criado_em, atualizado_em FROM conjuntos_filamentos WHERE ativo = 1 ORDER BY criado_em DESC")
        .map_err(|e| format!("Falha ao preparar consulta: {}", e))?;
        
    let conjuntos_iter = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let nome: String = row.get(1)?;
            let descricao: Option<String> = row.get(2)?;
            let d: f64 = row.get(3)?;
            let criado_em: String = row.get(4)?;
            let atualizado_em: String = row.get(5)?;
            Ok((id, nome, descricao, d, criado_em, atualizado_em))
        })
        .map_err(|e| format!("Falha ao executar consulta: {}", e))?;

    let mut result = Vec::new();
    for conjunto_res in conjuntos_iter {
        let (id, nome, descricao, d, criado_em, atualizado_em) = conjunto_res
            .map_err(|e| format!("Erro ao ler linha: {}", e))?;
            
        // Buscar os valores de filamento para este conjunto
        let valores = buscar_valores_filamentos(&conn, id)?;
        
        result.push(ConjuntoFilamentosDto {
            id,
            nome,
            descricao,
            d,
            criado_em,
            atualizado_em,
            valores,
        });
    }

    Ok(result)
}

/// Comando Tauri: Edita um conjunto de filamentos existente.
#[tauri::command]
pub fn editar_conjunto(
    app_handle: tauri::AppHandle,
    id: i64,
    nome: String,
    descricao: Option<String>,
    valores: Vec<f64>,
) -> Result<ConjuntoFilamentosDto, String> {
    if nome.trim().is_empty() {
        return Err("O nome do conjunto não pode ser vazio.".to_string());
    }

    // Recalcular d
    let d = calcular_d(&valores)?;

    let mut conn = obter_conexao(&app_handle)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Falha ao iniciar transação: {}", e))?;

    // Atualizar dados do conjunto
    let rows_affected = tx
        .execute(
            "UPDATE conjuntos_filamentos SET nome = ?1, descricao = ?2, d = ?3, atualizado_em = CURRENT_TIMESTAMP WHERE id = ?4 AND ativo = 1",
            params![nome, descricao, d, id],
        )
        .map_err(|e| format!("Falha ao atualizar conjunto: {}", e))?;

    if rows_affected == 0 {
        return Err("Conjunto de filamentos não encontrado ou inativo.".to_string());
    }

    // Remover filamentos antigos
    tx.execute("DELETE FROM filamentos WHERE conjunto_id = ?1", params![id])
        .map_err(|e| format!("Falha ao limpar filamentos antigos: {}", e))?;

    // Ordenar os filamentos
    let mut valores_ordenados = valores.clone();
    valores_ordenados.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Inserir os novos filamentos
    for (i, &val) in valores_ordenados.iter().enumerate() {
        tx.execute(
            "INSERT INTO filamentos (conjunto_id, forca_g, ordem) VALUES (?1, ?2, ?3)",
            params![id, val, i as i64],
        )
        .map_err(|e| format!("Falha ao salvar filamento {}: {}", val, e))?;
    }

    tx.commit()
        .map_err(|e| format!("Falha ao confirmar alterações: {}", e))?;

    let conjunto = buscar_conjunto_por_id(&conn, id)?;
    Ok(conjunto)
}

/// Comando Tauri: Realiza a exclusão lógica (soft-delete) de um conjunto de filamentos.
#[tauri::command]
pub fn excluir_conjunto(app_handle: tauri::AppHandle, id: i64) -> Result<(), String> {
    let conn = obter_conexao(&app_handle)?;
    
    let rows_affected = conn
        .execute(
            "UPDATE conjuntos_filamentos SET ativo = 0, atualizado_em = CURRENT_TIMESTAMP WHERE id = ?1 AND ativo = 1",
            params![id],
        )
        .map_err(|e| format!("Falha ao excluir conjunto: {}", e))?;

    if rows_affected == 0 {
        return Err("Conjunto de filamentos não encontrado ou já excluído.".to_string());
    }

    Ok(())
}

/// Comando Tauri: Recalcula e atualiza o d de um conjunto com base nos filamentos salvos no banco.
#[tauri::command]
pub fn recalcular_d_conjunto(app_handle: tauri::AppHandle, id: i64) -> Result<f64, String> {
    let mut conn = obter_conexao(&app_handle)?;
    
    let valores = buscar_valores_filamentos(&conn, id)?;
    let d = calcular_d(&valores)?;

    conn.execute(
        "UPDATE conjuntos_filamentos SET d = ?1, atualizado_em = CURRENT_TIMESTAMP WHERE id = ?2 AND ativo = 1",
        params![d, id],
    )
    .map_err(|e| format!("Falha ao atualizar d no banco de dados: {}", e))?;

    Ok(d)
}

// ---- Funções Auxiliares de Consulta ----

fn buscar_conjunto_por_id(conn: &Connection, id: i64) -> Result<ConjuntoFilamentosDto, String> {
    let mut stmt = conn
        .prepare("SELECT id, nome, descricao, d, criado_em, atualizado_em FROM conjuntos_filamentos WHERE id = ?1 AND ativo = 1")
        .map_err(|e| format!("Falha ao preparar consulta: {}", e))?;

    let row = stmt
        .query_row(params![id], |row| {
            let id: i64 = row.get(0)?;
            let nome: String = row.get(1)?;
            let descricao: Option<String> = row.get(2)?;
            let d: f64 = row.get(3)?;
            let criado_em: String = row.get(4)?;
            let atualizado_em: String = row.get(5)?;
            Ok((id, nome, descricao, d, criado_em, atualizado_em))
        })
        .map_err(|e| format!("Conjunto de filamentos não encontrado: {}", e))?;

    let (id, nome, descricao, d, criado_em, atualizado_em) = row;
    let valores = buscar_valores_filamentos(conn, id)?;

    Ok(ConjuntoFilamentosDto {
        id,
        nome,
        descricao,
        d,
        criado_em,
        atualizado_em,
        valores,
    })
}

fn buscar_valores_filamentos(conn: &Connection, conjunto_id: i64) -> Result<Vec<f64>, String> {
    let mut stmt = conn
        .prepare("SELECT forca_g FROM filamentos WHERE conjunto_id = ?1 ORDER BY ordem ASC")
        .map_err(|e| format!("Falha ao preparar consulta de filamentos: {}", e))?;

    let filamentos_iter = stmt
        .query_map(params![conjunto_id], |row| {
            let forca_g: f64 = row.get(0)?;
            Ok(forca_g)
        })
        .map_err(|e| format!("Falha ao consultar filamentos: {}", e))?;

    let mut valores = Vec::new();
    for val_res in filamentos_iter {
        let val = val_res.map_err(|e| format!("Erro ao ler valor do filamento: {}", e))?;
        valores.push(val);
    }

    Ok(valores)
}

// =============================================================================
// TESTES UNITÁRIOS
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f64 = 1e-6;

    #[test]
    fn caso_normal_valores_ordenados() {
        // Exemplo: 0.008, 0.02, 0.04, 0.07, 0.16, 0.4, 0.6, 1.0, 1.4, 2.0, 4.0, 6.0
        let filamentos = vec![0.008, 0.02, 0.04, 0.07, 0.16, 0.4, 0.6, 1.0, 1.4, 2.0, 4.0, 6.0];
        let d = calcular_d(&filamentos).expect("deve calcular d com sucesso");

        let log_max = 6.0f64.log10();
        let log_min = 0.008f64.log10();
        let d_esperado = (log_max - log_min) / 11.0;

        assert!((d - d_esperado).abs() < TOL, "d = {}, esperado = {}", d, d_esperado);
    }

    #[test]
    fn caso_normal_valores_desordenados() {
        // Mesmos valores acima, mas fora de ordem
        let filamentos = vec![1.4, 0.02, 0.04, 4.0, 0.07, 0.16, 0.4, 0.6, 1.0, 0.008, 2.0, 6.0];
        let d = calcular_d(&filamentos).expect("deve calcular d mesmo desordenado");

        let log_max = 6.0f64.log10();
        let log_min = 0.008f64.log10();
        let d_esperado = (log_max - log_min) / 11.0;

        assert!((d - d_esperado).abs() < TOL, "d = {}, esperado = {}", d, d_esperado);
    }

    #[test]
    fn erro_valores_insuficientes() {
        assert!(calcular_d(&[]).is_err(), "vazio deve retornar erro");
        assert!(calcular_d(&[0.02]).is_err(), "1 elemento deve retornar erro");
    }

    #[test]
    fn erro_valores_invalidos() {
        // Zero ou negativos
        assert!(calcular_d(&[0.02, 0.0]).is_err(), "zero deve retornar erro");
        assert!(calcular_d(&[0.02, -0.04]).is_err(), "negativo deve retornar erro");
        assert!(calcular_d(&[0.02, f64::NAN]).is_err(), "NaN deve retornar erro");
    }

    #[test]
    fn erro_duplicados() {
        assert!(calcular_d(&[0.02, 0.04, 0.02]).is_err(), "duplicados devem retornar erro");
    }

    #[test]
    fn caso_kit_exemplo_valores() {
        // Exemplo de kit de filamentos von Frey (valores de catálogo padrão):
        // 0.02, 0.07, 0.16, 0.4, 1.0, 2.0, 4.0
        let filamentos = vec![0.02, 0.07, 0.16, 0.4, 1.0, 2.0, 4.0];
        let d = calcular_d(&filamentos).expect("deve calcular d com sucesso");
        
        let d_esperado = 0.3835;
        assert!((d - d_esperado).abs() < 0.0001, "d = {}, esperado = {}", d, d_esperado);
    }
}
