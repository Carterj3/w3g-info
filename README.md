# About
The primary purpose of this project was to learn Rust. A lot of this code will look like the style changed over time (it did).

Due to things out of my control (Blizzard went after hostbots so my data source no longer existed) there became pretty much no point in this project and since it's primary goal was to get more experience with Rust / Docker / Servers / etc and that happened I'm putting it down.

# Mistakes

## Kafka
While for true production systems Kafka may be great. When you're trying to use as minimal amount of resources dedicating half the servers RAM to Kafka is a bit expensive.

## PubSub
All of the Rust services take an extremely small amount of CPU / RAM. It also prohibts the use of borrowing in a lot of cases because you can't deserialize into a borrowed reference.

I also had no need for the persistance of the pubsub messages (or really even pubsub, it was overkill).

It also seems to be a bit poor to go all in on PubSub as things such as the Leaderboard / updating stats are bett

## Testing
Uh yeah ... Definitely laying things out in the whole lib+main though is good and would make testing easier.

## Database Support
Mongo's not a bad database but it doesn't have the best support in Rust. Usernames (business logic) are case-insensitive but collation isn't supported by the rust library for mongo so regex search is how it has to be done which is unfortunately an O(n) operation.
Also somehow Mongo would use ~.4+ GB of RAM until I added indexes and then the RAM significantly went down but still annoying when all of the VMs resources belong to non-business logic or that the database will goes OOM issues.

## Musl-ish
Really the lack of a build computer is probably the issue. Running the Dockerfile to create w3g-all (which could be modified to generate a bunch of individual images but am lazy and not really any benefits) takes a rather long time especially since it downloads & compiles a lot of libraries but the macbook fails because of no musl-gcc so if I had some linux thing setup to auto-build this would go much faster / better.

# Good Ideas

## Message format
Using the name of the topic to dictate what the format of the data seems to be a good idea although its still kind of weird how some of the services were linked (i.e. router asked lobby forward the players to stats to forward the stats of the lobby back to router).
The idea of a debug HashMap still seems good although I didn't use it but storing data like timings in it certainly would be useful.
The destinations field still seems good though. The services down the line still need to understand the format of the data being sent but it definitely would allow for future expansion using the previous services although it also makes it way harder to tell if a change is going to break something :/ (Not as good as the whole versioning REST APIs).

## Rust
Like everything takes 0 resources and runs fast.

# Credits

## Understanding WC3 replay format
- Strilanc ( https://github.com/Strilanc/Tinker/tree/master/Warcraft3/Replay )
- Blue & Nagger ( http://w3g.deepnode.de/ )