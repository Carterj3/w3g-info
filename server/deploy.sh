if [[ -z "${DO_SERVER}" ]]; then
  read -e -p "Enter Server IP:" IP
else
  IP="${DO_SERVER}"
fi

docker save -o /tmp/w3g-all.image lesuorac/w3g-all:latest
scp /tmp/w3g-all.image root@$IP:/data
rm /tmp/w3g-all.image

docker save -o /tmp/w3g-ui.image lesuorac/w3g-ui:latest
scp /tmp/w3g-ui.image root@$IP:/data
rm /tmp/w3g-ui.image

ssh root@$IP "
/usr/bin/docker load -i /data/w3g-all.image && \
rm /data/w3g-all.image

/usr/bin/docker load -i /data/w3g-ui.image && \
rm /data/w3g-ui.image

cd /data
/usr/local/bin/docker-compose down && /usr/local/bin/docker-compose up -d
"

# TODO: restart containers using the new image + delete old image