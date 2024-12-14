Для токенов:

Endpoint: https://tokens.jup.ag/tokens?tags=verified,community

Для Raydium CLMM:

Endpoint: https://api.raydium.io/v2/ammV3/ammPools

Для Orca whirlpool:

Endpoint: https://api.mainnet.orca.so/v1/whirlpool/list

Для Meteora Dlmm:

Endpoint: https://dlmm-api.meteora.ag/pair/all


Команды для скачивания данных:

curl https://dlmm-api.meteora.ag/pair/all | jq '.' > pools/meteora_pools.json

curl https://api.mainnet.orca.so/v1/whirlpool/list | jq '.' > pools/orca_pools.json

curl https://api.raydium.io/v2/ammV3/ammPools | jq '.' > pools/raydium_pools.json

curl https://tokens.jup.ag/tokens?tags=verified,community | jq '.' > pools/tokens.json