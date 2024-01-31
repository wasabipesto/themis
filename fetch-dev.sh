#!/bin/bash
# dev build and run script

# build and deploy the docker image
docker build -t themis-fetch fetch || exit
docker rm themis-fetch-dev
docker run -d \
    --env-file ./dev.env \
    --net valinor_default \
    --name themis-fetch-dev \
    themis-fetch

# tail logs if requested
if [ "$1" = "-f" ]; then
    docker logs themis-fetch-dev -f
fi