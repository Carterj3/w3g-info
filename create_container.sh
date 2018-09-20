# Kill all containers
docker kill $(docker ps -q)

# Delete all containers
docker rm $(docker ps -aq)

# Build the docker image
docker build \
    -t lesuorac/w3g-all \
    .

# Delete untagged + dangling images
docker rmi $(docker images -q -f dangling=true)