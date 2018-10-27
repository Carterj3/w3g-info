# General
- Mutal TLS (probably just do basic auth or something for now)
- - https://medium.com/@itseranga/tls-mutual-authentication-with-golang-and-nginx-937f0da22a0e
- debug-ms
- - /health (what services are up, maybe Metrics per service)
- - - Services + stats (mem / cpu per) maybe like kafka IO per service
- - - Disk space
- - - Total CPU / Memory
- - /kafka (do a quick loopback check to see if Kafka is working)
- - Force w3g-downloader-ms to keep going
- - Force w3g-downloader-ms to resend replays that are downloaded but were not sent (according to mongo)

# UI
- Convert 
- Header
- - Lobby
- - Leaderboard
- - Privacy (CloudFlare set some cookies)
- - About
- - - Project / GitHub
- - - Definition of stats (BBT / Ties)

# w3g-stats-ms
- Change DB from Mongo to something that has `case-insensitive` support in Rust (Collation not supported yet)
- bulk lookup of players[n] shouldn't be O(n) database access

# General
- rustdoc

- rename most w3g to id since most services are id specific and adding dota/etc should? just be more services.
- borrow a lot more often. Very little reason to use Strings are prolific as I have.
- lib/main style to make testing possible

- ssl certs
https://www.humankode.com/ssl/how-to-set-up-free-ssl-certificates-from-lets-encrypt-using-docker-and-nginx