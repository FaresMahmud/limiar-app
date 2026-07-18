mod dixon;
mod dixon_table;

use dixon::{estimar_limiar, Resposta};
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_sql::Builder::default().build())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![calcular_limiar])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
