# =============================================================================
# UNVERIFIED — never executed
#
# This Dockerfile was authored on a machine with no Docker daemon.
# The reviewer should check first:
#   1. node:24-bookworm is the desired base image and tag.
#   2. curl is present (used by Compose healthcheck).
#   3. pnpm install warms node_modules from the lockfile.
# =============================================================================

FROM node:24-bookworm

RUN apt-get update \
    && apt-get install -y curl \
    && rm -rf /var/lib/apt/lists/*

RUN npm install -g pnpm@latest

WORKDIR /app

COPY fake-sites/news/package.json fake-sites/news/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
