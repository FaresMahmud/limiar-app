use std::collections::HashMap;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::Manager;
use crate::dixon::{estimar_limiar, Resposta};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SequenciaDto {
    pub id: i64,
    pub animal_id: i64,
    pub timepoint_id: i64,
    pub status: String,
    pub filamento_inicial: f64,
    pub limiar: Option<f64>,
    pub respostas: Vec<RespostaLinhaDto>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RespostaLinhaDto {
    pub ordem: i64,
    pub filamento_g: f64,
    pub resposta: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProximaSugestaoDto {
    pub sequencia_id: i64,
    pub proximo_filamento: f64,
    pub aviso: Option<String>,
    pub n_nominal: usize,
    pub pode_finalizar: bool,
    pub respostas: Vec<RespostaLinhaDto>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResultadoLimiarDto {
    pub sequencia_id: i64,
    pub limiar: f64,
    pub k: f64,
    pub xf: f64,
    pub d: f64,
    pub n_nominal: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SequenciaConcluidaDto {
    pub id: i64,
    pub animal_id: i64,
    pub animal_marcacao: String,
    pub grupo_nome: String,
    pub grupo_cor: String,
    pub timepoint_id: i64,
    pub timepoint_rotulo: String,
    pub filamento_inicial: f64,
    pub limiar: Option<f64>,
    pub status: String,
    pub criado_em: String,
    pub respostas: String,
}

/// Abre uma conexão com o banco de dados SQLite local.
fn obter_conexao(app_handle: &tauri::AppHandle) -> Result<Connection, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Falha ao obter diretório de dados do app: {}", e))?;
    let db_path = app_dir.join("limiar.db");
    Connection::open(db_path)
        .map_err(|e| format!("Falha ao conectar ao banco de dados: {}", e))
}

// =============================================================================
// FUNÇÕES AUXILIARES DE LÓGICA CIENTÍFICA
// =============================================================================

/// Determina o próximo filamento a sugerir.
/// Se atingir o limite (mais forte ou mais fraco), repete e retorna um aviso.
pub fn sugerir_proximo_filamento(
    valores: &[f64],
    ultimo_valor: f64,
    ultima_resposta: &str,
) -> (f64, Option<String>) {
    if valores.is_empty() {
        return (ultimo_valor, None);
    }
    
    // Localizar o índice correspondente
    let mut idx = 0;
    let mut encontrado = false;
    for (i, &v) in valores.iter().enumerate() {
        if (v - ultimo_valor).abs() < 1e-9 {
            idx = i;
            encontrado = true;
            break;
        }
    }
    if !encontrado {
        // Fallback: achar o mais próximo
        let mut min_diff = f64::MAX;
        for (i, &v) in valores.iter().enumerate() {
            let diff = (v - ultimo_valor).abs();
            if diff < min_diff {
                min_diff = diff;
                idx = i;
            }
        }
    }

    match ultima_resposta {
        "O" => {
            if idx + 1 < valores.len() {
                (valores[idx + 1], None)
            } else {
                (valores[idx], Some("Atenção: Limite superior atingido. Sugerindo o filamento mais forte novamente.".to_string()))
            }
        }
        "X" => {
            if idx > 0 {
                (valores[idx - 1], None)
            } else {
                (valores[idx], Some("Atenção: Limite inferior atingido. Sugerindo o filamento mais fraco novamente.".to_string()))
            }
        }
        _ => (ultimo_valor, None),
    }
}

/// Calcula o N nominal com base na sequência de respostas O/X.
/// N nominal máximo coberto pela Tabela 7 de Dixon (ver `dixon_table.rs`).
pub const N_NOMINAL_MAX: usize = 6;

/// A finalização só é possível quando o N nominal está na faixa que a Tabela 7
/// cobre (2 a 6).
///
/// ⚠️ O limite SUPERIOR é essencial: sem ele, o botão "Finalizar" ficava habilitado
/// em séries com N > 6, a finalização falhava sempre no motor de Dixon
/// (`NNominalForaDaTabela`) e — como o erro era exibido só no topo da página, fora
/// da viewport — o usuário via o botão "não fazer nada". Ver ARQUITETURA.md §10.
pub fn pode_finalizar_agora(n_nominal: usize) -> bool {
    n_nominal >= 2 && n_nominal <= N_NOMINAL_MAX
}

/// Aviso mostrado quando a série passou do N máximo da tabela.
fn aviso_n_excedido(n_nominal: usize) -> Option<String> {
    if n_nominal > N_NOMINAL_MAX {
        Some(format!(
            "Esta série já está em N = {}, mas a Tabela 7 de Dixon cobre apenas N de 2 a {}. \
             Desfaça aplicações até N ≤ {} para poder finalizar, ou descarte o teste.",
            n_nominal, N_NOMINAL_MAX, N_NOMINAL_MAX
        ))
    } else {
        None
    }
}

pub fn calcular_n_nominal_atual(respostas: &[String]) -> usize {
    if respostas.is_empty() {
        return 0;
    }
    let lider = &respostas[0];
    let m = respostas.iter().take_while(|x| *x == lider).count();
    if m == respostas.len() {
        // Nenhuma reversão ainda
        return 0;
    }
    let cauda_len = respostas.len() - m;
    cauda_len + 1
}

// =============================================================================
// COMANDOS TAURI
// =============================================================================

/// Inicia uma nova sequência de testes.
/// Retorna erro se o animal já tiver uma sequência 'em_andamento' no mesmo timepoint.
#[tauri::command]
pub fn iniciar_sequencia(
    app_handle: tauri::AppHandle,
    animal_id: i64,
    timepoint_id: i64,
    filamento_inicial: f64,
) -> Result<SequenciaDto, String> {
    let conn = obter_conexao(&app_handle)?;

    // Verificar se já existe uma ativa
    let mut stmt = conn
        .prepare("SELECT id FROM sequencias_teste WHERE animal_id = ?1 AND timepoint_id = ?2 AND status = 'em_andamento'")
        .map_err(|e| format!("Falha ao verificar sequências ativas: {}", e))?;
        
    let existe_ativa = stmt.exists(params![animal_id, timepoint_id])
        .map_err(|e| format!("Erro ao consultar banco: {}", e))?;

    if existe_ativa {
        return Err("Este animal já possui uma sequência de testes em andamento para este timepoint.".to_string());
    }

    conn.execute(
        "INSERT INTO sequencias_teste (animal_id, timepoint_id, status, filamento_inicial) VALUES (?1, ?2, 'em_andamento', ?3)",
        params![animal_id, timepoint_id, filamento_inicial],
    )
    .map_err(|e| format!("Falha ao salvar sequência no banco: {}", e))?;

    let seq_id = conn.last_insert_rowid();

    Ok(SequenciaDto {
        id: seq_id,
        animal_id,
        timepoint_id,
        status: "em_andamento".to_string(),
        filamento_inicial,
        limiar: None,
        respostas: Vec::new(),
    })
}

/// Registra uma resposta individual ('O' ou 'X') na sequência em andamento.
/// Determina o próximo filamento a sugerir.
#[tauri::command]
pub fn registrar_resposta(
    app_handle: tauri::AppHandle,
    sequencia_id: i64,
    resposta: String,
) -> Result<ProximaSugestaoDto, String> {
    if resposta != "O" && resposta != "X" {
        return Err("A resposta deve ser 'O' (não respondeu) ou 'X' (respondeu).".to_string());
    }

    let mut conn = obter_conexao(&app_handle)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Falha ao iniciar transação: {}", e))?;

    // 1. Obter informações básicas da sequência
    let mut stmt_s = tx
        .prepare("SELECT animal_id, filamento_inicial, status FROM sequencias_teste WHERE id = ?1")
        .map_err(|e| format!("Falha ao preparar consulta: {}", e))?;
        
    let (animal_id, filamento_inicial, status) = stmt_s
        .query_row(params![sequencia_id], |row| {
            let animal_id: i64 = row.get(0)?;
            let filamento_inicial: f64 = row.get(1)?;
            let status: String = row.get(2)?;
            Ok((animal_id, filamento_inicial, status))
        })
        .map_err(|e| format!("Sequência de teste não encontrada: {}", e))?;

    if status != "em_andamento" {
        return Err("Esta sequência de teste já foi finalizada.".to_string());
    }

    // 2. Buscar a lista de filamentos cadastrados para o experimento da sequência
    let mut stmt_f = tx
        .prepare(
            "SELECT f.forca_g FROM filamentos f \
             JOIN experimentos e ON e.conjunto_id = f.conjunto_id \
             JOIN animais a ON a.experimento_id = e.id \
             WHERE a.id = ?1 ORDER BY f.ordem ASC"
        )
        .map_err(|e| format!("Falha ao preparar consulta dos filamentos: {}", e))?;
        
    let fil_iter = stmt_f
        .query_map(params![animal_id], |row| row.get::<_, f64>(0))
        .map_err(|e| format!("Erro ao obter filamentos: {}", e))?;

    let mut valores_kit = Vec::new();
    for f_res in fil_iter {
        valores_kit.push(f_res.map_err(|e| format!("Erro ao carregar filamento: {}", e))?);
    }
    
    if valores_kit.is_empty() {
        return Err("Nenhum filamento configurado para o kit deste experimento.".to_string());
    }

    // 3. Obter as respostas já cadastradas
    let mut stmt_r = tx
        .prepare("SELECT ordem, filamento_g, resposta FROM respostas_sequencia WHERE sequencia_id = ?1 ORDER BY ordem ASC")
        .map_err(|e| format!("Falha ao preparar consulta das respostas: {}", e))?;
        
    let resp_iter = stmt_r
        .query_map(params![sequencia_id], |row| {
            Ok(RespostaLinhaDto {
                ordem: row.get(0)?,
                filamento_g: row.get(1)?,
                resposta: row.get(2)?,
            })
        })
        .map_err(|e| format!("Erro ao listar respostas: {}", e))?;

    let mut respostas_atuais = Vec::new();
    for r_res in resp_iter {
        respostas_atuais.push(r_res.map_err(|e| format!("Erro ao ler resposta: {}", e))?);
    }

    // 4. Calcular filamento testado e nova ordem
    let ordem_nova = respostas_atuais.len() as i64;
    let filamento_testado = if ordem_nova == 0 {
        filamento_inicial
    } else {
        // O valor testado nesta etapa foi a sugestão calculada no passo anterior
        let ultima_resp = &respostas_atuais[(ordem_nova - 1) as usize];
        let (sugestao, _) = sugerir_proximo_filamento(&valores_kit, ultima_resp.filamento_g, &ultima_resp.resposta);
        sugestao
    };

    // 5. Inserir a nova resposta
    tx.execute(
        "INSERT INTO respostas_sequencia (sequencia_id, ordem, filamento_g, resposta) VALUES (?1, ?2, ?3, ?4)",
        params![sequencia_id, ordem_nova, filamento_testado, resposta],
    )
    .map_err(|e| format!("Falha ao registrar resposta no banco: {}", e))?;

    // Adiciona na lista local para o cálculo de sugestão do próximo passo
    respostas_atuais.push(RespostaLinhaDto {
        ordem: ordem_nova,
        filamento_g: filamento_testado,
        resposta: resposta.clone(),
    });

    drop(stmt_s);
    drop(stmt_f);
    drop(stmt_r);

    tx.commit()
        .map_err(|e| format!("Falha ao salvar transação: {}", e))?;

    // 6. Calcular a próxima sugestão e estatísticas científicas
    let (proximo_filamento, aviso) = sugerir_proximo_filamento(&valores_kit, filamento_testado, &resposta);
    
    let respostas_str: Vec<String> = respostas_atuais.iter().map(|r| r.resposta.clone()).collect();
    let n_nominal = calcular_n_nominal_atual(&respostas_str);
    let pode_finalizar = pode_finalizar_agora(n_nominal);
    let aviso = aviso_n_excedido(n_nominal).or(aviso);

    Ok(ProximaSugestaoDto {
        sequencia_id,
        proximo_filamento,
        aviso,
        n_nominal,
        pode_finalizar,
        respostas: respostas_atuais,
    })
}

/// Desfaz o último teste registrado na sequência em andamento.
#[tauri::command]
pub fn desfazer_ultima_resposta(
    app_handle: tauri::AppHandle,
    sequencia_id: i64,
) -> Result<ProximaSugestaoDto, String> {
    let mut conn = obter_conexao(&app_handle)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("Falha ao iniciar transação: {}", e))?;

    // 1. Validar se está em andamento
    let mut stmt_s = tx
        .prepare("SELECT animal_id, filamento_inicial, status FROM sequencias_teste WHERE id = ?1")
        .map_err(|e| format!("Falha ao preparar consulta: {}", e))?;
        
    let (animal_id, filamento_inicial, status) = stmt_s
        .query_row(params![sequencia_id], |row| {
            let animal_id: i64 = row.get(0)?;
            let filamento_inicial: f64 = row.get(1)?;
            let status: String = row.get(2)?;
            Ok((animal_id, filamento_inicial, status))
        })
        .map_err(|e| format!("Sequência não encontrada: {}", e))?;

    if status != "em_andamento" {
        return Err("Apenas sequências de teste em andamento podem ser modificadas.".to_string());
    }

    // 2. Excluir a de ordem mais alta
    tx.execute(
        "DELETE FROM respostas_sequencia WHERE sequencia_id = ?1 AND ordem = (SELECT MAX(ordem) FROM respostas_sequencia WHERE sequencia_id = ?1)",
        params![sequencia_id],
    )
    .map_err(|e| format!("Falha ao excluir última resposta: {}", e))?;

    // 3. Buscar filamentos do kit
    let mut stmt_f = tx
        .prepare(
            "SELECT f.forca_g FROM filamentos f \
             JOIN experimentos e ON e.conjunto_id = f.conjunto_id \
             JOIN animais a ON a.experimento_id = e.id \
             WHERE a.id = ?1 ORDER BY f.ordem ASC"
        )
        .map_err(|e| format!("Falha ao preparar consulta dos filamentos: {}", e))?;
        
    let fil_iter = stmt_f
        .query_map(params![animal_id], |row| row.get::<_, f64>(0))
        .map_err(|e| format!("Erro ao obter filamentos: {}", e))?;

    let mut valores_kit = Vec::new();
    for f_res in fil_iter {
        valores_kit.push(f_res.map_err(|e| format!("Erro ao carregar filamento: {}", e))?);
    }

    // 4. Buscar respostas restantes
    let mut stmt_r = tx
        .prepare("SELECT ordem, filamento_g, resposta FROM respostas_sequencia WHERE sequencia_id = ?1 ORDER BY ordem ASC")
        .map_err(|e| format!("Falha ao preparar consulta das respostas: {}", e))?;
        
    let resp_iter = stmt_r
        .query_map(params![sequencia_id], |row| {
            Ok(RespostaLinhaDto {
                ordem: row.get(0)?,
                filamento_g: row.get(1)?,
                resposta: row.get(2)?,
            })
        })
        .map_err(|e| format!("Erro ao listar respostas: {}", e))?;

    let mut respostas_restantes = Vec::new();
    for r_res in resp_iter {
        respostas_restantes.push(r_res.map_err(|e| format!("Erro ao ler resposta: {}", e))?);
    }

    drop(stmt_s);
    drop(stmt_f);
    drop(stmt_r);

    tx.commit()
        .map_err(|e| format!("Falha ao salvar transação: {}", e))?;

    // 5. Determinar próxima sugestão e estatísticas com base no novo estado
    let (proximo_filamento, aviso) = if respostas_restantes.is_empty() {
        (filamento_inicial, None)
    } else {
        let ultima_resp = &respostas_restantes[respostas_restantes.len() - 1];
        sugerir_proximo_filamento(&valores_kit, ultima_resp.filamento_g, &ultima_resp.resposta)
    };

    let respostas_str: Vec<String> = respostas_restantes.iter().map(|r| r.resposta.clone()).collect();
    let n_nominal = calcular_n_nominal_atual(&respostas_str);
    let pode_finalizar = pode_finalizar_agora(n_nominal);
    let aviso = aviso_n_excedido(n_nominal).or(aviso);

    Ok(ProximaSugestaoDto {
        sequencia_id,
        proximo_filamento,
        aviso,
        n_nominal,
        pode_finalizar,
        respostas: respostas_restantes,
    })
}

/// Finaliza a sequência e executa o cálculo do limiar pelo motor de Dixon.
#[tauri::command]
pub fn finalizar_sequencia(
    app_handle: tauri::AppHandle,
    sequencia_id: i64,
) -> Result<ResultadoLimiarDto, String> {
    let mut conn = obter_conexao(&app_handle)?;
    finalizar_sequencia_conn(&mut conn, sequencia_id)
}

/// Núcleo da finalização, separado do `AppHandle` para ser testável com um banco
/// em memória. Qualquer falha aqui (sequência já finalizada, série vazia, N fora
/// da Tabela 7 de Dixon, erro de banco) volta como `Err` e **precisa** ser exibida
/// ao usuário — ver `erroTeste` em App.svelte e ARQUITETURA.md §10.
pub fn finalizar_sequencia_conn(
    conn: &mut Connection,
    sequencia_id: i64,
) -> Result<ResultadoLimiarDto, String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("Falha ao iniciar transação: {}", e))?;

    // 1. Verificar se a sequência está ativa
    let mut stmt_s = tx
        .prepare("SELECT animal_id, status FROM sequencias_teste WHERE id = ?1")
        .map_err(|e| format!("Falha ao buscar sequência: {}", e))?;
        
    let (animal_id, status) = stmt_s
        .query_row(params![sequencia_id], |row| {
            let animal_id: i64 = row.get(0)?;
            let status: String = row.get(1)?;
            Ok((animal_id, status))
        })
        .map_err(|e| format!("Sequência não encontrada: {}", e))?;

    if status != "em_andamento" {
        return Err("Esta sequência de teste já foi finalizada.".to_string());
    }

    // 2. Obter o valor d do kit cadastrado
    let mut stmt_d = tx
        .prepare(
            "SELECT c.d FROM conjuntos_filamentos c \
             JOIN experimentos e ON e.conjunto_id = c.id \
             JOIN animais a ON a.experimento_id = e.id \
             WHERE a.id = ?1"
        )
        .map_err(|e| format!("Falha ao preparar consulta de d: {}", e))?;
        
    let d = stmt_d
        .query_row(params![animal_id], |row| row.get::<_, f64>(0))
        .map_err(|e| format!("Não foi possível obter o d do kit de filamentos deste animal: {}", e))?;

    // 3. Obter todas as respostas cadastradas
    let mut stmt_r = tx
        .prepare("SELECT filamento_g, resposta FROM respostas_sequencia WHERE sequencia_id = ?1 ORDER BY ordem ASC")
        .map_err(|e| format!("Falha ao preparar consulta das respostas: {}", e))?;
        
    let resp_iter = stmt_r
        .query_map(params![sequencia_id], |row| {
            let fil: f64 = row.get(0)?;
            let resp: String = row.get(1)?;
            Ok((fil, resp))
        })
        .map_err(|e| format!("Erro ao obter respostas: {}", e))?;

    let mut doses = Vec::new();
    let mut respostas_dto = Vec::new();
    for r_res in resp_iter {
        let (fil, resp) = r_res.map_err(|e| format!("Erro ao ler resposta: {}", e))?;
        doses.push(fil);
        respostas_dto.push(resp);
    }

    if respostas_dto.is_empty() {
        return Err("A sequência de testes está vazia. Registre pelo menos uma resposta antes de finalizar.".to_string());
    }

    // 4. Mapear para tipos aceitos pelo dixon.rs
    let respostas_dixon: Vec<Resposta> = respostas_dto
        .iter()
        .map(|r| {
            if r == "X" {
                Resposta::Respondeu
            } else {
                Resposta::NaoRespondeu
            }
        })
        .collect();

    // 5. Chamar motor de cálculo de Dixon
    let estimativa = estimar_limiar(&respostas_dixon, &doses, d)
        .map_err(|e| format!("Falha ao calcular o limiar de Dixon: {}", e))?;

    // 6. Atualizar a sequência como concluída
    tx.execute(
        "UPDATE sequencias_teste SET \
         status = 'concluida', \
         limiar = ?1, \
         estimativa_log = ?2, \
         k_dixon = ?3, \
         d_usado = ?4, \
         n_nominal = ?5, \
         atualizado_em = CURRENT_TIMESTAMP \
         WHERE id = ?6",
        params![
            estimativa.limiar,
            estimativa.estimativa_log,
            estimativa.k,
            estimativa.d,
            estimativa.n_nominal as i64,
            sequencia_id
        ],
    )
    .map_err(|e| format!("Falha ao salvar o limiar no banco: {}", e))?;

    drop(stmt_s);
    drop(stmt_d);
    drop(stmt_r);

    tx.commit()
        .map_err(|e| format!("Falha ao confirmar transação: {}", e))?;

    Ok(ResultadoLimiarDto {
        sequencia_id,
        limiar: estimativa.limiar,
        k: estimativa.k,
        xf: *doses.last().unwrap(),
        d: estimativa.d,
        n_nominal: estimativa.n_nominal,
    })
}

/// Obtém a sequência de teste em andamento para o animal e timepoint informado (se existir).
#[tauri::command]
pub fn obter_sequencia_ativa(
    app_handle: tauri::AppHandle,
    animal_id: i64,
    timepoint_id: i64,
) -> Result<Option<SequenciaDto>, String> {
    let conn = obter_conexao(&app_handle)?;

    let mut stmt = conn
        .prepare(
            "SELECT id, filamento_inicial FROM sequencias_teste \
             WHERE animal_id = ?1 AND timepoint_id = ?2 AND status = 'em_andamento'"
        )
        .map_err(|e| format!("Falha ao preparar busca de sequência ativa: {}", e))?;

    let row_res = stmt.query_row(params![animal_id, timepoint_id], |row| {
        let id: i64 = row.get(0)?;
        let filamento_inicial: f64 = row.get(1)?;
        Ok((id, filamento_inicial))
    });

    match row_res {
        Ok((id, filamento_inicial)) => {
            // Buscar respostas já cadastradas para restaurar
            let mut stmt_r = conn
                .prepare("SELECT ordem, filamento_g, resposta FROM respostas_sequencia WHERE sequencia_id = ?1 ORDER BY ordem ASC")
                .map_err(|e| format!("Falha ao carregar respostas: {}", e))?;
                
            let resp_iter = stmt_r
                .query_map(params![id], |row| {
                    Ok(RespostaLinhaDto {
                        ordem: row.get(0)?,
                        filamento_g: row.get(1)?,
                        resposta: row.get(2)?,
                    })
                })
                .map_err(|e| format!("Erro ao ler respostas: {}", e))?;

            let mut respostas = Vec::new();
            for r in resp_iter {
                respostas.push(r.map_err(|e| format!("Erro de leitura: {}", e))?);
            }

            Ok(Some(SequenciaDto {
                id,
                animal_id,
                timepoint_id,
                status: "em_andamento".to_string(),
                filamento_inicial,
                limiar: None,
                respostas,
            }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Erro ao buscar no banco: {}", e)),
    }
}

/// Lista todas as sequências já concluídas de um experimento.
#[tauri::command]
pub fn listar_sequencias_concluidas(
    app_handle: tauri::AppHandle,
    experimento_id: i64,
) -> Result<Vec<SequenciaConcluidaDto>, String> {
    let conn = obter_conexao(&app_handle)?;

    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.animal_id, a.marcacao, g.nome, g.cor, s.timepoint_id, t.rotulo, s.filamento_inicial, s.limiar, s.status, s.criado_em \
             FROM sequencias_teste s \
             JOIN animais a ON s.animal_id = a.id \
             JOIN grupos g ON a.grupo_id = g.id \
             JOIN timepoints t ON s.timepoint_id = t.id \
             WHERE a.experimento_id = ?1 \
             ORDER BY s.criado_em DESC"
        )
        .map_err(|e| format!("Falha ao preparar consulta: {}", e))?;

    let seq_iter = stmt
        .query_map(params![experimento_id], |row| {
            let id: i64 = row.get(0)?;
            let animal_id: i64 = row.get(1)?;
            let animal_marcacao: String = row.get(2)?;
            let grupo_nome: String = row.get(3)?;
            let grupo_cor: String = row.get(4)?;
            let timepoint_id: i64 = row.get(5)?;
            let timepoint_rotulo: String = row.get(6)?;
            let filamento_inicial: f64 = row.get(7)?;
            let limiar: Option<f64> = row.get(8)?;
            let status: String = row.get(9)?;
            let criado_em: String = row.get(10)?;
            Ok((id, animal_id, animal_marcacao, grupo_nome, grupo_cor, timepoint_id, timepoint_rotulo, filamento_inicial, limiar, status, criado_em))
        })
        .map_err(|e| format!("Erro ao obter lista de concluídas: {}", e))?;

    let mut result = Vec::new();
    for row_res in seq_iter {
        let (id, animal_id, animal_marcacao, grupo_nome, grupo_cor, timepoint_id, timepoint_rotulo, filamento_inicial, limiar, status, criado_em) =
            row_res.map_err(|e| format!("Erro ao mapear linha: {}", e))?;

        // Montar a string de respostas concatenadas (ex: "OXXOX")
        let mut stmt_r = conn
            .prepare("SELECT resposta FROM respostas_sequencia WHERE sequencia_id = ?1 ORDER BY ordem ASC")
            .map_err(|e| format!("Falha ao buscar respostas para a string: {}", e))?;
            
        let resp_iter = stmt_r
            .query_map(params![id], |row| row.get::<_, String>(0))
            .map_err(|e| format!("Erro ao mapear respostas: {}", e))?;

        let mut respostas = String::new();
        for r_res in resp_iter {
            respostas.push_str(&r_res.map_err(|e| format!("Erro ao carregar resposta: {}", e))?);
        }

        result.push(SequenciaConcluidaDto {
            id,
            animal_id,
            animal_marcacao,
            grupo_nome,
            grupo_cor,
            timepoint_id,
            timepoint_rotulo,
            filamento_inicial,
            limiar,
            status,
            criado_em,
            respostas,
        });
    }

    Ok(result)
}

/// Comando Tauri: Descarta uma sequência em andamento.
#[tauri::command]
pub fn cancelar_sequencia(app_handle: tauri::AppHandle, id: i64) -> Result<(), String> {
    let conn = obter_conexao(&app_handle)?;
    
    // Deleta fisicamente pois é uma sequência descartada pelo operador
    conn.execute("DELETE FROM sequencias_teste WHERE id = ?1 AND status = 'em_andamento'", params![id])
        .map_err(|e| format!("Falha ao cancelar sequência: {}", e))?;
        
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrupoTimepointStatsDto {
    pub grupo_id: i64,
    pub grupo_nome: String,
    pub grupo_cor: String,
    pub timepoint_id: i64,
    pub timepoint_rotulo: String,
    pub timepoint_ordem: i64,
    pub n: i64,
    pub media_geometrica_g: f64,
    pub limite_inferior_g: Option<f64>,
    pub limite_superior_g: Option<f64>,
}

/// Executa os cálculos estatísticos puros no espaço logarítmico para um grupo de limiares.
/// Retorna (n, media_geometrica, limite_inferior, limite_superior)
pub fn calcular_estatisticas_log(limiares: &[f64]) -> (i64, f64, Option<f64>, Option<f64>) {
    let n = limiares.len() as i64;
    if n == 0 {
        return (0, 0.0, None, None);
    }
    
    let logs: Vec<f64> = limiares.iter().map(|&x| x.log10()).collect();
    let soma_logs: f64 = logs.iter().sum();
    let media_log = soma_logs / (n as f64);
    
    let media_geometrica_g = 10.0f64.powf(media_log);
    
    let (limite_inferior_g, limite_superior_g) = if n > 1 {
        let soma_variancia: f64 = logs.iter().map(|&l| (l - media_log).powi(2)).sum();
        let desvio_padrao_log = (soma_variancia / ((n - 1) as f64)).sqrt();
        let erro_padrao_log = desvio_padrao_log / (n as f64).sqrt();
        
        let inf = 10.0f64.powf(media_log - erro_padrao_log);
        let sup = 10.0f64.powf(media_log + erro_padrao_log);
        (Some(inf), Some(sup))
    } else {
        (None, None)
    };
    
    (n, media_geometrica_g, limite_inferior_g, limite_superior_g)
}

/// Comando Tauri: Agrega estatisticamente os limiares concluídos do experimento informado.
#[tauri::command]
pub fn calcular_estatisticas_experimento(
    app_handle: tauri::AppHandle,
    experimento_id: i64,
) -> Result<Vec<GrupoTimepointStatsDto>, String> {
    let conn = obter_conexao(&app_handle)?;
    
    let mut stmt = conn.prepare(
        "SELECT \
            a.grupo_id, \
            g.nome AS grupo_nome, \
            g.cor AS grupo_cor, \
            s.timepoint_id, \
            t.rotulo AS timepoint_rotulo, \
            t.ordem AS timepoint_ordem, \
            s.limiar \
         FROM sequencias_teste s \
         JOIN animais a ON s.animal_id = a.id \
         JOIN grupos g ON a.grupo_id = g.id \
         JOIN timepoints t ON s.timepoint_id = t.id \
         WHERE a.experimento_id = ?1 AND s.status = 'concluida' \
         ORDER BY a.grupo_id, t.ordem"
    ).map_err(|e| format!("Falha ao preparar consulta estatística: {}", e))?;

    let rows_iter = stmt.query_map(params![experimento_id], |row| {
        let grupo_id: i64 = row.get(0)?;
        let grupo_nome: String = row.get(1)?;
        let grupo_cor: String = row.get(2)?;
        let timepoint_id: i64 = row.get(3)?;
        let timepoint_rotulo: String = row.get(4)?;
        let timepoint_ordem: i64 = row.get(5)?;
        let limiar: f64 = row.get(6)?;
        Ok((grupo_id, grupo_nome, grupo_cor, timepoint_id, timepoint_rotulo, timepoint_ordem, limiar))
    }).map_err(|e| format!("Erro ao executar consulta: {}", e))?;

    struct GrupoTpKey {
        grupo_id: i64,
        grupo_nome: String,
        grupo_cor: String,
        timepoint_id: i64,
        timepoint_rotulo: String,
        timepoint_ordem: i64,
    }
    
    let mut grupos_map: HashMap<(i64, i64), (GrupoTpKey, Vec<f64>)> = HashMap::new();
    
    for row_res in rows_iter {
        let (g_id, g_nome, g_cor, tp_id, tp_rotulo, tp_ordem, limiar) = 
            row_res.map_err(|e| format!("Erro ao carregar linha: {}", e))?;
            
        let entry = grupos_map.entry((g_id, tp_id)).or_insert_with(|| {
            (
                GrupoTpKey {
                    grupo_id: g_id,
                    grupo_nome: g_nome,
                    grupo_cor: g_cor,
                    timepoint_id: tp_id,
                    timepoint_rotulo: tp_rotulo,
                    timepoint_ordem: tp_ordem,
                },
                Vec::new()
            )
        });
        
        entry.1.push(limiar);
    }
    
    let mut stats_list = Vec::new();
    
    for ((_g_id, _tp_id), (key, limiares)) in grupos_map {
        let (n, media_geometrica_g, limite_inferior_g, limite_superior_g) = calcular_estatisticas_log(&limiares);
        if n == 0 {
            continue;
        }
        
        stats_list.push(GrupoTimepointStatsDto {
            grupo_id: key.grupo_id,
            grupo_nome: key.grupo_nome,
            grupo_cor: key.grupo_cor,
            timepoint_id: key.timepoint_id,
            timepoint_rotulo: key.timepoint_rotulo,
            timepoint_ordem: key.timepoint_ordem,
            n,
            media_geometrica_g,
            limite_inferior_g,
            limite_superior_g,
        });
    }
    
    stats_list.sort_by(|a, b| {
        match a.grupo_id.cmp(&b.grupo_id) {
            std::cmp::Ordering::Equal => a.timepoint_ordem.cmp(&b.timepoint_ordem),
            other => other,
        }
    });
    
    Ok(stats_list)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RespostaCruaDto {
    pub grupo: String,
    pub animal: String,
    pub timepoint: String,
    pub ordem: i64,
    pub filamento: f64,
    pub resposta: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LimiarExperimentoDto {
    pub grupo: String,
    pub animal: String,
    pub timepoint: String,
    pub filamento_inicial: f64,
    pub serie_respostas: String,
    pub limiar: Option<f64>,
}

/// Obtém todas as respostas cruas de testes de um experimento.
#[tauri::command]
pub fn obter_respostas_cruas_experimento(
    app_handle: tauri::AppHandle,
    experimento_id: i64,
) -> Result<Vec<RespostaCruaDto>, String> {
    let conn = obter_conexao(&app_handle)?;
    
    let mut stmt = conn.prepare(
        "SELECT \
            g.nome AS grupo, \
            a.marcacao AS animal, \
            t.rotulo AS timepoint, \
            r.ordem, \
            r.filamento_g AS filamento, \
            r.resposta \
         FROM respostas_sequencia r \
         JOIN sequencias_teste s ON r.sequencia_id = s.id \
         JOIN animais a ON s.animal_id = a.id \
         JOIN grupos g ON a.grupo_id = g.id \
         JOIN timepoints t ON s.timepoint_id = t.id \
         WHERE a.experimento_id = ?1 \
         ORDER BY g.nome, a.marcacao, t.ordem, r.ordem"
    ).map_err(|e| format!("Falha ao preparar consulta de respostas cruas: {}", e))?;

    let rows = stmt.query_map(params![experimento_id], |row| {
        Ok(RespostaCruaDto {
            grupo: row.get(0)?,
            animal: row.get(1)?,
            timepoint: row.get(2)?,
            ordem: row.get(3)?,
            filamento: row.get(4)?,
            resposta: row.get(5)?,
        })
    }).map_err(|e| format!("Erro ao executar consulta: {}", e))?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| format!("Erro ao carregar linha: {}", e))?);
    }
    Ok(result)
}

/// Obtém todos os limiares calculados de um experimento.
#[tauri::command]
pub fn obter_limiares_experimento(
    app_handle: tauri::AppHandle,
    experimento_id: i64,
) -> Result<Vec<LimiarExperimentoDto>, String> {
    let conn = obter_conexao(&app_handle)?;
    
    let mut stmt = conn.prepare(
        "SELECT \
            g.nome AS grupo, \
            a.marcacao AS animal, \
            t.rotulo AS timepoint, \
            s.filamento_inicial, \
            s.id AS sequencia_id, \
            s.limiar \
         FROM sequencias_teste s \
         JOIN animais a ON s.animal_id = a.id \
         JOIN grupos g ON a.grupo_id = g.id \
         JOIN timepoints t ON s.timepoint_id = t.id \
         WHERE a.experimento_id = ?1 AND s.status = 'concluida' \
         ORDER BY g.nome, a.marcacao, t.ordem"
    ).map_err(|e| format!("Falha ao preparar consulta de limiares: {}", e))?;

    let rows = stmt.query_map(params![experimento_id], |row| {
        let grupo: String = row.get(0)?;
        let animal: String = row.get(1)?;
        let timepoint: String = row.get(2)?;
        let filamento_inicial: f64 = row.get(3)?;
        let sequencia_id: i64 = row.get(4)?;
        let limiar: Option<f64> = row.get(5)?;
        Ok((grupo, animal, timepoint, filamento_inicial, sequencia_id, limiar))
    }).map_err(|e| format!("Erro ao executar consulta: {}", e))?;

    let mut result = Vec::new();
    for r_res in rows {
        let (grupo, animal, timepoint, filamento_inicial, seq_id, limiar) = 
            r_res.map_err(|e| format!("Erro ao carregar linha: {}", e))?;
            
        let mut resp_stmt = conn.prepare(
            "SELECT resposta FROM respostas_sequencia WHERE sequencia_id = ?1 ORDER BY ordem"
        ).map_err(|e| format!("Erro na subconsulta: {}", e))?;
        
        let resp_rows = resp_stmt.query_map(params![seq_id], |r| {
            let resp: String = r.get(0)?;
            Ok(resp)
        }).map_err(|e| format!("Erro na subconsulta execution: {}", e))?;
        
        let mut serie = String::new();
        for resp in resp_rows {
            if let Ok(val) = resp {
                serie.push_str(&val);
            }
        }
        
        result.push(LimiarExperimentoDto {
            grupo,
            animal,
            timepoint,
            filamento_inicial,
            serie_respostas: serie,
            limiar,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estatisticas_log_redondo() {
        // [1, 10, 100] -> logs = [0, 1, 2] -> media_log = 1 -> geometric_mean = 10
        let limiares = vec![1.0, 10.0, 100.0];
        let (n, media, inf, sup) = calcular_estatisticas_log(&limiares);
        
        assert_eq!(n, 3);
        assert!((media - 10.0).abs() < 1e-9);
        
        // n=3, log var = ((0-1)^2 + (1-1)^2 + (2-1)^2) / 2 = (1 + 0 + 1) / 2 = 1
        // log sd = sqrt(1) = 1
        // log se = 1 / sqrt(3) = 0.57735026919
        // inf = 10^(1 - 0.57735026919) = 10^0.42264973 = 2.646408
        // sup = 10^(1 + 0.57735026919) = 10^1.57735027 = 37.78809
        let expected_inf = 10.0f64.powf(1.0 - 1.0 / 3.0f64.sqrt());
        let expected_sup = 10.0f64.powf(1.0 + 1.0 / 3.0f64.sqrt());
        
        assert!(inf.is_some());
        assert!(sup.is_some());
        assert!((inf.unwrap() - expected_inf).abs() < 1e-6);
        assert!((sup.unwrap() - expected_sup).abs() < 1e-6);
    }

    #[test]
    fn test_estatisticas_n_um() {
        let limiares = vec![5.0];
        let (n, media, inf, sup) = calcular_estatisticas_log(&limiares);
        assert_eq!(n, 1);
        assert!((media - 5.0).abs() < 1e-9);
        assert!(inf.is_none());
        assert!(sup.is_none());
    }

    #[test]
    fn test_estatisticas_vazio() {
        let limiares = vec![];
        let (n, media, inf, sup) = calcular_estatisticas_log(&limiares);
        assert_eq!(n, 0);
        assert_eq!(media, 0.0);
        assert!(inf.is_none());
        assert!(sup.is_none());
    }

    // =========================================================================
    // FINALIZAÇÃO DA SEQUÊNCIA (bug do botão "não faz nada")
    // =========================================================================

    /// Banco em memória com kit (d conhecido), experimento, grupo, animal,
    /// timepoint e uma sequência em andamento pronta para receber respostas.
    fn banco_com_sequencia(d: f64, filamento_inicial: f64) -> (Connection, i64) {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE conjuntos_filamentos (id INTEGER PRIMARY KEY AUTOINCREMENT, nome TEXT NOT NULL,
                descricao TEXT, d REAL NOT NULL, ativo INTEGER NOT NULL DEFAULT 1,
                criado_em DATETIME DEFAULT CURRENT_TIMESTAMP, atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP);
             CREATE TABLE filamentos (id INTEGER PRIMARY KEY AUTOINCREMENT, conjunto_id INTEGER NOT NULL,
                forca_g REAL NOT NULL, ordem INTEGER NOT NULL);
             CREATE TABLE experimentos (id INTEGER PRIMARY KEY AUTOINCREMENT, nome TEXT NOT NULL, descricao TEXT,
                conjunto_id INTEGER NOT NULL, responsavel TEXT, ativo INTEGER NOT NULL DEFAULT 1,
                criado_em DATETIME DEFAULT CURRENT_TIMESTAMP, atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP);
             CREATE TABLE grupos (id INTEGER PRIMARY KEY AUTOINCREMENT, experimento_id INTEGER NOT NULL,
                nome TEXT NOT NULL, cor TEXT NOT NULL, ativo INTEGER NOT NULL DEFAULT 1,
                criado_em DATETIME DEFAULT CURRENT_TIMESTAMP, atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP);
             CREATE TABLE animais (id INTEGER PRIMARY KEY AUTOINCREMENT, experimento_id INTEGER NOT NULL,
                grupo_id INTEGER NOT NULL, marcacao TEXT NOT NULL, peso REAL, ativo INTEGER NOT NULL DEFAULT 1,
                criado_em DATETIME DEFAULT CURRENT_TIMESTAMP, atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP);
             CREATE TABLE timepoints (id INTEGER PRIMARY KEY AUTOINCREMENT, experimento_id INTEGER NOT NULL,
                rotulo TEXT NOT NULL, ordem INTEGER NOT NULL, opcional INTEGER NOT NULL DEFAULT 0,
                criado_em DATETIME DEFAULT CURRENT_TIMESTAMP, atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP);
             CREATE TABLE sequencias_teste (id INTEGER PRIMARY KEY AUTOINCREMENT, animal_id INTEGER NOT NULL,
                timepoint_id INTEGER NOT NULL, status TEXT NOT NULL, filamento_inicial REAL NOT NULL,
                limiar REAL, estimativa_log REAL, k_dixon REAL, d_usado REAL, n_nominal INTEGER,
                criado_em DATETIME DEFAULT CURRENT_TIMESTAMP, atualizado_em DATETIME DEFAULT CURRENT_TIMESTAMP);
             CREATE TABLE respostas_sequencia (id INTEGER PRIMARY KEY AUTOINCREMENT, sequencia_id INTEGER NOT NULL,
                ordem INTEGER NOT NULL, filamento_g REAL NOT NULL, resposta TEXT NOT NULL,
                criado_em DATETIME DEFAULT CURRENT_TIMESTAMP);",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO conjuntos_filamentos (nome, d, ativo) VALUES ('Kit Teste', ?1, 1)",
            params![d],
        ).unwrap();
        conn.execute("INSERT INTO experimentos (nome, conjunto_id, ativo) VALUES ('Exp', 1, 1)", []).unwrap();
        conn.execute("INSERT INTO grupos (experimento_id, nome, cor, ativo) VALUES (1, 'Controle', '#000', 1)", []).unwrap();
        conn.execute("INSERT INTO animais (experimento_id, grupo_id, marcacao, ativo) VALUES (1, 1, '1P', 1)", []).unwrap();
        conn.execute("INSERT INTO timepoints (experimento_id, rotulo, ordem) VALUES (1, 'basal 1', 0)", []).unwrap();
        conn.execute(
            "INSERT INTO sequencias_teste (animal_id, timepoint_id, status, filamento_inicial) VALUES (1, 1, 'em_andamento', ?1)",
            params![filamento_inicial],
        ).unwrap();
        let seq_id = conn.last_insert_rowid();
        (conn, seq_id)
    }

    fn gravar_respostas(conn: &Connection, seq_id: i64, pares: &[(f64, &str)]) {
        for (i, (fil, resp)) in pares.iter().enumerate() {
            conn.execute(
                "INSERT INTO respostas_sequencia (sequencia_id, ordem, filamento_g, resposta) VALUES (?1, ?2, ?3, ?4)",
                params![seq_id, i as i64, fil, resp],
            ).unwrap();
        }
    }

    /// SUCESSO: finaliza a série do exemplo do artigo (Figure 6) e confere que o
    /// limiar é calculado e persistido, e a sequência marcada como concluída.
    #[test]
    fn finalizar_sequencia_calcula_e_persiste_limiar() {
        // Série OXXOXO com doses 8,16,8,4,8,4 e d=0.301 => k=0.831, log=0.852
        let (mut conn, seq_id) = banco_com_sequencia(0.301, 8.0);
        gravar_respostas(&conn, seq_id, &[
            (8.0, "O"), (16.0, "X"), (8.0, "X"), (4.0, "O"), (8.0, "X"), (4.0, "O"),
        ]);

        let r = finalizar_sequencia_conn(&mut conn, seq_id).expect("deve finalizar");

        assert_eq!(r.n_nominal, 6);
        assert!((r.k - 0.831).abs() < 0.001, "k = {}", r.k);
        assert!((r.xf - 4.0).abs() < 1e-9, "xf = {}", r.xf);
        assert!((r.limiar - 10f64.powf(0.852)).abs() < 0.01, "limiar = {}", r.limiar);

        // Persistência: status concluída e limiar gravado.
        let (status, limiar): (String, Option<f64>) = conn
            .query_row("SELECT status, limiar FROM sequencias_teste WHERE id = ?1", params![seq_id],
                |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();
        assert_eq!(status, "concluida");
        assert!(limiar.is_some());
    }

    /// ERRO PROPAGADO: finalizar duas vezes deve falhar com mensagem clara
    /// (e não ser engolido silenciosamente).
    #[test]
    fn finalizar_sequencia_ja_finalizada_retorna_erro() {
        let (mut conn, seq_id) = banco_com_sequencia(0.301, 8.0);
        gravar_respostas(&conn, seq_id, &[
            (8.0, "O"), (16.0, "X"), (8.0, "X"), (4.0, "O"), (8.0, "X"), (4.0, "O"),
        ]);
        finalizar_sequencia_conn(&mut conn, seq_id).expect("primeira finalização ok");

        let segunda = finalizar_sequencia_conn(&mut conn, seq_id);
        assert!(segunda.is_err(), "a segunda finalização deve falhar");
        let msg = segunda.unwrap_err();
        assert!(msg.contains("já foi finalizada"), "mensagem pouco clara: {}", msg);
    }

    /// ERRO PROPAGADO: série sem nenhuma resposta.
    #[test]
    fn finalizar_sequencia_vazia_retorna_erro() {
        let (mut conn, seq_id) = banco_com_sequencia(0.301, 8.0);
        let r = finalizar_sequencia_conn(&mut conn, seq_id);
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("vazia"));
    }

    /// ERRO PROPAGADO: N nominal acima do que a Tabela 7 cobre (2..=6).
    /// Era exatamente este caso que o botão habilitado produzia — e que aparecia
    /// como "o botão não faz nada" porque o erro não era exibido.
    #[test]
    fn finalizar_sequencia_com_n_acima_de_6_retorna_erro_claro() {
        let (mut conn, seq_id) = banco_com_sequencia(0.301, 8.0);
        // "O" seguido de 6 respostas => segunda parte com 6 elementos => N = 7
        gravar_respostas(&conn, seq_id, &[
            (8.0, "O"), (16.0, "X"), (8.0, "O"), (16.0, "X"), (8.0, "O"), (16.0, "X"), (8.0, "O"),
        ]);
        let r = finalizar_sequencia_conn(&mut conn, seq_id);
        assert!(r.is_err(), "N acima de 6 deve falhar");
        let msg = r.unwrap_err();
        assert!(msg.contains("Dixon") || msg.contains("N nominal"), "mensagem: {}", msg);

        // E a sequência continua em andamento (nada foi corrompido).
        let status: String = conn
            .query_row("SELECT status FROM sequencias_teste WHERE id = ?1", params![seq_id], |r| r.get(0))
            .unwrap();
        assert_eq!(status, "em_andamento");
    }

    /// O botão só pode ser habilitado na faixa que a Tabela 7 cobre.
    #[test]
    fn pode_finalizar_respeita_faixa_da_tabela() {
        assert!(!pode_finalizar_agora(0), "sem reversão não finaliza");
        assert!(!pode_finalizar_agora(1));
        assert!(pode_finalizar_agora(2));
        assert!(pode_finalizar_agora(6));
        assert!(!pode_finalizar_agora(7), "N=7 está fora da Tabela 7");
        assert!(!pode_finalizar_agora(10));
    }
}
