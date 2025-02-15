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

# Build the astro site in docker
astro-build:
    -docker run -it --rm \
        -v .:/app \
        -w /app/site \
        -u "$(id -u):$(id -g)" \
        -p 4321:4321 \
        --name astro \
        node:23-bookworm \
        npx astro build

# Start the astro dev server in docker
astro-dev:
    -docker run -it --rm \
        -v .:/app \
        -w /app/site \
        -u "$(id -u):$(id -g)" \
        -p 4321:4321 \
        --name astro \
        node:23-bookworm \
        npx astro dev --host

# Start a shell in the astro docker container
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
    rclone sync site/dist $SITE_TARGET --progress
