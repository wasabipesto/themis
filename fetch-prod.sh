docker rm themis-fetch-prod
docker build -t themis-fetch fetch
docker run -d \
    --env-file ./prod.env \
    --net valinor_default \
    --name themis-fetch-prod \
    themis-fetch
if [ "$1" = "-f" ]; then
    docker logs themis-fetch-prod -f
fi