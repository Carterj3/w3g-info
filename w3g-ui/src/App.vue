<template>
    <v-app class="app"> 
        <IdLobby :lobby=lobby />
    </v-app>
</template>

<script>
import IdLobby from "@/components/id-lobby/id-lobby";
import IdApi from "./plugins/id-api";

export default {
  name: "App",
  components: {
    IdLobby
  },
  data() {
    return {
      lobby: {
        builders: [],
        titans: []
      }
    };
  },
  computed: {
    title: function() {
      if (
        !this.lobby ||
        !this.lobby.titans ||
        !this.lobby.titans.players ||
        !this.lobby.builders ||
        !this.lobby.builders.players
      ) {
        return `API Error :(`;
      }
      let numTitans = this.lobby.titans.players
        ? this.lobby.titans.players.length
        : 0;
      let numBuilders = this.lobby.builders.players
        ? this.lobby.builders.players.length
        : 0;

      return `${numBuilders + numTitans} / 11 ${
        numTitans == 0 ? "No Titan" : ""
      }`;
    }
  },
  methods: {
    getIdLobby: function() {
      IdApi.getLobby(
        function(response) {
          this.lobby = response.data; 
        }.bind(this),
        function(error) {
          this.$log.error(error);
        }.bind(this)
      );
    }
  },
  watch: {
    title: function(newValue) {
      window.document.title = newValue;
    }
  },
  created: function() {
    this.getIdLobby();

    setInterval(
      function() {
        this.getIdLobby();
      }.bind(this),
      5 * 1000
    );
  }
};
</script>

<style scoped>
.app {
  background-image: linear-gradient(to right bottom, #20d872, #044882);
}
</style>
