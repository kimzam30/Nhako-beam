# syntax=docker/dockerfile:1
#
# Nhako Beam server: the iroh relay and the iroh address lookup server (pkarr),
# built from their published crates at a pinned version. One image, two binaries.
# docker-compose.yml runs each as its own service.

ARG RUST_IMAGE=rust:1.97-slim-trixie
ARG RUNTIME_IMAGE=gcr.io/distroless/cc-debian13:nonroot

FROM ${RUST_IMAGE} AS build
# Keep these on the same version as the iroh crate in the app (Cargo.lock).
ARG IROH_VERSION=1.3.0
# --locked uses each crate's own published lock file, so builds repeat.
# The optional "extra_ca" secret is only for building behind a proxy that
# inspects TLS; normal builds don't need it.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=secret,id=extra_ca,required=false \
    if [ -f /run/secrets/extra_ca ]; then export CARGO_HTTP_CAINFO=/run/secrets/extra_ca; fi; \
    cargo install --locked --root /out iroh-relay --version "${IROH_VERSION}" --features server \
 && cargo install --locked --root /out iroh-dns-server --version "${IROH_VERSION}" \
 && mkdir -p /out/data

FROM ${RUNTIME_IMAGE}
COPY --from=build /out/bin/iroh-relay /out/bin/iroh-dns-server /usr/local/bin/
COPY --from=build --chown=65532:65532 /out/data /data
COPY config/ /etc/nhako/
USER 65532:65532
ENV IROH_DNS_DATA_DIR=/data
LABEL org.opencontainers.image.title="Nhako Beam server" \
      org.opencontainers.image.description="iroh relay and address lookup server for Nhako Beam" \
      org.opencontainers.image.source="https://github.com/kimzam30/nhako-beam"
ENTRYPOINT ["/usr/local/bin/iroh-relay"]
CMD ["--config-path", "/etc/nhako/relay.toml"]
