# OpenCEP API

API de consulta de CEPs brasileiros com cache local e fallback automático para ViaCEP. Compatível 100% com a API do ViaCEP.

## 🚀 Características

- **Cache Local**: Base de dados completa de CEPs brasileiros servida localmente para máxima performance
- **Fallback Inteligente**: Quando o CEP não está na base local, faz proxy automático para o ViaCEP
- **Pesquisa por Endereço**: Suporta pesquisa por UF/Cidade/Logradouro (proxy para ViaCEP)
- **100% Compatível**: Mesma API e formato de resposta do ViaCEP
- **Containerizado**: Deploy fácil com Docker/Podman
- **Leve e Rápido**: Nginx Alpine com base de dados estática

## 📋 Pré-requisitos

- Docker ou Podman
- Docker Compose

## 🔧 Instalação

1. Clone o repositório:

```bash
git clone https://github.com/ricardoapaes/opencep.git
cd opencep
```

2. Configure a versão da base de dados (opcional):

```bash
export OPENCEP_VERSION=2.0.1  # Versão padrão
```

3. Inicie o container:

```bash
docker compose up -d --build
```

A API estará disponível em `http://localhost:8080`

## 📖 Uso

### Consulta por CEP

**Endpoint**: `/ws/{cep}/{formato}/`

**Parâmetros**:

- `cep`: CEP com 8 dígitos (apenas números)
- `formato`: `json` ou `xml`

**Exemplos**:

```bash
# CEP de São Paulo
curl http://localhost:8080/ws/01001000/json/

# CEP do Rio Grande do Sul
curl http://localhost:8080/ws/87308084/json/
```

**Resposta**:

```json
{
  "cep": "01001-000",
  "logradouro": "Praça da Sé",
  "complemento": "lado ímpar",
  "bairro": "Sé",
  "localidade": "São Paulo",
  "uf": "SP",
  "ibge": "3550308",
  "gia": "1004",
  "ddd": "11",
  "siafi": "7107"
}
```

### Pesquisa por Endereço

**Endpoint**: `/ws/{UF}/{Cidade}/{Logradouro}/{formato}/`

**Parâmetros**:
- `UF`: Sigla do estado (2 letras maiúsculas)
- `Cidade`: Nome da cidade (mínimo 3 caracteres)
- `Logradouro`: Nome do logradouro (mínimo 3 caracteres)
- `formato`: `json` ou `xml`

**Exemplos**:

```bash
# Pesquisar por "Domingos" em Porto Alegre/RS
curl http://localhost:8080/ws/RS/Porto%20Alegre/Domingos/json/

# Pesquisar por "Domingos Jose" em Porto Alegre/RS
curl http://localhost:8080/ws/RS/Porto%20Alegre/Domingos%20Jose/json/

# Pesquisar por "Paulista" em São Paulo/SP
curl http://localhost:8080/ws/SP/São%20Paulo/Paulista/json/
```

**Resposta** (retorna até 50 CEPs):

```json
[
  {
    "cep": "90420-010",
    "logradouro": "Rua Domingos José Poli",
    "complemento": "",
    "bairro": "Auxiliadora",
    "localidade": "Porto Alegre",
    "uf": "RS",
    "ibge": "4314902",
    "gia": "",
    "ddd": "51",
    "siafi": "8801"
  }
]
```

### Rota Direta da Base Local

**Endpoint**: `/v1/{cep}.json`

Acessa diretamente a base de dados local sem fallback.

```bash
curl http://localhost:8080/v1/01001000.json
```

### Health Check

**Endpoint**: `/health`

Verifica se o serviço está funcionando.

```bash
curl http://localhost:8080/health
```

**Resposta**:

```json
{
  "status": "ok",
  "service": "opencep-api",
  "timestamp": "2025-10-16T10:30:45-03:00"
}
```

## 🏗️ Arquitetura

```
┌─────────────┐
│   Cliente   │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────┐
│          Nginx (Alpine)              │
│  ┌────────────────────────────────┐ │
│  │  /ws/{cep}/{formato}/          │ │
│  │  1. Try local: /v1/{cep}.json  │ │
│  │  2. Fallback: ViaCEP           │ │
│  └────────────────────────────────┘ │
│  ┌────────────────────────────────┐ │
│  │  /ws/{UF}/{Cidade}/{Log...}/   │ │
│  │  Proxy: ViaCEP                 │ │
│  └────────────────────────────────┘ │
└─────────────────────────────────────┘
       │                    │
       ▼                    ▼
┌──────────────┐    ┌──────────────┐
│ Base Local   │    │   ViaCEP     │
│ (JSON files) │    │   (Proxy)    │
└──────────────┘    └──────────────┘
```

## ⚙️ Configuração

### Variáveis de Ambiente

- `OPENCEP_VERSION`: Versão da base de dados OpenCEP (padrão: `2.0.1`)

### Portas

- `8080`: Porta HTTP exposta (configurável no `docker-compose.yml`)

### Personalização

Edite o `docker-compose.yml` para alterar configurações:

```yaml
services:
  opencep-api:
    build:
      args:
        - OPENCEP_VERSION=2.0.1  # Alterar versão da base
    ports:
      - "8080:80"  # Alterar porta externa
```

## 🔍 Logs

Visualizar logs em tempo real:

```bash
docker compose logs -f
```

Ver últimas 200 linhas:

```bash
docker compose logs -f --tail=200
```

## 🛠️ Desenvolvimento

### Estrutura do Projeto

```
opencep/
├── Dockerfile           # Multi-stage build: download base + nginx
├── docker-compose.yml   # Orquestração do container
├── nginx.conf          # Configuração das rotas e proxy
└── README.md           # Este arquivo
```

### Rebuild Completo

```bash
docker compose down
docker compose build --no-cache
docker compose up -d
```

### Testes

```bash
# Testar CEP local
curl -i http://localhost:8080/ws/01001000/json/

# Testar fallback (CEP inexistente na base local)
curl -i http://localhost:8080/ws/99999999/json/

# Testar pesquisa por endereço
curl -i http://localhost:8080/ws/RS/Porto%20Alegre/Domingos/json/
```

## 📦 Base de Dados

Este projeto utiliza a base de dados do [OpenCEP](https://github.com/SeuAliado/OpenCEP), que é baixada automaticamente durante o build da imagem.

**Versões disponíveis**: Veja as [releases do OpenCEP](https://github.com/SeuAliado/OpenCEP/releases)

## 🤝 Créditos

- **Base de Dados**: [OpenCEP](https://github.com/SeuAliado/OpenCEP) por SeuAliado
- **API de Fallback**: [ViaCEP](https://viacep.com.br/)

## 📄 Licença

Este projeto é fornecido "como está", sem garantias. Use por sua conta e risco.

## 🐛 Problemas Conhecidos

- A pesquisa por endereço sempre usa o ViaCEP (não há base local para essa funcionalidade)
- CEPs novos não presentes na base local serão consultados via ViaCEP

## 💡 Roadmap

- [ ] Atualização automática da base de dados
- [ ] Suporte a HTTPS
- [ ] Métricas e monitoramento
- [ ] Cache de respostas do ViaCEP
- [x] Health check endpoint

## 📞 Contato

Para dúvidas ou sugestões, abra uma [issue](https://github.com/ricardoapaes/opencep/issues).

---

**Desenvolvido com ❤️ usando Nginx + OpenCEP + ViaCEP**
