docker rm themis-fetch-dev
docker build -t themis-fetch fetch
docker run -d \
    --env-file ../dev.env \
    --net valinor_default \
    --name themis-fetch-dev \
    themis-fetch
if [ "$1" = "-f" ]; then
    docker logs themis-fetch-dev -f
fi