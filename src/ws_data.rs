// src/ws_data.rs
use serde::Deserialize;
use serde_json::Value;
use flume::{Sender, Receiver};

// Тип ответа для Processed (без поддержки слотов)
pub type ProcessedResponse = WebSocketResponse<NotificationParams<NotificationResultProcessed>>;
// Тип ответа для Orca DEX (с поддержкой слотов)
pub type OrcaFinalizedResponse = OrcaWebSocketResponse<NotificationParams<NotificationResultFinalized>>;


//////////////////////// КАНАЛЫ ДЛЯ АСИНХРОННОГО ПОТОКА ////////////////////////



// Каналы WebSocket для Orca DEX (с поддержкой слотов)
pub struct OrcaWebSocketChannels {
    pub program_tx: Sender<OrcaFinalizedResponse>,
    pub slot_tx: Sender<SlotInfo>,
}

// Реализация для OrcaWebSocketChannels
pub struct OrcaWebSocketReceivers {
    pub program_rx: Receiver<OrcaFinalizedResponse>,
    pub slot_rx: Receiver<SlotInfo>,
}

// Реализация для OrcaWebSocketChannels
impl OrcaWebSocketChannels {
    pub fn new() -> (Self, OrcaWebSocketReceivers) {
        let (program_tx, program_rx) = flume::unbounded();
        let (slot_tx, slot_rx) = flume::unbounded();
        
        (
            Self { program_tx, slot_tx },
            OrcaWebSocketReceivers { program_rx, slot_rx }
        )
    }
}

// Общие каналы WebSocket для всех DEX
pub struct WebSocketChannels {
    pub program_tx: Sender<ProcessedResponse>,
}

// Реализация для WebSocketChannels
pub struct WebSocketReceivers {
    pub program_rx: Receiver<ProcessedResponse>,
}

// Реализация для WebSocketChannels
impl WebSocketChannels {
    pub fn new() -> (Self, WebSocketReceivers) {
        let (program_tx, program_rx) = flume::unbounded();
        (
            Self { program_tx },
            WebSocketReceivers { program_rx }
        )
    }
}

// Трейт для обработки WebSocket каналов
pub trait WebSocketHandler<T: HasNotificationFields> {
    fn get_program_tx(&self) -> Option<&Sender<T>>;
    fn get_slot_tx(&self) -> Option<&Sender<SlotInfo>>;
}

// Трейт для работы с уведомлениями
#[allow(dead_code)]
pub trait HasNotificationFields {
    fn get_slot(&self) -> Option<SlotInfo>;
    fn get_params(&self) -> Option<NotificationParams<NotificationResult>>;
    fn get_notification(&self) -> Option<(u64, Value)>;
}

//////////////////////// КАНАЛЫ ДЛЯ АСИНХРОННОГО ПОТОКА ////////////////////////



// Перечисление для типов DEX
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum DexType {
    Orca,
    Raydium,
    Meteora,
}

// Базовая структура ответа WebSocket
#[derive(Debug, Deserialize, Clone)]
pub struct WebSocketResponse<P> {
    pub method: Option<String>,
    pub params: Option<P>,
    pub result: Option<u64>,
    pub id: Option<u64>,
}

// Структура с поддержкой слотов (специфична для Orca DEX)
#[derive(Debug, Deserialize, Clone)]
pub struct OrcaWebSocketResponse<P> {
    #[serde(flatten)]
    pub base: WebSocketResponse<P>,
    // Слоты доступны только для Orca DEX
    pub slot: Option<SlotInfo>,
}

// Общая структура для параметров уведомления
#[derive(Debug, Deserialize, Clone)]
pub struct NotificationParams<T> {
    pub result: T,
    pub subscription: u64,
}

// Типы результатов уведомлений
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum NotificationResult {
    Finalized(NotificationResultFinalized),
    Processed(NotificationResultProcessed),
}

// Результат уведомления для Finalized (поддерживает слоты, специфично для Orca)
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum NotificationResultFinalized {
    Program {
        context: Context,
        value: Value,
    },
    // Слоты доступны только в Orca DEX
    Slot {
        slot: u64,
        parent: u64,
        root: u64,
    },
}

// Результат уведомления для Processed (без поддержки слотов)
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum NotificationResultProcessed {
    Program {
        context: Context,
        value: Value,
    },
}

// Общие структуры данных
#[derive(Debug, Deserialize, Clone)]
pub struct Context {
    pub slot: u64,
}

// Структура ответа от программы подписки
#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct DataNotification {
    pub data: (String, String),
    pub executable: bool,
    pub lamports: u64,
    pub owner: String,
    #[serde(rename = "rentEpoch")]
    pub rent_epoch: u64,
    pub space: Option<u64>,
}

// Структура для адреса пула и данных от программы
#[derive(Debug, Deserialize, Clone)]
pub struct ProgramNotification {
    pub pubkey: String,
    pub account: DataNotification,
}

// Структура для слотов (специфична для Orca DEX)
#[derive(Debug, Deserialize, Clone)]
pub struct SlotInfo {
    pub slot: u64,
    pub parent: u64,
    pub root: u64,
}

// Реализация WebSocketHandler для OrcaWebSocketChannels
impl WebSocketHandler<OrcaFinalizedResponse> for OrcaWebSocketChannels {
    fn get_program_tx(&self) -> Option<&Sender<OrcaFinalizedResponse>> {
        Some(&self.program_tx)
    }

    fn get_slot_tx(&self) -> Option<&Sender<SlotInfo>> {
        Some(&self.slot_tx)
    }
}

// Реализация WebSocketHandler для WebSocketChannels
impl WebSocketHandler<ProcessedResponse> for WebSocketChannels {
    fn get_program_tx(&self) -> Option<&Sender<ProcessedResponse>> {
        Some(&self.program_tx)
    }

    fn get_slot_tx(&self) -> Option<&Sender<SlotInfo>> {
        None
    }
}

// Реализация HasNotificationFields для OrcaFinalizedResponse
impl HasNotificationFields for OrcaFinalizedResponse {
    fn get_slot(&self) -> Option<SlotInfo> {
        self.slot.clone()
    }

    fn get_params(&self) -> Option<NotificationParams<NotificationResult>> {
        self.base.params.clone().map(|p| NotificationParams {
            result: NotificationResult::Finalized(p.result),
            subscription: p.subscription,
        })
    }

    fn get_notification(&self) -> Option<(u64, Value)> {
        self.base.params.as_ref().and_then(|p| {
            match &p.result {
                NotificationResultFinalized::Program { context, value } => {
                    Some((context.slot, value.clone()))
                }
                _ => None
            }
        })
    }
}

// Реализация HasNotificationFields для ProcessedResponse 
impl HasNotificationFields for ProcessedResponse {
    fn get_slot(&self) -> Option<SlotInfo> {
        None
    }

    fn get_params(&self) -> Option<NotificationParams<NotificationResult>> {
        self.params.clone().map(|p| NotificationParams {
            result: NotificationResult::Processed(p.result),
            subscription: p.subscription,
        })
    }

    fn get_notification(&self) -> Option<(u64, Value)> {
        self.params.as_ref().and_then(|p| {
            match &p.result {
                NotificationResultProcessed::Program { context, value } => {
                    Some((context.slot, value.clone()))
                }
            }
        })
    }
}

// Трейт для проверки полей подписки
pub trait HasSubscriptionFields {
    fn has_valid_subscription(&self) -> bool;
}

// Реализация для разных типов ответов
impl HasSubscriptionFields for ProcessedResponse {
    fn has_valid_subscription(&self) -> bool {
        self.result.is_some() && self.id.is_some() && self.method.is_none()
    }
}

// Обработка обновления аккаунтов 
impl HasSubscriptionFields for OrcaFinalizedResponse {
    fn has_valid_subscription(&self) -> bool {
        self.base.result.is_some() && self.base.id.is_some() && self.base.method.is_none()
    }
}