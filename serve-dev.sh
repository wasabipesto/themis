#!/bin/bash
# dev build and deploy script

# build and deploy the docker image
docker build -t themis-serve-dev serve || exit
docker stop themis-serve-dev
docker rm themis-serve-dev
docker run -d \
    --env-file ./dev.env \
    --env HTTP_BIND=0.0.0.0:7043 \
    -p 7043:7043 \
    --net valinor_default \
    --name themis-serve-dev \
    themis-serve-dev

# tail logs if requested
if [ "$1" = "-f" ]; then
    docker logs themis-serve-dev -f
fi