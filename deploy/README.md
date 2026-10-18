# Deploy (single VPS)

```bash
cp deploy/auction.env.example deploy/auction.env   # set ADMIN_TOKEN, CORS_ORIGIN
cp deploy/web.env.example deploy/web.env           # set the site URL
cd deploy && SITE_HOST=your.site docker compose up -d --build
```

- The operator seed is generated on first start inside the `auction-data` volume. Back it up:
  `docker compose cp auction:/data/operator.seed ./operator.seed.backup` and store it offline.
- `GET https://your.site/bb/health` shows sync lag and the low-balance flag; with `x-admin-token` it also shows
  the balance, pending payouts, mempool activity and the last error.
- Admin endpoints are blocked at Caddy. Reach them from the host: `ssh -L 8787:127.0.0.1:8787 your.site`, then
  `curl http://127.0.0.1:8787/admin/... -H 'x-admin-token: …'`.
- Set `BIRTHDAY_HEIGHT` in `auction.env` to the chain tip at first start and keep it with the seed backup; a restore
  without it rescans only the last 100 blocks.
- Build from a clean clone (the `.dockerignore` keeps `target/`, `data/` and env files out of the context).
- Fund the operator address shown there before the first auction closes so refunds can be sent.
- Create a seller key: `curl -X POST https://your.site/bb/admin/sellers -H 'x-admin-token: …' -d '{"name":"Alice"}'`.
  Sellers list with `x-seller-key` on `POST /auctions`.
- Back up `/data/bb-main.sqlite` (auctions, bids, payouts, events, Go Dark envelopes) and
  `/data/wallet-main/data.sqlite` after every lot event with `sqlite3 file ".backup out"` (WAL mode makes a plain
  copy unsafe).
