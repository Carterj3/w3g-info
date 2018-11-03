import '@babel/polyfill'
import Vue from 'vue'
import VueRouter from 'vue-router'
import './plugins/vuetify'
import './plugins/axios'
import './plugins/vue-logger'
import './plugins/id-api'
import App from './App.vue'
import 'roboto-fontface/css/roboto/roboto-fontface.css'
import '@fortawesome/fontawesome-free/css/all.css'

import IdLobby from "@/components/id-lobby/id-lobby";
import IdLeaderBoard from "@/components/id-leader-board/id-leader-board";
import About from "@/components/static/about";
import Privacy from "@/components/static/privacy";

Vue.use(VueRouter)
 
Vue.config.productionTip = false

let router = new VueRouter({
  routes: [
    {
      path: '/lobby',
      name: 'ID Lobby',
      component: IdLobby,
    },
    {
      path: '/leader-board',
      name: 'ID Leader Board',
      component: IdLeaderBoard,
    },
    {
      path: '/about',
      name: 'About',
      component: About,
    },
    {
      path: '/privacy',
      name: 'Privacy',
      component: Privacy,
    },
    { path: '*', redirect: '/lobby' }
  ]
})
 
new Vue({
  render: h => h(App),
  router
}).$mount('#app')
