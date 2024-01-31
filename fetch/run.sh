docker rm themis-fetch
docker build -t themis-fetch .
docker run -d \
    --env-file ../dev.env \
    --net valinor_default \
    --name themis-fetch \
    themis-fetch
if [ "$1" = "-f" ]; then
    docker logs themis-fetch -f
fi