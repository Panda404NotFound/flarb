// src/params.rs

use solana_program::pubkey::Pubkey;
use std::str::FromStr;
use std::collections::HashMap;
use lazy_static::lazy_static;

// Глобальная структура с адресами токенов
lazy_static! {
    pub static ref TOKEN_ADDRESSES: HashMap<&'static str, Pubkey> = {
        let mut map = HashMap::new();
        map.insert("SOL", Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap());
        map.insert("USDC", Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap());
        map
    };
}

// Параметры для тестирования
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Params {
    pub slippage_bps: u64,
    pub amount_in: u64,
    pub amount_out: u64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            slippage_bps: 50,  // 0.5%
            amount_in: 100_000_000_000,  // 1 SOL (9 decimals)
            amount_out: 1_000_000,       // 1 USDC (6 decimals)
        }
    }
}


/* 

QuoteConfig параметры:

1. `slippage_bps: Option<u64>`
- Допустимое проскальзывание в базисных пунктах (bps)
- 1 bps = 0.01%
- Опциональный параметр
- Пример: 100 = 1% проскальзывание
- Используется для защиты от неблагоприятного изменения цены

2. `swap_mode: Option<SwapMode>`
- Определяет режим свапа
- Два возможных значения (из enum SwapMode):
  * ExactIn - точное количество входящих токенов
  * ExactOut - точное количество исходящих токенов
- Опциональный параметр, по умолчанию ExactIn

3. `dexes: Option<Vec<String>>`
- Список DEX'ов для использования при свапе
- Опциональный параметр
- Если указан, свап будет искать маршруты только через указанные DEX'ы
- Позволяет ограничить набор используемых бирж

4. `exclude_dexes: Option<Vec<String>>`
- Список DEX'ов для исключения
- Опциональный параметр
- Указанные DEX'ы не будут использоваться при поиске маршрута
- Полезно для исключения ненадежных или медленных DEX'ов

5. `only_direct_routes: bool`
- Булево значение для ограничения только прямых свапов
- Если true - использует только прямые свапы (A → B)
- Если false - может использовать сложные маршруты (A → C → B)
- Не опциональный параметр, должен быть указан

6. `as_legacy_transaction: Option<bool>`
- Флаг для использования legacy транзакций
- Опциональный параметр
- Если true - использует старый формат транзакций
- Полезно для совместимости со старыми кошельками

7. `platform_fee_bps: Option<u64>`
- Комиссия платформы в базисных пунктах
- Опциональный параметр
- Позволяет установить дополнительную комиссию
- Используется для интеграторов

8. `max_accounts: Option<u64>`
- Максимальное количество аккаунтов в транзакции
- Опциональный параметр
- Помогает контролировать размер транзакции
- Важен для соблюдения лимитов Solana

*/