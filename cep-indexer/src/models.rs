use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CepData {
    pub cep: String,
    pub logradouro: String,
    #[serde(default)]
    pub complemento: String,
    pub bairro: String,
    pub localidade: String,
    pub uf: String,
    pub ibge: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub cep: String,
    pub logradouro: String,
    pub complemento: String,
    pub bairro: String,
    pub localidade: String,
    pub uf: String,
    pub ibge: String,
}

impl From<CepData> for SearchResult {
    fn from(data: CepData) -> Self {
        SearchResult {
            cep: data.cep,
            logradouro: data.logradouro,
            complemento: data.complemento,
            bairro: data.bairro,
            localidade: data.localidade,
            uf: data.uf,
            ibge: data.ibge,
        }
    }
}
