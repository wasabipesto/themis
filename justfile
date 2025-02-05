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

# Start the astro dev server in docker
astro-dev:
    -docker run -it --rm \
        -v .:/app \
        -w /app/site \
        -u "$(id -u):$(id -g)" \
        -p 4321:4321 \
        --name node \
        node:23-bookworm \
        npx astro dev --host

# Start an empty shell in the astro docker container
astro-shell:
    -docker run -it --rm \
        -v .:/app \
        -w /app/site \
        -u "$(id -u):$(id -g)" \
        -p 4321:4321 \
        --name node \
        node:23-bookworm \
        bash
