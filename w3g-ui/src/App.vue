<template>
  <v-app class="app">
    <v-navigation-drawer v-model="drawer" clipped fixed app>
      <v-list class="transparent" >
        <v-list-tile href="/#/lobby" router>
          <v-list-tile-action>
            <v-icon class="fas fa-users" />
          </v-list-tile-action>
          <v-list-tile-content>
            <v-list-tile-title>Lobby</v-list-tile-title>
          </v-list-tile-content>
        </v-list-tile>
        <v-list-tile href="/#/leader-board" router>
          <v-list-tile-action>
            <v-icon class="fas fa-th-list" />
          </v-list-tile-action>
          <v-list-tile-content>
            <v-list-tile-title>Leader Board</v-list-tile-title>
          </v-list-tile-content>
        </v-list-tile>
        <!--
        <v-list-tile href="/#/search" router>
          <v-list-tile-action>
            <v-icon class="fas fa-search" />
          </v-list-tile-action>
          <v-list-tile-content>
            <v-list-tile-title>Search</v-list-tile-title>
          </v-list-tile-content>
        </v-list-tile>
        -->
        <v-list-tile href="/#/privacy" router>
          <v-list-tile-action>
            <v-icon class="fas fa-user-secret" />
          </v-list-tile-action>
          <v-list-tile-content>
            <v-list-tile-title>Privacy</v-list-tile-title>
          </v-list-tile-content>
        </v-list-tile>
        <v-list-tile href="/#/about" router>
          <v-list-tile-action>
            <v-icon class="fas fa-info" />
          </v-list-tile-action>
          <v-list-tile-content>
            <v-list-tile-title>About</v-list-tile-title>
          </v-list-tile-content>
        </v-list-tile>
      </v-list>
    </v-navigation-drawer>

    <v-toolbar app fixed clipped-left>
      <v-toolbar-side-icon @click.stop="drawer = !drawer"></v-toolbar-side-icon>
      <v-toolbar-title>Island Defense Info</v-toolbar-title>
    </v-toolbar>

    <v-content>
      <v-container fluid>
        <router-view></router-view>
      </v-container>
    </v-content>

  </v-app>
</template>

<script>
export default {
  name: "App",
  data() {
    return {
      drawer: false
    };
  }
};
</script>

<style scoped>
.app {
  background-image: linear-gradient(to right bottom, #20d872, #044882);
}

aside >>> div.transparent.v-list {
  background: #00000000
}

nav >>> div {
  background: rgba(202, 188, 145, 1);
}

>>> aside.v-navigation-drawer {
  background: rgba(202, 188, 145, 1);
  background-color: rgba(202, 188, 145, 1);
}
 

</style>