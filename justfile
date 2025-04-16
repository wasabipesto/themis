set dotenv-load

# List commands, default
default:
  just --list

# Download new markets to cache
[working-directory: 'download']
download *args:
    cargo run -r -- {{args}}

# Run download tests
[working-directory: 'download']
download-test:
      cargo test

# Extract markets from cache
[working-directory: 'extract']
extract *args:
    cargo run -r -- {{args}}

# Run extract tests
[working-directory: 'extract']
extract-test:
    cargo test

# Grade markets
[working-directory: 'grader']
grade *args:
  cargo run -r -- {{args}}

# Run grader tests
[working-directory: 'grader']
grade-test:
  cargo test

# Start the database containers
db-up:
    docker compose up -d

# Stop the database containers
db-down:
    docker compose down

# Get the database containers logs
db-logs:
    docker compose logs -f

# Run a SQL file on the database
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

# Get DB items from an endpoint
db-curl *endpoint:
    curl -sf \
    -X GET "${PGRST_URL}/{{endpoint}}" \
    -H "Authorization: Bearer ${PGRST_APIKEY}" | jq

# Refresh all database views
db-refresh-all:
    curl -sf \
    -X POST "${PGRST_URL}/rpc/refresh_all_materialized_views" \
    -H "Authorization: Bearer ${PGRST_APIKEY}" | jq

# Refresh the quick database views
db-refresh-quick:
    curl -sf \
    -X POST "${PGRST_URL}/rpc/refresh_quick_materialized_views" \
    -H "Authorization: Bearer ${PGRST_APIKEY}" | jq

# Start the main site dev server
[working-directory: 'site']
site-dev:
    NODE_OPTIONS=--max_old_space_size=8192 \
    npx astro dev

# Check the main site for errors
[working-directory: 'site']
site-test:
    NODE_OPTIONS=--max_old_space_size=8192 \
    npx astro check

# Build the main site
[working-directory: 'site']
site-build:
    NODE_OPTIONS=--max_old_space_size=8192 \
    npx astro build

# Push the main site with rclone
site-push:
    rclone sync site/dist $RCLONE_SITE_TARGET --progress

# Build the main site and deploy
deploy: site-test site-build site-push

# Start the grouper dev server
[working-directory: 'grouper']
group:
    PUBLIC_PGRST_URL=${PGRST_URL} \
    PUBLIC_PGRST_APIKEY=${PGRST_APIKEY} \
    PUBLIC_OLLAMA_URL=${OLLAMA_URL} \
    PUBLIC_OLLAMA_MODEL=${OLLAMA_MODEL} \
    npx astro dev

# Check the grouper site for errors
[working-directory: 'grouper']
group-test:
    npx astro check

# Run all tests
test-all: download-test extract-test grade-test site-test group-test

# Run nightly process
nightly: test-all
    just download --log-level warn --reset-cache
    just download --log-level warn
    just extract --log-level warn
    just grade --log-level warn
    just site-build site-push
