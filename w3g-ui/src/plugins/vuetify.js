import Vue from 'vue'
import {
  Vuetify,
  VApp,
  VExpansionPanel,
  VNavigationDrawer,
  VFooter,
  VList,
  VBtn,
  VIcon,
  VGrid,
  VCard,
  VDataTable,
  VToolbar,
  transitions
} from 'vuetify'
import 'vuetify/src/stylus/app.styl'

Vue.use(Vuetify, {
  components: {
    VApp,
    VNavigationDrawer,
    VExpansionPanel,
    VFooter,
    VList,
    VBtn,
    VIcon,
    VGrid,
    VCard,
    VDataTable,
    VToolbar,
    transitions
  },
  customProperties: true,
  iconfont: 'fa',
})
