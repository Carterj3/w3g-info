import axios from 'axios'

const baseUrl = "https://api.islanddefense.info"

export default {
    getLobby() {
        return axios.get(baseUrl + "/v1/lobby/island-defense");
    },
    getLeaderBoard() {
        return axios.get(baseUrl + "/v1/leaderBoard/island-defense");
    },
    getPlayer(username) {
        return axios.get(baseUrl + "/v1/player/island-defense", { params: { name: username }});
    }
}