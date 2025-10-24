# GitHub Actions CI/CD - Resumo

## O que foi implementado

### Workflow CI/CD (`.github/workflows/ci.yml`)

Pipeline completo que executa em:
- ✅ Push para `main`
- ✅ Pull Requests para `main`

### Etapas do Pipeline

#### 1. Build da Imagem Docker
- Usa BuildX para build otimizado
- Cache do GitHub Actions para builds mais rápidos

#### 2. Testes Automatizados

Executa 6 testes principais:

1. **Health Check** (`/health`)
   - Verifica se retorna `{"status":"ok"}`

2. **Busca por CEP - Cache Local** (`/ws/01001000/json/`)
   - Valida se retorna dados do CEP da base local

3. **Busca por CEP - Fallback ViaCEP** (`/ws/99999999/json/`)
   - Verifica se CEP inexistente retorna `{"erro":"true"}`

4. **Pesquisa por Endereço** (`/ws/RS/Porto Alegre/Domingos/json`)
   - Testa proxy para ViaCEP
   - Valida se retorna array JSON

5. **Acesso Direto v1** (`/v1/01001000.json`)
   - Verifica acesso direto aos arquivos JSON

6. **Página Inicial** (`/`)
   - Valida se HTML de documentação está servindo

#### 3. Publicação no GitHub Container Registry

**Tags automáticas:**

| Evento | Tag | Exemplo |
|--------|-----|---------|
| Pull Request | `pr-{número}` | `ghcr.io/ricardoapaes/opencep:pr-10` |
| Push para main | `latest` | `ghcr.io/ricardoapaes/opencep:latest` |
| Qualquer branch | `{branch}-{sha}` | `ghcr.io/ricardoapaes/opencep:main-abc1234` |

## Como usar

### 1. Testar localmente antes do push

```bash
# Executar os mesmos testes do CI
docker build -t opencep-api:test .
docker run -d --name opencep-test -p 8080:80 opencep-api:test
sleep 5

# Health check
curl -s http://localhost:8080/health | grep '"status":"ok"' && echo "✅ PASS" || echo "❌ FAIL"

# CEP local
curl -s http://localhost:8080/ws/01001000/json/ | grep '"cep"' && echo "✅ PASS" || echo "❌ FAIL"

# Fallback ViaCEP
curl -s http://localhost:8080/ws/99999999/json/ | grep '"erro"' && echo "✅ PASS" || echo "❌ FAIL"

# Pesquisa por endereço
curl -s http://localhost:8080/ws/RS/Porto%20Alegre/Domingos/json | grep -E '\[|"cep"' && echo "✅ PASS" || echo "❌ FAIL"

# Cleanup
docker rm -f opencep-test
```

### 2. Fazer push e aguardar CI

```bash
git add .
git commit -m "feat: nova funcionalidade"
git push origin main
```

O GitHub Actions irá:
1. Executar todos os testes
2. Se passar, publicar imagem em `ghcr.io/ricardoapaes/opencep:latest`

### 3. Usar imagem em produção

```bash
# Pull da imagem publicada
docker pull ghcr.io/ricardoapaes/opencep:latest

# Executar
docker run -d -p 8080:80 ghcr.io/ricardoapaes/opencep:latest
```

### 4. Testar Pull Request antes de fazer merge

```bash
# Criar PR #10, aguardar CI passar
# Depois testar a imagem da PR:
docker pull ghcr.io/ricardoapaes/opencep:pr-10
docker run -d -p 8080:80 ghcr.io/ricardoapaes/opencep:pr-10

# Se tudo OK, fazer merge
```

## Permissões necessárias

O workflow já está configurado com as permissões corretas:

```yaml
permissions:
  contents: read      # Ler código do repositório
  packages: write     # Escrever no GitHub Container Registry
```

**Importante**: O `GITHUB_TOKEN` é automaticamente fornecido pelo GitHub Actions, não precisa configurar nada manualmente.

## Visualizar execuções

1. Acesse: https://github.com/ricardoapaes/opencep/actions
2. Clique no workflow "CI/CD - OpenCEP API"
3. Veja os logs de cada execução

## Badges

Adicione ao README para mostrar status do CI:

```markdown
[![CI/CD](https://github.com/ricardoapaes/opencep/actions/workflows/ci.yml/badge.svg)](https://github.com/ricardoapaes/opencep/actions/workflows/ci.yml)
```

## Troubleshooting

### Testes falhando localmente mas passando no CI

- Verifique se está usando a mesma versão do Docker
- Certifique-se que portas estão liberadas (8080)

### Erro ao publicar no ghcr.io

- Verifique se o repositório permite packages públicos
- Vá em Settings → Actions → General → Workflow permissions
- Selecione "Read and write permissions"

### Cache não está funcionando

O cache é gerenciado automaticamente pelo GitHub Actions com `cache-from` e `cache-to`.

Para limpar cache:
1. Actions → Caches
2. Delete caches antigos

## Próximos passos (opcional)

### 1. Deploy automático no ECS

Adicionar step no workflow:

```yaml
- name: Deploy to AWS ECS
  if: github.ref == 'refs/heads/main'
  run: |
    aws ecs update-service \
      --cluster opencep-cluster \
      --service opencep-api \
      --force-new-deployment
  env:
    AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
    AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
    AWS_REGION: us-east-1
```

### 2. Notificações no Slack/Discord

```yaml
- name: Notify on success
  if: success()
  uses: slackapi/slack-github-action@v1
  with:
    webhook-url: ${{ secrets.SLACK_WEBHOOK }}
    payload: |
      {
        "text": "✅ OpenCEP API deployed successfully!"
      }
```

### 3. Scan de segurança

```yaml
- name: Run Trivy vulnerability scanner
  uses: aquasecurity/trivy-action@master
  with:
    image-ref: opencep-api:test
    format: 'sarif'
    output: 'trivy-results.sarif'
```

## Arquivos relacionados

- `.github/workflows/ci.yml` - Definição do workflow
- `.dockerignore` - Arquivos ignorados no build
- `Dockerfile` - Definição da imagem
- `nginx.conf` - Configuração do Nginx
- `README.md` - Documentação principal
- `DEPLOY.md` - Guia de deploy
- `docs/aws-ecs-deploy.md` - Deploy específico para AWS ECS
