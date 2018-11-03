<template>
    <v-container >
        <titan-panel :titans="leaderBoard.titans" />

        <builder-panel :builders="leaderBoard.builders" />
    </v-container>
</template>

<script>
    import IdApi from "@/plugins/id-api";
    import BuilderPanel from "@/components/id-leader-board/builder-panel";
    import TitanPanel from "@/components/id-leader-board/titan-panel";

export default {

  name: "IdLeaderBoard",
        components: {
            BuilderPanel,
            TitanPanel
        },
    data() {
        return {
            leaderBoard: {
                builders: [],
                titans: []
            },
            interval: null,
            tabs: null,
        };
    },
    methods: {
        updateLobby: function () {
            return IdApi.getLeaderBoard()
                .then(
                    function (response) {
                        this.leaderBoard = response.data;
                    }.bind(this)
                )
                .catch(
                    function (error) {
                        this.$log.error(error);
                    }.bind(this)
                );
        }
    },
    beforeRouteEnter(to, from, next) {
        IdApi.getLeaderBoard()
            .then(function (response) {
                next(vm => {
                    vm.leaderBoard = response.data;
                    vm.interval = setInterval(vm.updateLeaderBoard, 5 * 1000);
                    window.document.title = "Leader Board";
                })
            })
    },
    beforeRouteLeave(to, from, next) {
        clearInterval(this.interval);
        this.interval = null;

        next();
    }

}
</script>

<style>

</style>
