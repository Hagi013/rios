#!/bin/bash

# enable to use dhcpd in ubuntu
# ref) https://www.server-world.info/query?os=Ubuntu_20.04&p=dhcp&f=1

rm -rf /etc/dhcp/dhcpd.conf
rm -rf /var/lib/dhcp/dhcpd.leases

cat > /etc/dhcp/dhcpd.conf <<EOF
subnet 192.168.56.0 netmask 255.255.255.0 {
    range 192.168.56.2 192.168.56.252;
    option routers 192.168.56.254;
    option domain-name-servers 192.168.56.1;
}
EOF
touch /var/lib/dhcp/dhcpd.leases
chown root:dhcpd /var/lib/dhcp/
