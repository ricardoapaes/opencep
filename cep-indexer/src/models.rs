use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CepData {
    pub cep: String,
    pub logradouro: String,
    #[serde(default)]
    pub complemento: String,
    #[serde(default)]
    pub unidade: String,
    #[serde(default)]
    pub bairro: String,
    pub localidade: String,
    pub uf: String,
    #[serde(default)]
    pub estado: String,
    #[serde(default)]
    pub regiao: String,
    #[serde(default)]
    pub ibge: String,
    #[serde(default)]
    pub gia: String,
    #[serde(default)]
    pub ddd: String,
    #[serde(default)]
    pub siafi: String,
}

impl CepData {
    pub fn validate(&self) -> Result<()> {
        let cep = self.cep.replace('-', "");
        if cep.len() != 8 || !cep.bytes().all(|byte| byte.is_ascii_digit()) {
            bail!("invalid CEP: {}", self.cep);
        }
        if self.uf.len() != 2 || !self.uf.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            bail!("invalid UF for CEP {}: {}", self.cep, self.uf);
        }
        if self.localidade.trim().is_empty() {
            bail!("CEP {} has an empty locality", self.cep);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchResult {
    pub cep: String,
    pub logradouro: String,
    pub complemento: String,
    pub unidade: String,
    pub bairro: String,
    pub localidade: String,
    pub uf: String,
    pub estado: String,
    pub regiao: String,
    pub ibge: String,
    pub gia: String,
    pub ddd: String,
    pub siafi: String,
}

impl From<CepData> for SearchResult {
    fn from(data: CepData) -> Self {
        let clean_cep = data.cep.replace('-', "");
        let cep = if clean_cep.len() == 8 {
            format!("{}-{}", &clean_cep[..5], &clean_cep[5..])
        } else {
            data.cep
        };

        Self {
            cep,
            logradouro: data.logradouro,
            complemento: data.complemento,
            unidade: data.unidade,
            bairro: data.bairro,
            localidade: data.localidade,
            uf: data.uf,
            estado: data.estado,
            regiao: data.regiao,
            ibge: data.ibge,
            gia: data.gia,
            ddd: data.ddd,
            siafi: data.siafi,
        }
    }
}
