# CEP Indexer - busca reversa

Indexador e API HTTP em Rust para localizar CEPs por UF, cidade e logradouro. Os
JSONs do OpenCEP são transformados em um índice Tantivy imutável durante o build.

## Características

- busca sem diferença entre maiúsculas, minúsculas e acentos;
- aliases para tipos comuns de logradouro, como `Av.` e `Avenida`;
- tolerância a um ou dois erros de edição conforme o tamanho do termo;
- filtro exato por UF e ranking por relevância;
- índice publicado atomicamente e validado antes do uso;
- respostas JSON compatíveis com os campos conhecidos do ViaCEP;
- liveness separado de readiness.

## Requisitos

- Rust 1.89;
- diretório com os arquivos JSON do OpenCEP.

O `Cargo.lock` é versionado. Use `--locked` em builds e CI.

## Gerar o índice

```bash
JSON_DIR=/path/to/v1 \
INDEX_PATH=/tmp/cep_index \
OPENCEP_VERSION=2.0.1 \
cargo run --locked --release --bin indexer
```

O diretório publicado contém o índice Tantivy e `opencep-meta.json`, com versão
do dataset, versão do schema, quantidades indexada/ignorada e data de geração.
CEPs gerais sem logradouro são ignorados; qualquer outro registro inválido
interrompe o processo. O destino é imutável: para uma nova versão, informe outro
`INDEX_PATH`.

## Iniciar a API

```bash
INDEX_PATH=/tmp/cep_index PORT=3000 \
cargo run --locked --release --bin search-server
```

`SEARCH_CONCURRENCY` limita buscas CPU-bound simultâneas (padrão: 16). Quando a
capacidade está ocupada, novas solicitações recebem HTTP 429 em vez de formar uma
fila sem limite.

Endpoints:

```text
GET /health
GET /ready
GET /ws/{UF}/{Cidade}/{Logradouro}/json[/?limit=50]
GET /search/{UF}/{Cidade}/{Logradouro}[?limit=50]
```

UF deve possuir duas letras. Cidade e logradouro devem possuir pelo menos três
caracteres depois da normalização. O limite padrão é 50 e o máximo é 100.
Resultado inexistente é representado por `200 []` e não depende de serviço
externo.

O segmento de logradouro também aceita um número depois de vírgula:

```text
GET /ws/PR/Campo%20Mourão/Rua%20Quinto%20Salvadori,1774/json/
```

O número é removido antes da busca textual e aplicado aos candidatos conforme a
faixa presente em `complemento`. São aceitos limites inferiores/superiores,
faixas abertas com `ao fim`, pares ímpar/par separados por `/` e indicação de
`lado ímpar` ou `lado par`. Para proteger o serviço, uma busca numerada que
corresponda a mais de 10.000 endereços recebe HTTP 422 e deve ser refinada com um
logradouro mais específico.

## Testes

No Dev Container do projeto:

```bash
cargo test --locked --manifest-path cep-indexer/Cargo.toml --lib
cargo test --locked --manifest-path cep-indexer/Cargo.toml \
  --test indexer_cli --test search_api
cargo clippy --locked --manifest-path cep-indexer/Cargo.toml \
  --all-targets -- -D warnings

cargo test --locked --release --manifest-path cep-indexer/Cargo.toml \
  --test search_benchmark -- --ignored --nocapture
```

Os testes de integração usam somente fixtures locais e não chamam o ViaCEP.

## Limites conhecidos

- XML continua sendo atendido pelo fallback ViaCEP no Nginx.
- Latência, memória e tamanho do índice da base completa devem ser medidos no
  hardware de produção; o projeto não declara números sem benchmark.
- Autocomplete ainda não faz parte do contrato.
