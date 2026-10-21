#!/bin/sh
# The game servers' host for a GitHub Pages site (deploy/README.md, "GitHub Pages"): run as root on
# the VPS, from the folder holding what it installs:
#   conker-server   the Linux server binary (cargo build --release -p conker-server)
#   server.ts, data.ts   web/app/server.ts and web/app/src/data.ts (the web server)
#   assets/         the pack from deploy/pack-wasm.mjs (the browsers' own, byte for byte)
# It adds one nginx site (an nginx already there keeps its others), gets a Let's Encrypt certificate
# for DOMAIN, installs Bun and runs the web server as conker-web: /api/ for the Pages site's pages
# only (CORS), at most MATCHES matches, each in the unit's memory and CPU share so the host's other
# services keep theirs.
#   DOMAIN=game.example.org PUBLIC=203.0.113.7 ORIGIN=https://you.github.io EMAIL=you@example.org \
#   [PAGES=https://you.github.io/repo/] [MATCHES=3] [API_PORT=8090] [FIRST_PORT=5900] sh pages-vps.sh
# Run again to update: the binary, the web server and the pack are replaced, the certificate kept.
set -e
: "${DOMAIN:?}" "${PUBLIC:?}" "${ORIGIN:?}" "${EMAIL:?}"
PAGES=${PAGES:-$ORIGIN/}
MATCHES=${MATCHES:-3}
API_PORT=${API_PORT:-8090}
FIRST_PORT=${FIRST_PORT:-5900}
LAST_PORT=$((FIRST_PORT + 2 * MATCHES - 1))
here=$(cd "$(dirname "$0")" && pwd)
for f in conker-server server.ts data.ts assets/pack.json; do
  [ -e "$here/$f" ] || { echo "missing $here/$f" >&2; exit 1; }
done

command -v nginx > /dev/null || apt-get install -y nginx
command -v certbot > /dev/null || apt-get install -y certbot
if [ ! -x /usr/local/bin/bun ]; then
  command -v unzip > /dev/null || apt-get install -y unzip
  curl -fsSL https://bun.sh/install | BUN_INSTALL=/usr/local/bun bash
  ln -sf /usr/local/bun/bin/bun /usr/local/bin/bun
fi

id conker > /dev/null 2>&1 || useradd --system --home /opt/conker --shell /usr/sbin/nologin conker
install -d -o root -g conker -m 0750 /opt/conker /etc/conker
install -d -o root -g conker -m 0755 /opt/conker/src/web/app/src /srv/conker
[ -s /etc/conker/token.key ] || openssl rand -hex 32 | install -o root -g conker -m 0640 /dev/stdin /etc/conker/token.key

# The game: the binary, the web server (server.ts imports src/data.ts), the pack.
install -m 0755 "$here/conker-server" /opt/conker/conker-server
install -m 0644 "$here/server.ts" /opt/conker/src/web/app/server.ts
install -m 0644 "$here/data.ts" /opt/conker/src/web/app/src/data.ts
rm -rf /srv/conker/assets.new
cp -R "$here/assets" /srv/conker/assets.new
chmod -R a+rX /srv/conker/assets.new
rm -rf /srv/conker/assets.old
[ ! -d /srv/conker/assets ] || mv /srv/conker/assets /srv/conker/assets.old
mv /srv/conker/assets.new /srv/conker/assets
rm -rf /srv/conker/assets.old

# The certificate: certbot answers on port 80 through nginx (webroot), then the hook copies it where
# the game servers (group conker) read it and reloads nginx.
cat > /etc/conker/renew-hook.sh << 'EOF'
#!/bin/sh
# certbot --deploy-hook: the game servers look at /etc/conker/tls every 10 s (matches go on).
set -e
install -d -m 0750 -o root -g conker /etc/conker/tls
install -m 0640 -o root -g conker "$RENEWED_LINEAGE/fullchain.pem" "$RENEWED_LINEAGE/privkey.pem" /etc/conker/tls/
systemctl reload nginx
systemctl start conker-web
EOF
chmod 0755 /etc/conker/renew-hook.sh
install -d /var/www/letsencrypt

site=/etc/nginx/sites-available/conker
http_part() {
  cat << EOF
# conker-web's /api/ for the GitHub Pages site $PAGES (deploy/pages-vps.sh).
limit_req_zone \$binary_remote_addr zone=conker_api:1m rate=60r/m;

server {
    listen 80;
    listen [::]:80;
    server_name $DOMAIN;
    location /.well-known/acme-challenge/ { root /var/www/letsencrypt; }
    location / { return 301 https://\$host\$request_uri; }
}
EOF
}
if [ ! -s "/etc/letsencrypt/live/$DOMAIN/fullchain.pem" ]; then
  http_part > "$site"
  ln -sf "$site" /etc/nginx/sites-enabled/conker
  nginx -t && systemctl reload nginx
  certbot certonly --webroot -w /var/www/letsencrypt -d "$DOMAIN" -m "$EMAIL" --agree-tos -n \
    --deploy-hook /etc/conker/renew-hook.sh
fi
RENEWED_LINEAGE=/etc/letsencrypt/live/$DOMAIN /etc/conker/renew-hook.sh 2> /dev/null || true

{
  http_part
  cat << EOF

server {
    listen 443 ssl;
    listen [::]:443 ssl;
    http2 on;
    server_name $DOMAIN;
    ssl_certificate /etc/letsencrypt/live/$DOMAIN/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/$DOMAIN/privkey.pem;

    location /api/ {
        limit_req zone=conker_api burst=30 nodelay;
        limit_req_status 429;
        proxy_pass http://127.0.0.1:$API_PORT;
        proxy_set_header Host \$host;
        # The client's own address (never one it sends): the game servers' token rate limit.
        proxy_set_header X-Forwarded-For \$remote_addr;
        proxy_read_timeout 60s;
    }
    location / { return 302 $PAGES; }
}
EOF
} > "$site"
ln -sf "$site" /etc/nginx/sites-enabled/conker
nginx -t && systemctl reload nginx

cat > /etc/systemd/system/conker-web.service << EOF
# deploy/pages-vps.sh: the web server for the GitHub Pages site $PAGES and its game servers.
[Unit]
Description=Conker web server and its game servers (GitHub Pages)
After=network-online.target
Wants=network-online.target

[Service]
User=conker
Group=conker
ExecStart=/usr/local/bin/bun /opt/conker/src/web/app/server.ts --host 127.0.0.1 --port $API_PORT \\
    --game-ports $FIRST_PORT-$LAST_PORT --server /opt/conker/conker-server --assets /srv/conker/assets \\
    --cert /etc/conker/tls/fullchain.pem --cert-key /etc/conker/tls/privkey.pem \\
    --token-key /etc/conker/token.key --public $PUBLIC --cors $ORIGIN
Environment=RUST_LOG=info HOME=/tmp
Restart=always
RestartSec=2
# A guest on this host: its matches (children) share this memory and yield the CPU first.
MemoryMax=450M
CPUWeight=50
Nice=5
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
ProtectKernelTunables=yes
ProtectControlGroups=yes
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
LockPersonality=yes

[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl enable conker-web
systemctl restart conker-web
sleep 2
systemctl --no-pager status conker-web | head -12
echo "game ports $FIRST_PORT-$LAST_PORT (TCP + UDP) must be open to the internet"
