ИНСТРУКЦИЯ:

1. Скачайте образ и markets (проверь официальный эндпоинт маркета)
docker pull ghcr.io/jup-ag/jupiter-swap-api:v6.0.34

mkdir -p ./jupiter-swap-api/cache/

curl -o ./jupiter-swap-api/cache/markets-v4.json https://cache.jup.ag/markets?v=4

2. Запустите контейнер
docker run -d \
  --restart unless-stopped \
  --name jupiter-swap-api \
  --cpus=4 \
  --memory=4g \
  --memory-swap=4g \
  --ipc=host \
  -p 8080:8080 \
  -e RUST_LOG=info \
  # -v $(pwd)/jupiter-swap-api/cache:/app/cache \
  -v $(pwd)/jupiter-swap-api/scripts:/app/scripts \
  # -e MARKET_MODE=file \
  # -e MARKET_CACHE=/app/cache/markets-v4.json \
  -e RPC_URL="https://mainnet.helius-rpc.com/" \
  -e SECONDARY_RPC_URLS="https://quote-api.jup.ag/v6" \
  -e HOST=0.0.0.0 \
  -e PORT=8080 \
  -e TOTAL_THREAD_COUNT=4 \
  -e WEBSERVER_THREAD_COUNT=4 \
  -e UPDATE_THREAD_COUNT=4 \
  -e DISABLE_SWAP_CACHE_LOADING=true \
  -e ALLOW_CIRCULAR_ARBITRAGE=true \
  ghcr.io/jup-ag/jupiter-swap-api:v6.0.34 \
  /bin/bash -c "chmod +x /app/scripts/monitor.sh && /app/scripts/monitor.sh"

3. Дождитесь полной инициализации (примерно 2-5 минут)

4. Проверьте, что контейнер работает
curl http://localhost:8080/health


# ./jupiter-swap-api --help
Usage: jupiter-swap-api [OPTIONS] --rpc-url <RPC_URL>

Options:
      --market-cache <MARKET_CACHE>
          Jupiter europa URL, file path or remote file path, check production Jupiter cache for format https://cache.jup.ag/markets?v=4. Will default to the associated market mode default when not specified Note: the params field is required for some AMMs and is AMM type specific [env: MARKET_CACHE=]
      --market-mode <MARKET_MODE>
          Switch between market modes, file and remote will not receive new markets from Europa [env: MARKET_MODE=] [default: europa] [possible values: europa, remote, file]
      --rpc-url <RPC_URL>
          RPC URL for polling and fetching user accounts [env: RPC_URL=https://mainnet.helius-rpc.com/]
      --secondary-rpc-urls <SECONDARY_RPC_URLS>...
          Secondary RPC URLs used for some RPC calls [env: SECONDARY_RPC_URLS=]
  -e, --yellowstone-grpc-endpoint <YELLOWSTONE_GRPC_ENDPOINT>
          Yellowstone gRPC endpoint e.g. https://jupiter.rpcpool.com [env: YELLOWSTONE_GRPC_ENDPOINT=]
  -x, --yellowstone-grpc-x-token <YELLOWSTONE_GRPC_X_TOKEN>
          Yellowstone gRPC x token, the token after the hostname [env: YELLOWSTONE_GRPC_X_TOKEN=]
      --yellowstone-grpc-enable-ping
          Enable pinging the grpc server, useful for a load balanced Yellowstone GRPC endpoint https://github.com/rpcpool/yellowstone-grpc/issues/225 [env: YELLOWSTONE_GRPC_ENABLE_PING=]
      --snapshot-poll-interval-ms <SNAPSHOT_POLL_INTERVAL_MS>
          Interval after which AMMs related account should be fetched, in yellowstone grpc mode, there will be a periodic poll to snapshot the confirmed state of AMM accounts Default to 200 ms for poll mode and 30000 ms for yellowstone grpc mode [env: SNAPSHOT_POLL_INTERVAL_MS=]
      --enable-external-amm-loading
          Enable loading external AMMs from keyedUiAccounts in swap related endpoints [env: ENABLE_EXTERNAL_AMM_LOADING=]
      --disable-swap-cache-loading
          Disable loading caches necessary for swap related features to function properly, such as address lookup tables... This is useful for quote only APIs [env: DISABLE_SWAP_CACHE_LOADING=]
      --allow-circular-arbitrage
          Allow arbitrage quote and swap, where input mint is equal to output mint [env: ALLOW_CIRCULAR_ARBITRAGE=]
      --sentry-dsn <SENTRY_DSN>
          Sentry DSN to send error to [env: SENTRY_DSN=]
      --dex-program-ids <DEX_PROGRAM_IDS>...
          List of DEX program ids to include, other program ids won't be loaded, you can get program ids from https://quote-api.jup.ag/v6/program-id-to-label [env: DEX_PROGRAM_IDS=]
      --exclude-dex-program-ids <EXCLUDE_DEX_PROGRAM_IDS>...
          List of DEX program ids to exclude, from all program ids, excluded program ids won't be loaded, you can get program ids from https://quote-api.jup.ag/v6/program-id-to-label [env: EXCLUDE_DEX_PROGRAM_IDS=]
      --filter-markets-with-mints <FILTER_MARKETS_WITH_MINTS>...
          List of mints to filter markets to include, markets which do not have at least 2 mints from this set will be excluded [env: FILTER_MARKETS_WITH_MINTS=]
  -H, --host <HOST>
          The host [env: HOST=0.0.0.0] [default: 0.0.0.0]
  -p, --port <PORT>
          A port number on which to start the application [env: PORT=8080] [default: 8080]
      --metrics-port <METRICS_PORT>
          Port for Prometheus metrics endpoint `/metrics` [env: METRICS_PORT=]
  -s, --expose-quote-and-simulate
          Enable the /quote-and-simulate endpoint to quote and simulate a swap in a single request [env: EXPOSE_QUOTE_AND_SIMULATE=]
      --enable-deprecated-indexed-route-maps
          Enable computating and serving the /indexed-route-map Deprecated and not recommended to be enabled due to the high overhead [env: ENABLE_DEPRECATED_INDEXED_ROUTE_MAPS=]
      --enable-new-dexes
          Enable new dexes that have been recently integrated, new dexes: [] [env: ENABLE_NEW_DEXES=]
      --enable-diagnostic
          Enable the /diagnostic endpoint to quote [env: ENABLE_DIAGNOSTIC=]
      --enable-add-market
          Enable the /add-market endpoint to hot load a new market [env: ENABLE_ADD_MARKET=]
      --total-thread-count <TOTAL_THREAD_COUNT>
          Total count of thread to use for the jupiter-swap-api process [env: TOTAL_THREAD_COUNT=] [default: 2]
      --webserver-thread-count <WEBSERVER_THREAD_COUNT>
          Count of thread [env: WEBSERVER_THREAD_COUNT=] [default: 2]
      --update-thread-count <UPDATE_THREAD_COUNT>
          [env: UPDATE_THREAD_COUNT=] [default: 4]
      --loki-url <LOKI_URL>
          Loki url [env: LOKI_URL=]
      --loki-username <LOKI_USERNAME>
          Loki username [env: LOKI_USERNAME=]
      --loki-password <LOKI_PASSWORD>
          Loki password [env: LOKI_PASSWORD=]
      --loki-custom-labels <LOKI_CUSTOM_LABELS>...
          Custom labels to add to the loki metrics e.g. `APP_NAME=jupiter-swap-api,ENVIRONMENT=production` [env: LOKI_CUSTOM_LABELS=]
  -h, --help
          Print help
  -V, --version
          Print version