mod dixon;
mod dixon_table;
mod filamentos;
mod experimentos;
mod sequencias;

use dixon::{estimar_limiar, Resposta};
use filamentos::{
    criar_conjunto, editar_conjunto, excluir_conjunto, listar_conjuntos, recalcular_d_conjunto,
};
use experimentos::{
    criar_experimento, criar_experimento_completo, listar_experimentos, obter_experimento, editar_experimento, excluir_experimento,
    criar_grupo, editar_grupo, excluir_grupo,
    criar_animal, editar_animal, excluir_animal,
};
use sequencias::{
    iniciar_sequencia, registrar_resposta, desfazer_ultima_resposta, finalizar_sequencia,
    obter_sequencia_ativa, listar_sequencias_concluidas, cancelar_sequencia,
    calcular_estatisticas_experimento, obter_respostas_cruas_experimento, obter_limiares_experimento,
};
use serde::{Deserialize, Serialize};

/// Payload de saída do comando de cálculo (espelha `dixon::Estimativa`, serializável
/// para o frontend). Todos os campos intermediários vão junto para rastreabilidade.
#[derive(Debug, Serialize)]
pub struct LimiarResultado {
    pub limiar: f64,
    pub estimativa_log: f64,
    pub k: f64,
    pub xf_log10: f64,
    pub d: f64,
    pub n_nominal: usize,
    pub second_part: String,
    pub coluna: usize,
    pub sinal_invertido: bool,
    pub incremento_aplicado: bool,
    pub erro_padrao_sigma: f64,
}

/// Como o frontend informa cada resposta O/X.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RespostaDto {
    /// O — não retirou a pata.
    O,
    /// X — retirou a pata.
    X,
}

impl From<&RespostaDto> for Resposta {
    fn from(r: &RespostaDto) -> Self {
        match r {
            RespostaDto::O => Resposta::NaoRespondeu,
            RespostaDto::X => Resposta::Respondeu,
        }
    }
}

/// Comando Tauri: calcula o limiar (LD50/PWT) de uma série up-and-down.
///
/// Recebe a sequência cronológica de respostas O/X, as doses/filamentos reais
/// correspondentes e o intervalo `d` (em log10). Retorna o limiar e os valores
/// intermediários, ou uma mensagem de erro clara (nunca faz panic).
#[tauri::command]
fn calcular_limiar(
    respostas: Vec<RespostaDto>,
    doses: Vec<f64>,
    d: f64,
) -> Result<LimiarResultado, String> {
    let respostas: Vec<Resposta> = respostas.iter().map(Resposta::from).collect();
    match estimar_limiar(&respostas, &doses, d) {
        Ok(e) => Ok(LimiarResultado {
            limiar: e.limiar,
            estimativa_log: e.estimativa_log,
            k: e.k,
            xf_log10: e.xf_log10,
            d: e.d,
            n_nominal: e.n_nominal,
            second_part: e.second_part,
            coluna: e.coluna,
            sinal_invertido: e.sinal_invertido,
            incremento_aplicado: e.incremento_aplicado,
            erro_padrao_sigma: e.erro_padrao_sigma,
        }),
        Err(err) => Err(err.to_string()),
    }
}

fn executar_migracoes(conn: &rusqlite::Connection) -> Result<(), String> {
    let mut stmt = conn.prepare("PRAGMA user_version")
        .map_err(|e| format!("Falha ao ler user_version: {}", e))?;
    let version: i32 = stmt.query_row([], |row| row.get(0))
        .map_err(|e| format!("Falha ao obter user_version: {}", e))?;

    println!("Versão atual do schema SQLite: {}", version);

    if version < 1 {
        println!("Rodando migração 1: create_initial_tables");
        conn.execute_batch(include_str!("../migrations/001_create_initial_tables.sql"))
            .map_err(|e| format!("Falha na migração 1: {}", e))?;
        conn.execute("PRAGMA user_version = 1", [])
            .map_err(|e| format!("Falha ao atualizar user_version para 1: {}", e))?;
    }
    if version < 2 {
        println!("Rodando migração 2: create_experimentos_tables");
        conn.execute_batch(include_str!("../migrations/002_create_experimentos_tables.sql"))
            .map_err(|e| format!("Falha na migração 2: {}", e))?;
        conn.execute("PRAGMA user_version = 2", [])
            .map_err(|e| format!("Falha ao atualizar user_version para 2: {}", e))?;
    }
    if version < 3 {
        println!("Rodando migração 3: create_sequencias_tables");
        conn.execute_batch(include_str!("../migrations/003_create_sequencias_tables.sql"))
            .map_err(|e| format!("Falha na migração 3: {}", e))?;
        conn.execute("PRAGMA user_version = 3", [])
            .map_err(|e| format!("Falha ao atualizar user_version para 3: {}", e))?;
    }

    // Log de confirmação: quais tabelas existem no banco após as migrações.
    // Facilita diagnosticar futuramente problemas de "no such table".
    let mut stmt_tabelas = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .map_err(|e| format!("Falha ao listar tabelas: {}", e))?;
    let tabelas: Vec<String> = stmt_tabelas
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Falha ao consultar tabelas: {}", e))?
        .filter_map(Result::ok)
        .collect();
    let versao_final: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap_or(-1);
    println!(
        "Migrações concluídas. user_version = {} | {} tabelas: {}",
        versao_final,
        tabelas.len(),
        tabelas.join(", ")
    );

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // NOTA DE ARQUITETURA: toda a persistência é feita via comandos Rust que abrem
  // conexões `rusqlite` diretamente no arquivo `<app_data_dir>/limiar.db` (ver
  // `obter_conexao` em filamentos.rs/experimentos.rs/sequencias.rs). O schema é
  // criado por `executar_migracoes` no setup abaixo — ESTA é a única fonte de
  // verdade das migrações. Não usamos `tauri-plugin-sql` (o frontend nunca chama
  // `Database.load`), justamente para evitar dois bancos/dois mecanismos de
  // migração divergentes (foi o que confundiu o bug "no such table").
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_fs::init())
    .setup(|app| {
      use tauri::Manager;

      let app_dir = app.path().app_data_dir()?;
      std::fs::create_dir_all(&app_dir)?;
      let db_path = app_dir.join("limiar.db");

      println!("Inicializando banco de dados em: {:?}", db_path);
      let conn = rusqlite::Connection::open(db_path)?;
      executar_migracoes(&conn).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
        calcular_limiar,
        criar_conjunto,
        listar_conjuntos,
        editar_conjunto,
        excluir_conjunto,
        recalcular_d_conjunto,
        criar_experimento,
        criar_experimento_completo,
        listar_experimentos,
        obter_experimento,
        editar_experimento,
        excluir_experimento,
        criar_grupo,
        editar_grupo,
        excluir_grupo,
        criar_animal,
        editar_animal,
        excluir_animal,
        iniciar_sequencia,
        registrar_resposta,
        desfazer_ultima_resposta,
        finalizar_sequencia,
        obter_sequencia_ativa,
        listar_sequencias_concluidas,
        cancelar_sequencia,
        calcular_estatisticas_experimento,
        obter_respostas_cruas_experimento,
        obter_limiares_experimento
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
