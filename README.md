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

### Usando imagem pré-construída (Recomendado)

```bash
# Usar imagem do GitHub Container Registry
docker run -d -p 8080:80 ghcr.io/ricardoapaes/opencep:latest
```

Ou com Docker Compose:

```yaml
# docker-compose.yml
services:
  opencep-api:
    image: ghcr.io/ricardoapaes/opencep:latest
    ports:
      - "8080:80"
    restart: unless-stopped
```

```bash
docker compose up -d
```

### Build local

1. Clone o repositório:

```bash
git clone https://github.com/ricardoapaes/opencep.git
cd opencep
```

2. Configure o ambiente (opcional):

```bash
# Copiar arquivo de exemplo
cp .env.example .env

# Editar se necessário
# OPENCEP_VERSION=2.0.1
# DOCKER_BUILDKIT=1  # Habilita cache otimizado
```

3. Inicie o container:

```bash
# Opção 1: Usando o script helper (recomendado)
./build.sh
docker compose up -d

# Opção 2: Manual com BuildKit habilitado
export DOCKER_BUILDKIT=1
export COMPOSE_DOCKER_CLI_BUILD=1

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
- `NGINX_DNS_RESOLVER`: endereço do servidor DNS usado pelo Nginx para resolver o ViaCEP. A imagem usa `1.1.1.1` por padrão (requer saída DNS para esse endereço); o Compose usa `127.0.0.11`, DNS interno da rede Docker. Em ECS EC2, a task define `169.254.169.253` via variável `nginx_dns_resolver` na IaC. Configure um endereço alcançável pela sua hospedagem antes de publicar a imagem em outra rede. Não configure uma lista mista de servidores com alcançabilidade diferente.

O arquivo `default.conf.template` é renderizado pelo entrypoint oficial da imagem Nginx em `/etc/nginx/conf.d/default.conf` a cada inicialização. O filtro `NGINX_ENVSUBST_FILTER=^NGINX_DNS_RESOLVER$` substitui **somente** o resolver; variáveis de rota do Nginx como `$request_uri`, `$viacep_host` e capturas continuam intactas. Não monte um arquivo diretamente sobre a configuração gerada.

Para usar outro DNS, passe `-e NGINX_DNS_RESOLVER=<IP_DNS>` no `docker run`, defina `NGINX_DNS_RESOLVER` no ambiente do Compose ou altere a variável de entrada da IaC para ECS. O valor deve ser um endereço DNS aceito pela diretiva `resolver` do Nginx e acessível do contêiner (não um IP de DNS exclusivo da AWS fora dela).

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

## 🚀 CI/CD

O projeto possui workflow automatizado no GitHub Actions que:

1. **Testa automaticamente** em cada push/PR:
   - Renderização do DNS padrão e alternativo, preservação das variáveis Nginx e `nginx -t`
   - Resolução de DNS e endpoints de proxy em rede Docker com resolver alternativo
   - Health check endpoint
   - Consulta por CEP (cache local)
   - Fallback para ViaCEP
   - Pesquisa por endereço
   - Acesso direto à base local
   - Página inicial

2. **Publica imagem no GitHub Container Registry**:
   - **Pull Requests**: `ghcr.io/ricardoapaes/opencep:pr-{número}`
   - **Branch main**: `ghcr.io/ricardoapaes/opencep:latest`

### Usar imagem de uma Pull Request

```bash
# Exemplo: testar imagem da PR #10
docker pull ghcr.io/ricardoapaes/opencep:pr-10
docker run -d -p 8080:80 ghcr.io/ricardoapaes/opencep:pr-10
```

### Ver status do CI

[![CI/CD](https://github.com/ricardoapaes/opencep/actions/workflows/ci.yml/badge.svg)](https://github.com/ricardoapaes/opencep/actions/workflows/ci.yml)

## 🛠️ Desenvolvimento

### Estrutura do Projeto

```
opencep/
├── Dockerfile           # Multi-stage build: download base + nginx
├── docker-compose.yml   # Orquestração do container
├── default.conf.template # Rotas e proxy; DNS renderizado no startup
├── .devcontainer/      # Ambiente mínimo para inspecionar/testar configuração Nginx
├── build.sh            # Script helper para builds otimizados
├── .env.example        # Exemplo de variáveis de ambiente
└── README.md           # Este arquivo
```

### Rebuild Completo

```bash
docker compose down
docker compose build --no-cache
docker compose up -d
```

### Otimização de Build

O projeto usa **BuildKit cache mounts** para evitar downloads repetidos da base de dados:

**Performance:**

- 🐌 **Primeiro build**: Download completo (~500MB) - ~2-3 minutos
- 🚀 **Builds subsequentes**: Usa arquivo em cache - ~10-20 segundos!
- 💾 **Cache persistente**: Arquivos mantidos entre builds

**Como funciona:**

```
Primeiro build:  Download 500MB → Extrai → Build imagem
                 ⏱️  ~2-3 minutos

Segundo build:   Cache hit! → Extrai → Build imagem
                 ⏱️  ~10-20 segundos (15x mais rápido!)
```

Para habilitar o BuildKit (recomendado):

```bash
export DOCKER_BUILDKIT=1
export COMPOSE_DOCKER_CLI_BUILD=1
docker compose build
```

Ou use o script helper:

```bash
./build.sh
```

### Testes

O Dev Container usa `nginx:alpine` sem Docker socket, instalação automática ou acesso à AWS. Nele, após autorização para iniciá-lo, pode-se renderizar manualmente o template com `envsubst` e validar com `nginx -t`; testes de build da imagem, DNS da rede Docker e endpoints ficam na CI, pois exigem lifecycle de contêineres. A configuração roda como root porque o Nginx precisa escrever PID/logs durante `nginx -t`; não há montagem de credenciais nem comandos de inicialização do projeto. Iniciar/reconstruir o Dev Container exige autorização separada.

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
