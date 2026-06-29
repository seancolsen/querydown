# Development/testing container for Querydown.
#
# Gives an isolated environment with the full toolchain (Rust workspace, wasm
# bindings, and the SvelteKit site) plus Claude Code, so the agent can run
# commands with full permissions without touching the host system.
#
# Pinned to the same Rust version used on the host (1.91) so build behavior
# matches. Bump this when the host toolchain changes.
FROM rust:1.91-bookworm

# Match the host user so files created in the bind-mounted workspace stay
# owned by you rather than root. Override at build time if your UID/GID differ:
#   docker compose build --build-arg USER_UID=$(id -u) --build-arg USER_GID=$(id -g)
ARG USERNAME=dev
ARG USER_UID=1000
ARG USER_GID=1000

# System packages + Node.js 20 (needed for the site and for Claude Code).
#
# `postgresql` backs the `db-tests` feature, which runs the compiled corpus SQL against real
# databases (see DEVELOPMENT.md). The Postgres binaries land under /usr/lib/postgresql/<ver>/bin;
# symlink them onto PATH so `cargo test` can spawn them. DuckDB is no longer installed as a CLI —
# the `db-tests` feature compiles and statically links it via the bundled `duckdb` crate.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        git \
        curl \
        ca-certificates \
        pkg-config \
        sudo \
        unzip \
        postgresql \
    && ln -sf /usr/lib/postgresql/*/bin/* /usr/local/bin/ \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# Rust components and the wasm target/tooling used by bindings/js.
RUN rustup component add clippy rustfmt \
    && rustup target add wasm32-unknown-unknown \
    && curl -fsSL https://rustwasm.github.io/wasm-pack/installer/init.sh | sh

# Claude Code CLI and doctoc (regenerates the docs tables of contents — see DEVELOPMENT.md).
RUN npm install -g @anthropic-ai/claude-code doctoc

# Ensure cargo is on PATH for login shells too (the base image's ENV PATH is
# otherwise reset by /etc/profile in a `bash -l` context).
RUN echo 'export PATH=/usr/local/cargo/bin:$PATH' > /etc/profile.d/cargo.sh

# Create the non-root user and give it ownership of the Rust toolchain dirs so
# the cargo registry/target named volumes (mounted later) initialize writable.
RUN groupadd --gid "${USER_GID}" "${USERNAME}" 2>/dev/null || true \
    && useradd --uid "${USER_UID}" --gid "${USER_GID}" -m -s /bin/bash "${USERNAME}" \
    && echo "${USERNAME} ALL=(ALL) NOPASSWD:ALL" > "/etc/sudoers.d/${USERNAME}" \
    && chmod 0440 "/etc/sudoers.d/${USERNAME}" \
    && mkdir -p /workspace /usr/local/cargo/registry /usr/local/cargo/git \
    && chown -R "${USER_UID}:${USER_GID}" /workspace /usr/local/cargo /usr/local/rustup

COPY docker/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# Script to append a timestamp to the Claude notification queue.
RUN printf '#!/bin/sh\necho "$(date \047+%%Y-%%m-%%d %%H:%%M:%%S\047)" >> ~/.claude/notification-queue.txt\n' \
    > /usr/local/bin/ding \
    && chmod +x /usr/local/bin/ding

USER ${USERNAME}
WORKDIR /workspace

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["/bin/bash"]
