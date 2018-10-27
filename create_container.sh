# Kill all containers
docker kill $(docker ps -q)

# Delete all containers
docker rm $(docker ps -aq)

## Rust images
# Currently all of the backend shares the same image
docker build \
    -t lesuorac/w3g-all \
    .

## Node images
# Build the UI
docker build \
    -t lesuorac/w3g-ui \
    w3g-ui

# Delete untagged + dangling images
docker rmi $(docker images -q -f dangling=true)