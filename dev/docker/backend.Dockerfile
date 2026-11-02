# =============================================================================
# UNVERIFIED — never executed
#
# This Dockerfile was authored on a machine with no Docker daemon.
# The reviewer should check first:
#   1. rust:1-bookworm is the desired base image and tag.
#   2. cargo-watch is installed and available on PATH.
# =============================================================================

FROM rust:1-bookworm

RUN cargo install cargo-watch

WORKDIR /app
