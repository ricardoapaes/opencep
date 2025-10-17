#!/bin/bash
# Script helper para builds otimizados com cache

set -e

# Cores para output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 OpenCEP Build Helper${NC}\n"

# Habilita BuildKit
export DOCKER_BUILDKIT=1
export COMPOSE_DOCKER_CLI_BUILD=1

echo -e "${GREEN}✓ BuildKit habilitado${NC}"
echo -e "${YELLOW}ℹ Cache de builds será mantido em memória${NC}\n"

# Build
echo -e "${BLUE}Iniciando build...${NC}"
docker compose build "$@"

echo -e "\n${GREEN}✓ Build concluído!${NC}"
echo -e "${YELLOW}Para iniciar: docker compose up -d${NC}"
