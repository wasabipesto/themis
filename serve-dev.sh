#!/bin/bash
# dev build and deploy script

# build and deploy the docker image
docker build -t themis-serve serve || exit
docker stop themis-serve-dev
docker rm themis-serve-dev
docker run -d \
    --env-file ./dev.env \
    --env HTTP_BIND=0.0.0.0:7042 \
    -p 7042:7042 \
    --net valinor_default \
    --name themis-serve-dev \
    themis-serve

# tail logs if requested
if [ "$1" = "-f" ]; then
    docker logs themis-serve-dev -f
fi