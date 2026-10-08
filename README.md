# OpenCEP API

API de consulta de CEPs brasileiros com cache local e fallback automático para ViaCEP. Compatível 100% com a API do ViaCEP.

## 🚀 Características

- **Cache Local**: Base de dados completa de CEPs brasileiros servida localmente para máxima performance
- **Busca Reversa Local**: Pesquisa por endereço usando índice Tantivy embutido no serviço Rust
- **Fallback Inteligente**: Quando o CEP não está na base local, faz proxy automático para o ViaCEP
- **100% Compatível**: Mesma API e formato de resposta do ViaCEP
- **Busca Tolerante**: Normalização de acentos, abreviações e pequenos erros de digitação
- **Containerizado**: Deploy fácil com Docker/Podman
- **Leve e Rápido**: Nginx Alpine com base de dados estática + servidor Rust otimizado

## 📋 Pré-requisitos

- Docker com o plugin Compose; ou Podman com `podman compose`/`podman-compose`

Os exemplos usam `docker`. Em uma instalação Podman nativa, substitua por
`podman` quando o provider Compose estiver configurado.

## 🔧 Instalação

### Usando apenas a imagem Nginx

```bash
# Usar imagem do GitHub Container Registry
docker run -d -p 8080:80 ghcr.io/ricardoapaes/opencep:latest
```

A imagem Nginx isolada atende consultas diretas por CEP. Para busca reversa local,
use o Compose do repositório, que também constrói e inicia `cep-search`.

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

> **Busca Local**: usa um índice Tantivy local no serviço Rust, sem depender do
> ViaCEP para buscas válidas. O fallback externo ocorre somente se o backend
> estiver indisponível; ausência de resultados retorna `[]`.

**Parâmetros**:
- `UF`: Sigla do estado (2 letras maiúsculas)
- `Cidade`: Nome da cidade (mínimo 3 caracteres)
- `Logradouro`: Nome do logradouro (mínimo 3 caracteres)
- `formato`: `json`; XML continua sendo encaminhado ao ViaCEP

**Exemplos**:

```bash
# Pesquisar por "Domingos" em Porto Alegre/RS
curl http://localhost:8080/ws/RS/Porto%20Alegre/Domingos/json/

# Pesquisar por "Domingos Jose" em Porto Alegre/RS
curl http://localhost:8080/ws/RS/Porto%20Alegre/Domingos%20Jose/json/

# Pesquisar por "Paulista" em São Paulo/SP
curl http://localhost:8080/ws/SP/São%20Paulo/Paulista/json/

# Com limite de resultados (padrão: 50, máximo: 100)
curl "http://localhost:8080/ws/SP/São%20Paulo/Paulista/json/?limit=10"
```

**Resposta** (retorna até 50 CEPs por padrão):

```json
[
  {
    "cep": "01310-100",
    "logradouro": "Avenida Paulista",
    "complemento": "lado ímpar",
    "bairro": "Bela Vista",
    "localidade": "São Paulo",
    "uf": "SP",
    "ibge": "3550308"
  },
  {
    "cep": "01310-200",
    "logradouro": "Avenida Paulista",
    "complemento": "lado par",
    "bairro": "Bela Vista",
    "localidade": "São Paulo",
    "uf": "SP",
    "ibge": "3550308"
  }
]
```

**Recursos da Busca Local**:
- ✅ Normalização automática (remove acentos)
- ✅ Tolerância a pequenos erros de digitação
- ✅ Ranking textual com filtro exato por UF
- ✅ Limite configurável de resultados
- ✅ Resultado vazio independente de serviço externo

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
┌───────────────────────────────────────────────┐
│          Nginx (Alpine) - Port 80/8080         │
│  ┌─────────────────────────────────────────┐  │
│  │  /ws/{cep}/{formato}/                   │  │
│  │  1. Try local: /v1/{cep}.json           │  │
│  │  2. Fallback: ViaCEP                    │  │
│  └─────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────┐  │
│  │  /ws/{UF}/{Cidade}/{Logradouro}/{fmt}/  │  │
│  │  1. Proxy: Rust Search Server (Tantivy) │──┼──┐
│  │  2. Fallback técnico: ViaCEP            │  │  │
│  └─────────────────────────────────────────┘  │  │
└───────────────┬───────────────────────────────┘  │
                │                                   │
                ▼                                   ▼
        ┌──────────────┐              ┌────────────────────┐
        │   ViaCEP     │              │  Rust Search       │
        │   (Proxy)    │              │  Server (Port 3000)│
        └──────────────┘              │  ┌──────────────┐  │
                                      │  │ Tantivy Index│  │
        ┌──────────────┐              │  │  read-only   │  │
        │ Base Local   │              │  │  Indexed     │  │
        │ (JSON files) │              │  └──────────────┘  │
        └──────────────┘              └────────────────────┘
```

**Componentes**:
- **Nginx**: Proxy reverso e servir arquivos estáticos
- **Rust Search Server**: API Axum de busca reversa
- **Tantivy Index**: índice textual versionado, validado e aberto para leitura
- **Base Local JSON**: arquivos individuais fornecidos pelas releases do OpenCEP
- **ViaCEP**: fallback para CEP direto, XML e falha técnica da busca reversa

## ⚙️ Configuração

### Variáveis de Ambiente

- `OPENCEP_VERSION`: Versão da base de dados OpenCEP (padrão: `2.0.1`)
- `NGINX_DNS_RESOLVER`: endereço do servidor DNS usado pelo Nginx. A imagem isolada usa `1.1.1.1`; o Compose detecta automaticamente os resolvers de Docker ou Podman pelo `/etc/resolv.conf`. Em ECS EC2, a task pode definir `169.254.169.253` pela variável `nginx_dns_resolver` da IaC.

O arquivo `default.conf.template` é renderizado pelo entrypoint oficial da imagem Nginx em `/etc/nginx/conf.d/default.conf` a cada inicialização. O script `16-opencep-resolver.envsh` usa o resolver explícito ou o detectado pelo entrypoint. O filtro de substituição preserva variáveis de rota do Nginx como `$request_uri`, `$viacep_host` e capturas.

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
├── Dockerfile              # Multi-stage: download + Rust build + indexação + runtime
├── docker-compose.yml      # 2 serviços: nginx + rust search server
├── default.conf.template   # Rotas e proxy; DNS renderizado no startup
├── .devcontainer/          # Rust 1.89, Nginx e ferramentas de validação
├── build.sh               # Script helper para builds (deprecated)
├── build-all.sh           # 🆕 Script completo com busca reversa
├── cep-indexer/           # 🆕 Código Rust
│   ├── Cargo.toml         # Dependências (Axum, Tantivy etc.)
│   ├── Cargo.lock         # Dependências reproduzíveis
│   ├── src/
│   │   ├── models.rs      # Estruturas de dados CEP
│   │   ├── indexer.rs     # CLI de geração do índice
│   │   ├── lib.rs         # Indexação, busca e API
│   │   └── server.rs      # Servidor HTTP de busca
│   └── README.md          # Documentação específica do Rust
├── docs/
│   ├── reverse-search.md  # 🆕 Documentação detalhada busca reversa
│   ├── aws-ecs-deploy.md  # Deploy AWS ECS
│   └── github-actions-ci.md
└── README.md              # Este arquivo
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

O Dev Container usa Rust 1.89 sobre Debian e inclui Nginx e ferramentas para os
testes locais. Não monta Docker socket, credenciais ou secrets e não inicia
serviços automaticamente.

```bash
devcontainer exec --workspace-folder . \
  cargo test --locked --manifest-path cep-indexer/Cargo.toml \
  --test indexer_cli --test search_api
```

Para testar rapidamente a integração Docker sem baixar a base completa:

```bash
docker build --target search-server-test -t opencep-search:test .
docker build --target nginx-server-test -t opencep-nginx:test .

docker compose -f docker-compose.yml -f docker-compose.ci.yml \
  up -d --no-build --wait --wait-timeout 60

curl -fsS http://localhost:8080/ready
curl -fsS \
  'http://localhost:8080/ws/SP/Sao%20Paulo/Paulsta/json/?limit=10'

docker compose -f docker-compose.yml -f docker-compose.ci.yml down
```

Esse fluxo usa quatro arquivos JSON de fixture, não acessa o ViaCEP e valida a
comunicação Nginx → Rust → índice Tantivy. Para testar a base completa, execute
`docker compose up -d --build --wait --wait-timeout 300`; o primeiro build baixa
e indexa a release configurada em `OPENCEP_VERSION`.

Se o Compose não ficar saudável, execute:

```bash
docker compose -f docker-compose.yml -f docker-compose.ci.yml ps -a
docker compose -f docker-compose.yml -f docker-compose.ci.yml \
  logs --no-color cep-search opencep-api
```

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

- A base completa ainda precisa de benchmark versionado antes de publicar metas de latência e memória.
- CEPs novos não presentes na base local serão consultados via ViaCEP

## 💡 Roadmap

- [x] **Busca reversa local** (Rust + Tantivy)
- [ ] Cache em memória (Redis) para queries frequentes
- [ ] Métricas e observabilidade (Prometheus)
- [ ] Atualização automática da base de dados
- [ ] Suporte a HTTPS
- [x] Health check endpoint
- [x] Fuzzy search (distância de edição por termo)
- [ ] Autocomplete de cidades/ruas

## 📚 Documentação Adicional

- [📖 Busca Reversa - Documentação Técnica](docs/reverse-search.md)
- [🚀 Deploy AWS ECS](docs/aws-ecs-deploy.md)
- [🔧 GitHub Actions CI/CD](docs/github-actions-ci.md)
- [⚙️ Código Rust - Indexador](cep-indexer/README.md)

## 📞 Contato

Para dúvidas ou sugestões, abra uma [issue](https://github.com/ricardoapaes/opencep/issues).

---

**Desenvolvido com ❤️ usando Nginx + OpenCEP + ViaCEP**
