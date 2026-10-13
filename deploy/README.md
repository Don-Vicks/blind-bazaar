# Deploy (single VPS)

```bash
cp deploy/auction.env.example deploy/auction.env   # set ADMIN_TOKEN, CORS_ORIGIN
cp deploy/web.env.example deploy/web.env           # set the site URL
cd deploy && SITE_HOST=your.site docker compose up -d --build
```

- The operator seed is generated on first start inside the `auction-data` volume. Back it up:
  `docker compose cp auction:/data/operator.seed ./operator.seed.backup` and store it offline.
- `GET https://your.site/bb/health` shows sync lag, balance, pending payouts and the low-balance flag.
- Fund the operator address shown there before the first auction closes so refunds can be sent.
- Create a seller key: `curl -X POST https://your.site/bb/admin/sellers -H 'x-admin-token: …' -d '{"name":"Alice"}'`.
  Sellers list with `x-seller-key` on `POST /auctions`.
- Back up `/data/bb-main.sqlite` (auctions, bids, payouts, events) and `/data/wallet-main/data.sqlite` daily.
