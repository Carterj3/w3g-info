<template>
    <v-container class="text-xs-center"> 
            <titan-panel :titans="lobby.titans" /> 
            <v-spacer class="py-2" />
            <builder-panel :builders="lobby.builders" /> 
        </v-container>
</template>

<script>
    import BuilderPanel from '@/components/id-lobby/builder-panel';
    import TitanPanel from '@/components/id-lobby/titan-panel';

export default {
        name: 'IdLobby',
        components: {
            BuilderPanel, 
            TitanPanel,
        },
        props: ["lobby"],
}
</script>

<style>

</style>
