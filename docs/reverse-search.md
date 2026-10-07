# Busca reversa de CEP

## Decisão

A busca reversa usa Rust, Axum e um índice Tantivy local. A base é estática e o
índice é produzido no build, publicado junto da mesma versão do OpenCEP e aberto
somente para leitura pelo servidor.

O protótipo anterior usava SQLite com `LIKE '%termo%'`. Esse padrão não aproveita
adequadamente índices B-tree e não oferece ranking ou tolerância a erros. Tantivy
foi escolhido para manter um único processo leve e fornecer recursos próprios de
busca textual, sem operar outro daemon.

## Fluxo

```text
v1/*.json
    -> indexer Rust
    -> valida registros e ignora CEPs gerais sem logradouro
    -> normaliza cidade e logradouro
    -> gera índice em diretório temporário
    -> valida contagem e metadados
    -> publica um novo diretório imutável atomicamente

Cliente
    -> Nginx
       -> consulta direta por CEP: arquivos JSON
       -> busca JSON por endereço: search-server:3000
       -> XML ou falha do backend: ViaCEP
```

Uma busca local válida sem resultados retorna `200 []`. Ela não aciona o ViaCEP.
O fallback externo existe apenas para indisponibilidade técnica do backend e para
XML, que ainda não é serializado localmente.

## Normalização e ranking

- decomposição Unicode e remoção de marcas diacríticas;
- minúsculas, pontuação transformada em separadores e espaços consolidados;
- aliases de `Av.`, `R.`, `Rod.`, `Trav.`, `Al.` e `Estr.`;
- UF como filtro obrigatório exato;
- termos exatos recebem peso maior;
- termos de quatro a sete caracteres aceitam distância de edição 1;
- termos maiores aceitam distância de edição 2;
- todos os termos informados para cidade e logradouro devem participar do match.

## Contrato HTTP

```text
GET /ws/{UF}/{Cidade}/{Logradouro}/json/
GET /ws/{UF}/{Cidade}/{Logradouro}/json?limit=10
```

- UF: exatamente duas letras;
- cidade e logradouro: mínimo de três caracteres normalizados;
- limite padrão: 50;
- limite máximo: 100;
- parâmetros inválidos: HTTP 400;
- falha interna: HTTP 500;
- capacidade de busca ocupada: HTTP 429;
- nenhum resultado: HTTP 200 com `[]`.

`/health` confirma que o processo está vivo. `/ready` só existe após o índice ter
sido aberto e informa versão do dataset, schema e quantidade de documentos.

## Artefato do índice

Variáveis do indexador:

- `JSON_DIR`: diretório dos JSONs, padrão `v1`;
- `INDEX_PATH`: diretório de destino, padrão `cep_index`;
- `OPENCEP_VERSION`: versão registrada nos metadados.

Variáveis do servidor:

- `INDEX_PATH`: diretório do índice, padrão `cep_index`;
- `PORT`: porta HTTP, padrão `3000`.
- `SEARCH_CONCURRENCY`: buscas simultâneas, padrão `16`.

`opencep-meta.json` registra:

- versão do schema;
- versão do dataset;
- quantidade de documentos;
- quantidade de CEPs gerais ignorados por não possuírem logradouro;
- instante de geração.

Um `INDEX_PATH` existente nunca é substituído. Cada versão deve usar um novo
caminho/artefato, preservando a versão publicada em caso de falha.

O servidor falha ao iniciar se os metadados estiverem ausentes, o schema for
incompatível ou a contagem divergir do índice.

## Docker Compose

O Compose inicia dois serviços na mesma rede:

- `opencep-api`: Nginx público na porta 8080;
- `cep-search`: backend interno, sem porta publicada no host.

O Nginx aguarda o `/ready` do backend. No Compose, `NGINX_DNS_RESOLVER` usa o DNS
interno `127.0.0.11`.

```bash
docker compose build
docker compose up -d
curl http://localhost:8080/ready
curl 'http://localhost:8080/ws/SP/Sao%20Paulo/Paulsta/json/?limit=10'
```

## Verificação

Os testes automatizados cobrem:

- CLI gerando índice e metadados;
- rejeição de dados inválidos sem índice parcial;
- busca sem acentos;
- erro de digitação;
- validação HTTP;
- readiness;
- resultado vazio sem chamada externa.

Antes de publicar uma nova base completa, ainda devem ser registrados tamanho do
artefato, duração de indexação, RSS e latências p50/p95/p99 no ambiente alvo.
