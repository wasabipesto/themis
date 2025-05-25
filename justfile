set dotenv-load

# List commands, default
default:
  just --list

# Download new markets to cache
[working-directory: 'download']
download *args:
    cargo run -qr -- {{args}}

# Run download tests
[working-directory: 'download']
download-test:
      cargo test -q

# Extract markets from cache
[working-directory: 'extract']
extract *args:
    cargo run -qr -- {{args}}

# Run extract tests
[working-directory: 'extract']
extract-test:
    cargo test -q

# Grade markets
[working-directory: 'grader']
grade *args:
  cargo run -qr -- {{args}}

# Run grader tests
[working-directory: 'grader']
grade-test:
  cargo test -q

# Start the database containers
db-up:
    docker compose up -d

# Stop the database containers
db-down:
    docker compose down

# Get the database containers logs
db-logs:
    docker compose logs -f

# Get a shell into the database
db-shell:
    docker exec -it $POSTGRES_CONTAINER_NAME psql \
    --username=$POSTGRES_USER \
    --dbname=$POSTGRES_DB

# Run a SQL file on the database
db-run-sql file:
    docker exec -i $POSTGRES_CONTAINER_NAME psql \
    --username=$POSTGRES_USER \
    --dbname=$POSTGRES_DB \
    < {{file}}

# Get the database schema
db-get-schema:
    docker exec -i $POSTGRES_CONTAINER_NAME pg_dump \
    --username=$POSTGRES_USER \
    --dbname=$POSTGRES_DB \
    --schema-only

# Load the default database schema
db-load-schema:
    just db-run-sql schema/01-roles.sql
    just db-run-sql schema/02-schema.sql
    just db-run-sql schema/03-views.sql
    just db-run-sql schema/04-vector-tables.sql
    just db-run-sql schema/05-vector-queries.sql
    just db-run-sql schema/06-feedback.sql
    just db-run-sql schema/10-platforms.sql
    just db-run-sql schema/11-categories.sql

# Run a manual database backup
db-backup:
    docker exec pgbackups /backup.sh

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
    NODE_OPTIONS="--max-old-space-size=64000" \
    npx astro dev

# Check the main site for errors
[working-directory: 'site']
site-test:
    npx astro check --silent

# Reset site cache
site-cache-reset:
    mkdir -p site/cache/archive
    mv site/cache/*.json site/cache/archive

# Build the main site
[working-directory: 'site']
site-build *args:
    NODE_OPTIONS="--max-old-space-size=64000" \
    npx astro build {{args}}

# Push the site to dev with rclone
site-push-dev:
    rclone sync site/dist $RCLONE_DEV_TARGET --progress

# Push the site to prod with rclone
[confirm]
site-push-prod:
    rclone sync site/dist $RCLONE_PROD_TARGET --progress

# Build the main site and deploy
deploy: site-test site-build site-push-dev site-push-prod

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
    npx astro check --silent

# Generate embeddings
embeddings *args:
    uv run scripts/update-embeddings.py {{args}}

# Run nightly process
nightly: download-test extract-test grade-test group-test site-test
    just download --log-level warn --reset-cache
    just download --log-level warn
    just extract --log-level warn
    uv run scripts/fix-criterion-probs.py
    just grade --log-level warn
    TQDM_MININTERVAL=100 just embeddings
    just site-cache-reset
    just site-build --silent
    just site-push-dev
