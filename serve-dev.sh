docker stop themis-serve-dev
docker rm themis-serve-dev
docker build -t themis-serve serve
docker run -d \
    --env-file ./dev.env \
    --env HTTP_BIND=0.0.0.0:7042 \
    -p 7042:7042 \
    --net valinor_default \
    --name themis-serve-dev \
    themis-serve
if [ "$1" = "-f" ]; then
    docker logs themis-serve-dev -f
fi