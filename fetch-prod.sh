#!/bin/bash
# prod build and run script

# build and deploy the docker image
docker build -t themis-fetch fetch || exit
docker rm themis-fetch-prod
docker run -d \
    --env-file ./prod.env \
    --net valinor_default \
    --name themis-fetch-prod \
    themis-fetch

# tail logs if requested
if [ "$1" = "-f" ]; then
    docker logs themis-fetch-prod -f
fi