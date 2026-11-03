FROM scratch
ARG VARIANT=full
LABEL org.opencontainers.image.title="OpenStream TV" \
      org.opencontainers.image.description="Multi-provider streaming and IPTV server (${VARIANT} build)" \
      org.opencontainers.image.source="https://github.com/wpfyorg/openstream-tv"
WORKDIR /app
ENV HOME=/app
COPY openstream /usr/local/bin/openstream
VOLUME ["/app/.openstream-tv"]
# 5001: plain HTTP (IPTV apps); 5443: HTTPS with a self-signed certificate stored in the data volume.
EXPOSE 5001 5443
ENTRYPOINT ["/usr/local/bin/openstream"]
CMD ["--skip-update-check", "serve", "--host", "0.0.0.0", "--port", "5001", "--tls", "--tls-port", "5443"]
