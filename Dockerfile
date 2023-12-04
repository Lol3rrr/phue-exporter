# Based on https://kerkour.com/rust-small-docker-image
FROM rustlang/rust:nightly AS builder

WORKDIR /server/phue-exporter

# Create appuser
ENV USER=goldpass
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

RUN rustup target add x86_64-unknown-linux-musl
RUN apt update && apt install -y musl-tools musl-dev

COPY ./ /server/phue-exporter

RUN cargo build --target x86_64-unknown-linux-musl --release

FROM scratch

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /server

# Copy our build
COPY --from=builder /server/phue-exporter/target/x86_64-unknown-linux-musl/release/phue-exporter ./

# Use an unprivileged user.
# TODO
# This does not work currently
# USER goldpas:goldpass

CMD ["/server/phue-exporter"]
