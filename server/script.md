
## Update system
```
apt-get update
apt-get upgrade
```

## Verify password is only way to login
```
vim /etc/ssh/sshd_config

PasswordAuthentication no


service ssh restart
```

## Block users who fail to login
```
apt-get install fail2ban
```

## UFW (firewall)
The docker one-click had a bunch of other ports open.
`ufw delete <1-index rule#>` removes them
```
ufw allow 22
ufw allow 80
ufw allow 443
ufw enable
```


## Auto update server
```
apt-get install unattended-upgrades

vim /etc/apt/apt.conf.d/10periodic

APT::Periodic::Update-Package-Lists "1";
APT::Periodic::Download-Upgradeable-Packages "1";
APT::Periodic::AutocleanInterval "7";
APT::Periodic::Unattended-Upgrade "1";


vim /etc/apt/apt.conf.d/50unattended-upgrades

Unattended-Upgrade::Allowed-Origins {
        "Ubuntu lucid-security";
//      "Ubuntu lucid-updates";
};
```

## Email logs to myself
```
apt-get install logwatch

vim /etc/cron.daily/00logwatch

/usr/sbin/logwatch --output mail --mailto JeffreyKCarter@gmail.com --detail high
```

## Check what Services are running
sudo netstat -plunt

## Add a user
```
adduser --disabled-password w3
usermod -aG docker w3g

# Give ourselves ssh
mkdir /home/w3g/.ssh
cp ~/.ssh/authorized_keys /home/w3g/.ssh

# Fix ssh permissions
chown -R w3g:w3g /home/w3g/
chmod 700 /home/w3g/.ssh
chmod 644 /home/w3g/.ssh/authorized_keys
```