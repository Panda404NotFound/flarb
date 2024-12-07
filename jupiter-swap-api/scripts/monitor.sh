#!/bin/bash

while true; do
    echo "[$(date)] Запускаем jupiter-swap-api..."
    
    # Запускаем процесс и перенаправляем его вывод в файл
    ./jupiter-swap-api \
        --rpc-url "$RPC_URL" \
        --market-mode file \
        --market-cache /app/cache/markets-v4.json \
        --host 0.0.0.0 \
        --port 8080 \
        --total-thread-count "$TOTAL_THREAD_COUNT" \
        --webserver-thread-count "$WEBSERVER_THREAD_COUNT" \
        --update-thread-count "$UPDATE_THREAD_COUNT" \
        --disable-swap-cache-loading \
        --allow-circular-arbitrage 2>&1 | tee /tmp/jupiter.log
    
    # Проверяем, была ли паника
    if grep -q "thread 'main' panicked" /tmp/jupiter.log; then
        echo "[$(date)] Обнаружена паника, ожидаем 10 секунд перед перезапуском..."
        sleep 10
    else
        echo "[$(date)] Процесс завершился без паники"
        exit 0
    fi
done 