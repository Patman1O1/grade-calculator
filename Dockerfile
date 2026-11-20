# One image with the Rust server and the built React app.
#   docker build -t grade-calculator .
#   docker run -p 8080:8080 grade-calculator   → http://localhost:8080

FROM node:22-slim AS frontend
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json frontend/.npmrc ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

FROM rust:1-slim-trixie AS backend
WORKDIR /app/backend
# Build dependencies first so they are cached between source changes.
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && touch src/lib.rs \
    && cargo build --release --locked && rm -rf src
COPY backend/ ./
RUN touch src/main.rs src/lib.rs && cargo build --release --locked

FROM debian:trixie-slim
COPY --from=backend /app/backend/target/release/grade-calculator /usr/local/bin/grade-calculator
COPY --from=frontend /app/frontend/dist /app/dist
ENV STATIC_DIR=/app/dist \
    PORT=8080
EXPOSE 8080
USER nobody
CMD ["grade-calculator"]
