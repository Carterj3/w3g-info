# Scripts
## Interrogate Docker
Execute /bin/bash inside a container
> docker exec -it 6ad981394cf0 /bin/bash

Remove images according to a pattern
> docker images -a | grep "pattern" | awk '{print $3}' | xargs docker rmi

How all the kafka docker compose was started
> docker-compose -f docker-compose.yml up -d

Create and run a million gb docker container
> docker build -t hello-rocket .
> docker run --rm -p 8080:8080 -it --name running-rocket hello-rocket

Delete all stopped containers
> docker rm $(docker ps -a -q)

## Rust
Set current folder to be nightly
> rustup override set nightly

Update rust*
> rustup update && cargo update

## Minimal Rust

https://github.com/golddranks/rust_musl_docker
https://hub.docker.com/r/golddranks/rust_musl_docker/tags/
```
docker run -it --rm \
    -v $PWD:/workdir \
    -v ~/.cargo/git:/root/.cargo/git \
    -v ~/.cargo/registry:/root/.cargo/registry \
    golddranks/rust_musl_docker:stretch_nightly-2018-07-24_openssl-1.1.0h \
    cargo build --release -vv --target=x86_64-unknown-linux-musl
```

# Misc
Allegedly replays are 20mb for 45 minutes of gameplay
Kafka groups allow for multi-consumers. If a member of the group receives a message then all consumers of the group have received it (probably becomes a bit less true when replication/etc are >1)

- https://crates.io/crates/log4rs
- https://crates.io/crates/select
- https://github.com/DataWraith/bbt
- view-source:https://entgaming.net/customstats/islanddefense/games/
- http://localhost:8080/v1/lobby/thisIsKindaFast%20NoUnicodePlease
- https://crates.io/crates/qrcode

# Cargo
## Run a single binary in workspace
`cargo run --bin w3g-lobby-ms`

# Versions
## One
Current lobby is the only dynamic page
- Expect W% for builders / Titan. May get pushed to v2 if not easy todo and just use elo math (incorrectly) for v1
- W/L for each player (on that role)
- TruSkill for each player (on that role)
Static pages: About, FAQ, Privacy Policy


## Two
Builders
- Death%, Common races
Titans
- APM (median), Common Titans

## Three
Builders
- Heatmap where they place structures (used to determine bases)
Titan
- Heatmap of pearl spots


/v1/lobby/island%20defense
{ builders: [   { name: String
                , color: String (enum)
                , truSkill: ??
                , wins: Integer
                , losses: Integer
                }
            ]
, titan:    { name: String
            , truSkill: ??
            , wins: Integer
            , losses: Integer
            }
, odds: { team: Integer
        , winChance: Double
        },
        { team: Integer
        , winChance: Double
        }
}

Request hits nginx container w/ lets encrypt

nginx forwards that REST request to a w3g-router-ms that knows how to use PubSub to handle requests

w3g-lobby-ms gets the PubSub request, uses cached to return the players in the current lobby over PubSub.

w3g-router-ms uses the response from PubSub to then ask for all the players' stats over PubSub

w3g-stats-ms gets the PubSub request, pulls from the DB to answer the stats request and response over PubSub

w3g-router-ms uses the response to finish the REST response

----

w3g-downloader-ms polls entgaming to figure out when games have finished and their replay is available. Sends the lobby info (so @useast info is not lost) + url? over PubSub

w3g-parser-ms gets the url+misc and sends a parsed version of it + lobby info over PubSub

w3g-rating-ms gets the PubSub parsed replay and sends a list of Winners/Losers over PubSub

w3g-stats-ms gets the PubSub W/L and updates the stats for those players


# Cleanup


## Delete all images
`docker rmi $(docker images -q)`

## DELETE DANGLING AND UNTAGGED IMAGES
`docker rmi $(docker images -q -f dangling=true)`

## Kill all containers
`docker kill $(docker ps -q)`

## Delete all containers
`docker rm $(docker ps -aq)`