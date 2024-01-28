#!/bin/bash
# dev build and deploy script
# mounts working directory and enables hot-reload

# build and deploy the docker image
docker build -t themis-client-dev -f Dockerfile-dev . || exit
docker stop themis-client-dev
docker rm themis-client-dev
docker run -d \
    -p 8147:8147 \
    -v ./:/usr/src/themis-client \
    --name themis-client-dev \
    themis-client-dev

# tail logs if requested
if [ "$1" = "-f" ]; then
    docker logs themis-client-dev -f
fi