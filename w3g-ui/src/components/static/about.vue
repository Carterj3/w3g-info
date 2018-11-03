<template>
    <v-container>
        <h2>What is this</h2>
        <p> This website shows statistics related to a map for Warcraft III, <a href="https://entgaming.net/forum/viewforum.php?f=101">Island Defense</a>. Unfortunately, it relies upon games played on Entgaming's servers which doesn't happen anymore due to recent Blizzard Patches.
        </p>

        <h2>How are the statistics computed?</h2>
        <p>
            The server downloads <a href="https://entgaming.net/customstats/islanddefense/games/">games</a> played and parses them to determine who played, how long the game was, and who won. If the game took over an hour it's a tie and no rating changes will occur (I don't like stallers). Otherwise the teams are ran through <a href="https://crates.io/crates/bbt">bbt</a> to get new ratings and then they're updated. Everybody starts with 1500.
        </p>

        <h2>Future Ideas</h2>
        <p>
            Well since nobody plays games anymore I'm probably not going to do much more work.
            Things I will do: Fix the HTML encoded usernames &amp; add a player-search.
            Things that would've been cool to do would be get stats on when people died, what races they were best at, where they common build (by time to tell lumber base v actual base), where titans commonly pearl, what items titans buy, etc.
        </p>

        <h2>Why did you do this?</h2>
        <p>
            Well I've had the domain names for awhile and I figured it'd make for a decent project to learn <a href="https://www.rust-lang.org/en-US/">Rust</a> (technically the UI is done in Vue but that wasn't a goal).
        </p>

        <h2>Where did Stealer go?</h2>
        <p>
            I have a job and play hockey on 6 teams... No time.
        </p>
    </v-container>
</template>

<script>
export default {

}
</script>

<style>

</style>
