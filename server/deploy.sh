read -e -p "Enter Server IP:" IP

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
"

# TODO: restart containers using the new image + delete old image