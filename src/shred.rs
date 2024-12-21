use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
#[allow(unused_imports)]
use log::{info, warn, error, debug};
use tokio::net::UdpSocket as TokioUdpSocket;
use solana_ledger::shred::Shred;
use anyhow::Result;
use crate::config::CONFIG;

#[derive(Default)]
struct ShredMetrics {
    received_packets: AtomicU64,
    processed_shreds: AtomicU64,
    invalid_packets: AtomicU64,
}

pub struct ShredReceiver {
    metrics: Arc<ShredMetrics>,
    bind_addr: SocketAddr,
}

impl ShredReceiver {
    pub fn new(port: u16) -> Result<Self> {
        let bind_addr = format!("0.0.0.0:{}", port).parse()?;
        
        Ok(Self {
            metrics: Arc::new(ShredMetrics::default()),
            bind_addr,
        })
    }

    pub async fn start(&self) -> Result<()> {
        info!("Инициализация получателя шредов на {}", self.bind_addr);

        // Создаем UDP сокет для приема данных от Jito proxy
        let socket = TokioUdpSocket::bind(self.bind_addr).await?;
        info!("UDP сокет привязан к {}", socket.local_addr()?);

        let exit = Arc::new(AtomicBool::new(false));
        let exit_clone = exit.clone();
        let metrics = self.metrics.clone();

        // Запускаем обработку входящих пакетов
        tokio::spawn(async move {
            let mut buf = [0u8; 1024 * 64];
            let mut last_log = std::time::Instant::now();
            
            info!("Начало получения пакетов от Jito ShredStream...");

            while !exit.load(Ordering::Relaxed) {
                debug!("Ожидание пакета...");
                match socket.recv_from(&mut buf).await {
                    Ok((n, addr)) => {
                        info!("Получены пакеты {}", n);
                        metrics.received_packets.fetch_add(1, Ordering::Relaxed);
                        
                        debug!(
                            "Получен пакет: размер={}, от={}, первые_байты={:?}",
                            n,
                            addr,
                            &buf[..std::cmp::min(n, 16)]
                        );

                        match Shred::new_from_serialized_shred(buf[..n].to_vec()) {
                            Ok(shred) => {
                                metrics.processed_shreds.fetch_add(1, Ordering::Relaxed);
                                info!(
                                    "Обработан шред: slot={}, index={}, type={:?}", 
                                    shred.slot(),
                                    shred.index(),
                                    if shred.is_data() { "Data" } else { "Code" }
                                );
                            }
                            Err(e) => {
                                metrics.invalid_packets.fetch_add(1, Ordering::Relaxed);
                                warn!("Ошибка десериализации: {}, размер={}", e, n);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Ошибка получения данных: {}", e);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }

                if last_log.elapsed() > Duration::from_secs(5) {
                    info!(
                        "Статистика: получено={}, обработано={}, ошибок={}", 
                        metrics.received_packets.load(Ordering::Relaxed),
                        metrics.processed_shreds.load(Ordering::Relaxed),
                        metrics.invalid_packets.load(Ordering::Relaxed)
                    );
                    last_log = std::time::Instant::now();
                }
            }
        });

        // Обработка Ctrl+C
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            info!("Получен сигнал завершения");
            exit_clone.store(true, Ordering::Relaxed);
        });

        Ok(())
    }
}

// Запуск получения шредов на порту jito_udp_port (8001)
pub async fn start_shred_stream() -> Result<()> {
    let port = CONFIG.jito_udp_port.parse::<u16>()?;
    let receiver = ShredReceiver::new(port)?;
    receiver.start().await
}

// Тестирование UDP соединения и отправки тестовых пакетов на порт Jito proxy
#[allow(dead_code)]
pub async fn test_udp_connection() -> Result<()> {
    let test_socket = TokioUdpSocket::bind("0.0.0.0:0").await?;
    let test_data = vec![0u8; 1232];
    
    info!("Отправка тестовых UDP пакетов на порт 20000 (Jito proxy)");
    
    for _ in 0..5 {
        test_socket.send_to(&test_data, "0.0.0.0:20000").await?;
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    
    Ok(())
}