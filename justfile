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

# Get DB items from an endpoint
db-refresh-views *endpoint:
    curl -sf \
    -X POST "${PGRST_URL}/rpc/refresh_all_materialized_views" \
    -H "Authorization: Bearer ${PGRST_APIKEY}" | jq

# Start the main site dev server
[working-directory: 'site']
dev:
    npx astro dev

# Check the main site for errors
[working-directory: 'site']
site-test:
    npx astro check

# Build the main site
[working-directory: 'site']
build:
    npx astro build

# Build the main site and deploy with rclone
deploy: site-test build
    rclone sync site/dist $RCLONE_SITE_TARGET --progress

# Start the grouper dev server
[working-directory: 'grouper']
group:
    npx astro dev

# Check the grouper site for errors
[working-directory: 'grouper']
group-test:
    npx astro check

# Build the grouper site
[working-directory: 'grouper']
group-build:
    npx astro build

# Build the grouper site and deploy with rclone
group-deploy: group-test group-build
    rclone sync grouper/dist $RCLONE_ADMIN_TARGET --progress

# Run all tests
test-all: download-test extract-test site-test group-test
