#!/usr/bin/env bash
set -e
OUT=target/dx/telcoin-explorer/release/web/public
dx build --release
rm -rf .vercel/output
mkdir -p .vercel/output/static
cp -r "$OUT"/. .vercel/output/static/
echo '{"version":3,"routes":[{"handle":"filesystem"},{"src":"/(.*)","dest":"/index.html"}]}' > .vercel/output/config.json
vercel deploy --prebuilt --prod
