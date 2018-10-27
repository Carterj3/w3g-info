import axios from 'axios'

const baseUrl = "https://api.islanddefense.info"

export default {
    getLobby(onResponse, onError) {
        return axios.get(baseUrl + "/v1/lobby/island-defense")
            .then(onResponse)
            .catch(onError);
    }
}