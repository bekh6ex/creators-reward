# =============================================================================
# UNVERIFIED — never executed
#
# This Dockerfile was authored on a machine with no Docker daemon.
# The reviewer should check first:
#   1. node:24-bookworm is the desired base image and tag.
#   2. pnpm@latest is installed globally.
#   3. The COPY warms node_modules so the named volume is seeded on first run.
# =============================================================================

FROM node:24-bookworm

RUN npm install -g pnpm@latest

WORKDIR /app

COPY frontend/creator/package.json frontend/creator/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
