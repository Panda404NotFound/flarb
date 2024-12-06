ИНСТРУКЦИЯ:

1. Скачайте образ
docker pull ghcr.io/jup-ag/jupiter-swap-api:v6.0.34

2. Создайте директорию для кэша
mkdir -p ./cache

3. Просто скачиваем файл без модификаций
curl -s "https://cache.jup.ag/markets?v=3" > ./cache/markets.json

4. Запустите контейнер
docker run -d \
  --restart unless-stopped \
  --name jupiter-swap-api \
  --cpus=8 \
  --memory=16g \ 
  --memory-swap=32g \
  -p 8080:8080 \
  -e RUST_LOG=info \
  -e RPC_URL="https://mainnet.helius-rpc.com/" \
  -e HOST=0.0.0.0 \
  -e PORT=8080 \
  -e MARKET_MODE=file \
  -e MARKET_CACHE=/app/cache/markets.json \
  -e RUST_BACKTRACE=1 \
  -e REQUEST_TIMEOUT=180 \
  -e LOOKUP_TABLE_CACHE_DURATION=600 \
  -e MAX_CONCURRENT_ALTS=400 \
  -e CACHE_UPDATE_INTERVAL=120 \
  -e CACHE_WARMUP=true \
  -e QUOTE_CACHE_TTL=30 \
  -e MAX_POOL_SIZE=200 \ 
  -e MAX_RETRIES=3 \
  -e ENABLE_QUOTES_ONLY=true \
  -e ENABLE_SKIP_PREFLIGHT=true \
  -e ALT_BATCH_SIZE=200 \ 
  -e ALT_REQUEST_INTERVAL=500 \ 
  -e CONCURRENT_REQUESTS=50 \ 
  -e BATCH_REQUEST_SIZE=25 \
  -e CONNECTION_TIMEOUT=60000 \ 
  -e KEEP_ALIVE_TIMEOUT=30000 \
  -e HTTP_KEEP_ALIVE=true \
  -e MAX_CONNECTIONS=1000 \
  -e RATE_LIMIT_BURST=500 \
  -v $(pwd)/cache:/app/cache \
  ghcr.io/jup-ag/jupiter-swap-api:v6.0.34

5. Проверьте, что контейнер работает
curl http://localhost:8080/health