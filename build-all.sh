#!/bin/bash
set -e

echo "🚀 OpenCEP - Build completo com busca reversa"
echo ""

# Habilita BuildKit
export DOCKER_BUILDKIT=1
export COMPOSE_DOCKER_CLI_BUILD=1

# Versão do OpenCEP
OPENCEP_VERSION=${OPENCEP_VERSION:-2.0.1}

echo "📦 Versão OpenCEP: ${OPENCEP_VERSION}"
echo ""

# Build com cache
echo "🔨 Construindo imagens..."
docker compose build --build-arg OPENCEP_VERSION=${OPENCEP_VERSION}

echo ""
echo "✅ Build concluído!"
echo ""
echo "Para iniciar os serviços:"
echo "  docker compose up -d"
echo ""
echo "Para testar:"
echo "  # CEP direto (cache local)"
echo "  curl http://localhost:8080/ws/01001000/json/"
echo ""
echo "  # Busca por endereço (novo!)"
echo "  curl http://localhost:8080/ws/SP/São%20Paulo/Paulista/json/"
echo ""
echo "  # Health checks"
echo "  curl http://localhost:8080/health"
echo "  curl http://localhost:3000/health"
