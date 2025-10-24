#!/bin/bash
# Script de teste rápido para a API OpenCEP com busca reversa

BASE_URL="${BASE_URL:-http://localhost:8080}"
SEARCH_URL="${SEARCH_URL:-http://localhost:3000}"

echo "🧪 Testes da API OpenCEP"
echo "========================"
echo ""

# Cores para output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

test_endpoint() {
    local name="$1"
    local url="$2"
    local expected_status="${3:-200}"
    
    echo -n "Testing $name... "
    status=$(curl -s -o /dev/null -w "%{http_code}" "$url")
    
    if [ "$status" -eq "$expected_status" ]; then
        echo -e "${GREEN}✓${NC} (HTTP $status)"
        return 0
    else
        echo -e "${RED}✗${NC} (HTTP $status, expected $expected_status)"
        return 1
    fi
}

echo "📡 Testando serviços..."
echo ""

# Health checks
test_endpoint "Nginx Health" "$BASE_URL/health"
test_endpoint "Search Server Health" "$SEARCH_URL/health"

echo ""
echo "🔍 Testando busca por CEP..."
echo ""

# Busca por CEP (cache local)
test_endpoint "CEP São Paulo (01001-000)" "$BASE_URL/ws/01001000/json/"
test_endpoint "CEP Porto Alegre (87308-084)" "$BASE_URL/ws/87308084/json/"

echo ""
echo "🗺️  Testando busca reversa por endereço..."
echo ""

# Busca reversa (novo!)
test_endpoint "Paulista em SP" "$BASE_URL/ws/SP/São%20Paulo/Paulista/json/"
test_endpoint "Domingos em Porto Alegre" "$BASE_URL/ws/RS/Porto%20Alegre/Domingos/json/"
test_endpoint "Ipiranga em SP" "$BASE_URL/ws/SP/São%20Paulo/Ipiranga/json/"

echo ""
echo "🔄 Testando fallback ViaCEP..."
echo ""

# CEP inexistente (deve fazer fallback para ViaCEP e retornar erro)
test_endpoint "CEP inválido (fallback)" "$BASE_URL/ws/99999999/json/" "200"

echo ""
echo "📊 Testando limites de busca..."
echo ""

# Com parâmetro limit
test_endpoint "Busca com limite" "$BASE_URL/ws/SP/São%20Paulo/Rua/json/?limit=5"

echo ""
echo "✅ Testes concluídos!"
echo ""

# Exemplos de output JSON
echo "📄 Exemplos de resposta:"
echo ""
echo -e "${YELLOW}1. Busca por CEP:${NC}"
curl -s "$BASE_URL/ws/01001000/json/" | jq '.' 2>/dev/null || echo "   (instale 'jq' para formatação JSON)"

echo ""
echo -e "${YELLOW}2. Busca reversa (primeiros 2 resultados):${NC}"
curl -s "$BASE_URL/ws/SP/São%20Paulo/Paulista/json/?limit=2" | jq '.' 2>/dev/null || echo "   (instale 'jq' para formatação JSON)"
