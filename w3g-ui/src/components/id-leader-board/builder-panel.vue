<template>
    <span> 
        <span class="headline">Top Builders</span>
        <v-data-table class="elevation-1" :headers="headers" :items="builders" hide-actions item-key="name" disable-initial-sort>
            <template slot="headers" slot-scope="props">
                <tr>
                    <th> Player </th>
                    <th> Rating </th>
                    <th> Wins </th>
                    <th> Losses </th>
                    <th> Ties </th>
                </tr>
            </template>
            <template slot="items" slot-scope="props">
                <tr @click="props.expanded = !props.expanded">
                    <td>{{ props.item.name }} @ {{ props.item.realm }}</td>
                    <td>{{ props.item.rating.toFixed(2) }} </td>
                    <td>{{ props.item.wins }}</td>
                    <td>{{ props.item.losses }}</td>
                    <td>{{ props.item.ties }}</td>
                </tr>
            </template>
            <!--
            <template slot="expand" slot-scope="props">
                <v-card>
                    <v-card-text>
                        TODO: Cool stats like heatmaps, common items, and W/L/T by builder
                        {{ builder }}
                    </v-card-text>
                </v-card>
            </template>
            -->

            <template slot="no-data">
                <span class="text-xs-center">
                    No Builders returned by server
                </span>
            </template>
        </v-data-table>
    </span>
</template>

<script>

export default {
  name: "BuilderPanel",
  props: ["builders"],
  data() {
    return {
      headers: [
        {
          text: "name",
          value: "name"
        },
        {
          text: "realm",
          value: "realm"
        },
        {
          text: "rating",
          value: "rating"
        },
        {
          text: "wins",
          value: "wins"
        },
        {
          text: "losses",
          value: "losses"
        },
        {
          text: "ties",
          value: "ties"
        }
      ]
    };
  }
};
</script>

<style scoped>
div >>> div.v-table__overflow {
  border-radius: 12px;
}

div >>> table.v-datatable.v-table.theme--light > thead {
  background: rgb(194, 178, 128);
}

div >>> table.v-datatable.v-table.theme--light > tbody {
  background: rgb(202, 188, 145);
  background: linear-gradient(
    138deg,
    rgba(202, 188, 145, 1) 17%,
    rgba(219, 209, 180, 1) 100%
  );
}
</style>
