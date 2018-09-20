
# w3g-stats-ms
- Composite-index on `player.name`+`player.realm`
- bulk lookup of players shouldn't be O(n) database access
- stats is relational data and so should be relationally stored

# w3g-lobby-ms
- Probably should just push updates to internal lobby model when players leave / join and always send that model to w3g-router-ms
- - Don't want to cache lobby data in w3g-router-ms though since it breaks whole design of w3g-router ( HTTP<->PubSub )
- - Difference is that right now only lobby model stores is player&realm. Future should store stats data as well to save lookups.
- - Maybe w3g-stats-ms should store the current lobby so that w3g-lobby-ms doesn't need to understand the stats per game-type

# w3g-router-ms
- Should really be async w.r.t. PubSub. Only need 1 thread listening for new HTTP and creating a PubSub request and thread pool looking to close HTTP when PubSub response comes in.
- timeouts when a stats request takes too long ( https://github.com/alexcrichton/futures-timer )

# Docker
- Maybe remove `build .` from docker-compose.yml since it seems to cause the image to be made like 5 times. Also probably need to target an image instead when deployed to a server since kinda dumb to scp&build source code.
- Need build script then though. Stop+Delete containers, rebuild image, delete untagged images, and run docker-compose.yml

# General
- rustdoc
- a way to push Errors through PubSub so caller doesn't have to wait on timeout to determine task failed 

- rename most w3g to id since most services are id specific and adding dota/etc should? just be more services.
- borrow a lot more often. Very little reason to use Strings are prolific as I have.
- lib/main style to make testing possible

- ssl certs
https://www.humankode.com/ssl/how-to-set-up-free-ssl-certificates-from-lets-encrypt-using-docker-and-nginx