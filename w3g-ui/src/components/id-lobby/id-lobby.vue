<template>
    <v-container class="text-xs-center">
        <titan-panel :titans="lobby.titans" />
        <v-spacer class="py-2" />
        <builder-panel :builders="lobby.builders" />
    </v-container>
</template>

<script>
    import IdApi from "@/plugins/id-api";
    import BuilderPanel from "@/components/id-lobby/builder-panel";
    import TitanPanel from "@/components/id-lobby/titan-panel";

    export default {
        name: "IdLobby",
        components: {
            BuilderPanel,
            TitanPanel
        },
        data() {
            return {
                lobby: {
                    builders: [],
                    titans: []
                },
                interval: null,
            };
        },
        computed: {
            title: function () {
                if (
                    !this.lobby ||
                    !this.lobby.titans ||
                    !this.lobby.titans.players ||
                    !this.lobby.builders ||
                    !this.lobby.builders.players
                ) {
                    return `API Error :(`;
                }
                let numTitans = this.lobby.titans.players ?
                    this.lobby.titans.players.length :
                    0;
                let numBuilders = this.lobby.builders.players ?
                    this.lobby.builders.players.length :
                    0;

                return `${numBuilders + numTitans} / 11 ${
        numTitans == 0 ? "No Titan" : ""
      }`;
            }
        },
        methods: {
            updateLobby: function () {
                return IdApi.getLobby()
                    .then(
                        function (response) {
                            this.lobby = response.data;
                        }.bind(this)
                    )
                    .catch(
                        function (error) {
                            this.$log.error(error);
                        }.bind(this)
                    );
            }
        },
        watch: {
            title: function (newValue) {
                window.document.title = newValue;
            }
        },
        beforeRouteEnter(to, from, next) {
            IdApi.getLobby()
                .then(function (response) {
                    next(vm => {
                        vm.lobby = response.data;
                        vm.interval = setInterval(vm.updateLobby, 5 * 1000);
                    })
                })
        },
        beforeRouteLeave(to, from, next) {
            clearInterval(this.interval);
            this.interval = null;

            next();
        }
    };
</script>

<style>
</style>