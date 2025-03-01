set dotenv-load

# List commands, default
default:
  just --list

# Download new markets to cache
[working-directory: 'download']
download *args:
    cargo run -r -- {{args}}

# Extract markets from cache
[working-directory: 'extract']
extract *args:
    cargo run -r -- {{args}}

# Run extract tests
[working-directory: 'extract']
test-extract *args:
    cargo test -- {{args}}

# Start the database containers
db-up:
    docker compose up -d

# Stop the database containers
db-down:
    docker compose down

# Stop the database containers
db-logs:
    docker compose logs -f

# Get the database schema
db-run-sql file:
    docker compose exec -T postgres psql \
    --username=$POSTGRES_USER \
    --dbname=$POSTGRES_DB \
    < {{file}}

# Get the database schema
db-schema:
    docker compose exec postgres pg_dump \
    --username=$POSTGRES_USER \
    --dbname=$POSTGRES_DB \
    --schema-only

# Run a manual database backup
db-backup:
    docker compose exec pgbackups /backup.sh

# Get items from an endpoint (example)
db-curl *endpoint:
    curl -sf \
    -X GET "http://${PGRST_HOST}:${PGRST_PORT}/{{endpoint}}" \
    -H "Authorization: Bearer ${PGRST_APIKEY}" | jq

# Build the astro site
astro-build:
    -docker run -it --rm \
        -v .:/app \
        -w /app/site \
        -u "$(id -u):$(id -g)" \
        -p 4321:4321 \
        --name astro \
        node:23-bookworm \
        npx astro build

# Start the astro dev server
astro-dev:
    -docker run -it --rm \
        -v .:/app \
        -w /app/site \
        -u "$(id -u):$(id -g)" \
        -p 4321:4321 \
        --name astro \
        node:23-bookworm \
        npx astro dev --host

# Start a shell in the astro environment
astro-shell:
    -docker run -it --rm \
        -v .:/app \
        -w /app/site \
        -u "$(id -u):$(id -g)" \
        -p 4321:4321 \
        --name astro \
        node:23-bookworm \
        bash

# Build the site and deploy with rclone
deploy: astro-build
    rclone sync site/dist $RCLONE_TARGET --progress
