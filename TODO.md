# General
- debug-ms
- - /health (what services are up, maybe Metrics per service)
- - - Services + stats (mem / cpu per) maybe like kafka IO per service
- - - Disk space
- - - Total CPU / Memory
- - /kafka (do a quick loopback check to see if Kafka is working)
- - Force w3g-downloader-ms to keep going
- - Force w3g-downloader-ms to resend replays that are downloaded but were not sent (according to mongo)

# w3g-stats-ms
- Change DB from Mongo to something that has `case-insensitive` support in Rust (Collation not supported yet)
- bulk lookup of players[n] shouldn't be O(n) database access

# General
- rustdoc

- rename most w3g to id since most services are id specific and adding dota/etc should? just be more services.
- lib/main style to make testing possible